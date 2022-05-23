use cosmwasm_std::CosmosMsg;
use provwasm_std::ProvenanceMsg;

use crate::core::error::ContractError;
use crate::core::types::verifier_detail::VerifierDetailV2;

use super::{aliases::AssetResult, functions::bank_send, traits::ResultExtensions};

/// This function distributes funds from the sender address to the targets defined by a [VerifierDetailV2](crate::core::types::verifier_detail::VerifierDetailV2).
/// It breaks down all percentages defined in the verifier detail's fee destinations and core onboarding
/// cost to derive a variable sized vector of destination messages.
/// Important: The response type is of [ProvenanceMsg](provwasm_std::ProvenanceMsg), which allows
/// these bank send messages to match the type used for contract execution routes.
///
/// # Parameters
///
/// * `verifier` The verifier detail from which to extract fee information.
pub fn calculate_verifier_cost_messages(
    verifier: &VerifierDetailV2,
) -> AssetResult<Vec<CosmosMsg<ProvenanceMsg>>> {
    let mut cost_messages: Vec<CosmosMsg<ProvenanceMsg>> = vec![];
    let denom = &verifier.onboarding_denom;
    // The onboarding_cost is a u128, so attempting to subtract the fee total from it when the fee total is a greater value
    // will result in an unhandled panic.  Detect this potentiality for a misconfigured verifier and exit early to prevent
    // future head-scratching (panics are very difficult to debug due to redacted responses)
    if verifier.fee_amount > verifier.onboarding_cost {
        return ContractError::generic(
             format!("misconfigured verifier data! fee total ({}{}) was greater than the total cost of onboarding ({}{})",
             verifier.fee_amount,
             denom,
             verifier.onboarding_cost,
             denom,
        )).to_err();
    }
    // The total funds disbursed to the verifier itself is the remainder from subtracting the fee cost from the onboarding cost
    let verifier_cost = verifier.onboarding_cost - verifier.fee_amount;
    // Append a bank send message from the contract to the verifier for the cost if the verifier receives funds
    if !verifier_cost.is_zero() {
        cost_messages.push(bank_send(&verifier.address, verifier_cost.u128(), denom));
    }
    let mut fee_total: u128 = 0;
    // Append a message for each destination
    verifier.fee_destinations.iter().for_each(|destination| {
        cost_messages.push(bank_send(
            &destination.address,
            destination.fee_amount.u128(),
            denom,
        ));
        fee_total += destination.fee_amount.u128();
    });
    if fee_total != verifier.fee_amount.u128() {
        return ContractError::generic(
            format!("misconfigured fee destinations! fee total ({}{}) was not equal to the specified fee amount ({}{})",
                fee_total,
                denom,
                verifier.fee_amount.u128(),
                denom,
            )
        ).to_err();
    }
    cost_messages.to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::{BankMsg, CosmosMsg, Uint128};
    use provwasm_std::ProvenanceMsg;

    use crate::core::types::fee_destination::FeeDestinationV2;
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::{
        core::error::ContractError,
        testutil::test_utilities::get_default_entity_detail,
        util::{constants::NHASH, traits::OptionExtensions},
    };

    use super::calculate_verifier_cost_messages;

    #[test]
    fn test_invalid_verifier_greater_fee_than_onboarding_cost() {
        // This verifier tries to send 150% of the fee to the fee destination. NO BUENO!
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            NHASH,
            Uint128::new(101),
            vec![FeeDestinationV2::new("fee", Uint128::new(101))],
            get_default_entity_detail().to_some(),
        );
        let error = calculate_verifier_cost_messages(&verifier).unwrap_err();
        match error {
            ContractError::GenericError { msg } => {
                assert_eq!(
                    "misconfigured verifier data! fee total (101nhash) was greater than the total cost of onboarding (100nhash)",
                    msg.as_str(),
                    "unexpected error message generated",
                );
            }
            _ => panic!(
                "unexpected error encountered when providing a bad verifier: {:?}",
                error
            ),
        }
    }

    #[test]
    fn test_invalid_verifier_fee_destination_sum_mismatch() {
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            NHASH,
            // Take half of the onboarding cost as a fee, but have the destination request more than that
            Uint128::new(50),
            // All fee destinations should always add up to the verifier's fee_amount
            vec![FeeDestinationV2::new("fee", Uint128::new(51))],
            None,
        );
        let error = calculate_verifier_cost_messages(&verifier).unwrap_err();
        match error {
            ContractError::GenericError { msg } => {
                assert_eq!(
                    "misconfigured fee destinations! fee total (51nhash) was not equal to the specified fee amount (50nhash)",
                    msg.as_str(),
                    "unexpected error message generated",
                );
            }
            _ => panic!(
                "unexpected error encountered when providing a bad verifier: {:?}",
                error
            ),
        }
    }

    #[test]
    fn test_only_send_to_verifier() {
        let verifier = VerifierDetailV2::new(
            "verifier",
            Uint128::new(100),
            NHASH,
            Uint128::zero(),
            vec![],
            None,
        );
        let messages = calculate_verifier_cost_messages(&verifier)
            .expect("validation should pass and messages should be returned");
        assert_eq!(
            1,
            messages.len(),
            "expected only a single message to be sent"
        );
        test_messages_contains_send_for_address(
            &messages,
            "verifier",
            100,
            NHASH,
            "expected all funds to be sent to the verifier",
        );
    }

    #[test]
    fn test_only_send_to_single_fee_destination() {
        let verifier = VerifierDetailV2::new(
            "verifier",
            Uint128::new(100),
            NHASH,
            Uint128::new(100),
            vec![FeeDestinationV2::new("fee-destination", Uint128::new(100))],
            None,
        );
        let messages = calculate_verifier_cost_messages(&verifier)
            .expect("validation should pass and messages should be returned");
        assert_eq!(
            1,
            messages.len(),
            "expected only a single message to be sent",
        );
        test_messages_contains_send_for_address(
            &messages,
            "fee-destination",
            100,
            NHASH,
            "expected all funds to be sent to the fee destination",
        );
    }

    #[test]
    fn test_even_split_between_verifier_and_single_fee_destination() {
        let verifier = VerifierDetailV2::new(
            "verifier",
            Uint128::new(100),
            NHASH,
            Uint128::new(50),
            vec![FeeDestinationV2::new("fee-destination", Uint128::new(50))],
            None,
        );
        let messages = calculate_verifier_cost_messages(&verifier)
            .expect("validation should pass and messages should be returned");
        assert_eq!(2, messages.len(), "expected two messages to be sent",);
        test_messages_contains_send_for_address(
            &messages,
            "verifier",
            50,
            NHASH,
            "expected half of the funds to be sent to the verifier",
        );
        test_messages_contains_send_for_address(
            &messages,
            "fee-destination",
            50,
            NHASH,
            "expected half of the funds to be sent to the fee destination",
        );
    }

    #[test]
    fn test_many_fee_destinations_and_some_to_verifier() {
        let verifier = VerifierDetailV2::new(
            "verifier",
            Uint128::new(200),
            NHASH,
            Uint128::new(100),
            vec![
                FeeDestinationV2::new("first", Uint128::new(20)),
                FeeDestinationV2::new("second", Uint128::new(20)),
                FeeDestinationV2::new("third", Uint128::new(40)),
                FeeDestinationV2::new("fourth", Uint128::new(5)),
                FeeDestinationV2::new("fifth", Uint128::new(15)),
            ],
            None,
        );
        let messages = calculate_verifier_cost_messages(&verifier)
            .expect("validation should pass and messages should be returned");
        assert_eq!(6, messages.len(), "expected six messages to be sent");
        test_messages_contains_send_for_address(
            &messages,
            "verifier",
            100,
            NHASH,
            "expected half of all funds to be sent to the verifier",
        );
        test_messages_contains_send_for_address(
            &messages,
            "first",
            20,
            NHASH,
            "expected 20 nhash of the fee to be sent to the first fee destination",
        );
        test_messages_contains_send_for_address(
            &messages,
            "second",
            20,
            NHASH,
            "expected 20 nhash of the fee to be sent to the second fee destination",
        );
        test_messages_contains_send_for_address(
            &messages,
            "third",
            40,
            NHASH,
            "expected 40 nhash of the fee to be sent to the third fee destination",
        );
        test_messages_contains_send_for_address(
            &messages,
            "fourth",
            5,
            NHASH,
            "expected 5 nhash of the fee to be sent to the fourth fee destination",
        );
        test_messages_contains_send_for_address(
            &messages,
            "fifth",
            15,
            NHASH,
            "expected 15 nhash of the fee to be sent to the fifth fee destination",
        );
    }

    /// Loops through all messages contained in the input slice until it finds a message with the given address,
    /// ensuring that the expected amount was sent in the expected denom to that address.  All output errors are
    /// prefixed with the input error_message string.
    fn test_messages_contains_send_for_address<
        S: Into<String>,
        D: Into<String>,
        M: Into<String>,
    >(
        messages: &[CosmosMsg<ProvenanceMsg>],
        address: S,
        expected_amount: u128,
        expected_denom: D,
        error_message: M,
    ) {
        let target_address: String = address.into();
        let target_denom: String = expected_denom.into();
        let err_msg: String = error_message.into();
        messages
            .iter()
            .find(|msg| match msg {
                CosmosMsg::Bank(bank_msg) => match bank_msg {
                    BankMsg::Send { to_address, amount } => {
                        if to_address == &target_address {
                            assert_eq!(
                                1,
                                amount.len(),
                                "{}: only one coin should be appended per message",
                                err_msg,
                            );
                            let coin = amount.first().unwrap();
                            assert_eq!(
                                expected_amount,
                                coin.amount.u128(),
                                "{}: incorrect amount sent to address",
                                err_msg,
                            );
                            assert_eq!(
                                target_denom, coin.denom,
                                "{}: incorrect denom in bank send message",
                                err_msg,
                            );
                            // Return true - this is the correct address and has passed assertions
                            true
                        } else {
                            // Return false - this is a bank send message, but not to the expected address
                            false
                        }
                    }
                    _ => false,
                },
                _ => false,
            })
            .expect(
                format!(
                    "{}: could not find address {} in any send messages",
                    err_msg, target_address,
                )
                .as_str(),
            );
    }
}
