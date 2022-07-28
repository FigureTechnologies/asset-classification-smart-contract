use cosmwasm_std::{coin, Addr, CosmosMsg, Env};
use provwasm_std::{assess_custom_fee, ProvenanceMsg};

use crate::core::error::ContractError;
use crate::core::types::entity_detail::EntityDetail;
use crate::core::types::fee_destination::FeeDestinationV2;
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::util::traits::OptionExtensions;

use super::{aliases::AssetResult, traits::ResultExtensions};

/// This function distributes funds from the sender address to the targets defined by a [VerifierDetailV2](crate::core::types::verifier_detail::VerifierDetailV2).
/// It breaks down all percentages defined in the verifier detail's fee destinations and core onboarding
/// cost to derive a variable sized vector of destination messages as custom Provenance Blockchain
/// MsgFees.
/// Important: The response type is of [ProvenanceMsg](provwasm_std::ProvenanceMsg), which allows
/// these bank send messages to match the type used for contract execution routes.
///
/// # Parameters
///
/// * `verifier` The verifier detail from which to extract fee information.
/// * `env` The environment value provided when calling execute routes.  Used to guaranteed retrieval
/// of the correct contract address for custom msg fees.
pub fn calculate_verifier_cost_messages(
    env: &Env,
    verifier: &VerifierDetailV2,
) -> AssetResult<Vec<CosmosMsg<ProvenanceMsg>>> {
    let mut cost_messages: Vec<CosmosMsg<ProvenanceMsg>> = vec![];
    let denom = &verifier.onboarding_denom;
    let mut fee_total: u128 = 0;
    // Append a message for each destination
    for destination in verifier.fee_destinations.iter() {
        cost_messages.push(assess_custom_fee(
            coin(
                // Always multiply the fee amount by 2.  This is because the Provenance Fee Module
                // takes 50% of the value provided.  Doubling the value in this way will ensure that
                // all fee destinations get exactly the amount requested in the FeeDestination.
                destination.fee_amount.u128() * 2,
                denom,
            ),
            generate_destination_fee_name(destination),
            // The "from" field must always be the contract's address to ensure that message
            // execution failures do not occur
            env.contract.address.to_owned(),
            // All FeeDestination addresses are verified as valid bech32 addresses when they are
            // added to the contract, so this conversion is inherently fine to do
            Some(Addr::unchecked(&destination.address)),
        )?);
        fee_total += destination.fee_amount.u128();
    }
    // Fee distribution can, at most, be equal to the onboarding cost.  The onboarding cost should
    // always reflect the exact total that is taken from the requestor address when onboarding a new
    // scope.
    if fee_total > verifier.onboarding_cost.u128() {
        return ContractError::generic(
            format!("misconfigured fee destinations! fee total ({}{}) was greater than the specified onboarding cost ({}{})",
                fee_total,
                denom,
                verifier.onboarding_cost.u128(),
                denom,
            )
        ).to_err();
    }
    // The total funds disbursed to the verifier itself is the remainder from subtracting the fee cost from the onboarding cost
    let verifier_cost = verifier.onboarding_cost.u128() - fee_total;
    // Append a bank send message from the contract to the verifier for the cost if the verifier receives funds
    if verifier_cost > 0 {
        cost_messages.push(assess_custom_fee(
            coin(verifier_cost, denom),
            generate_verifier_fee_name(verifier),
            env.contract.address.to_owned(),
            Some(Addr::unchecked(&verifier.address)),
        )?);
    }
    cost_messages.to_ok()
}

fn generate_destination_fee_name(destination: &FeeDestinationV2) -> Option<String> {
    format!(
        "Fee for {}",
        generate_entity_detail_name(&destination.entity_detail)
            .unwrap_or_else(|| destination.address.to_owned()),
    )
    .to_some()
}

fn generate_verifier_fee_name(verifier: &VerifierDetailV2) -> Option<String> {
    generate_entity_detail_name(&verifier.entity_detail)
        .map(|detail_name| format!("{} Fee", detail_name))
        .unwrap_or_else(|| "Verifier Fee".to_string())
        .to_some()
}

fn generate_entity_detail_name(entity_detail: &Option<EntityDetail>) -> Option<String> {
    entity_detail.to_owned().and_then(|detail| detail.name)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_env;
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
            vec![FeeDestinationV2::new("fee", Uint128::new(101))],
            get_default_entity_detail().to_some(),
        );
        let error = calculate_verifier_cost_messages(&mock_env(), &verifier).unwrap_err();
        match error {
            ContractError::GenericError { msg } => {
                assert_eq!(
                    "misconfigured fee destinations! fee total (101nhash) was greater than the specified onboarding cost (100nhash)",
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
        let verifier = VerifierDetailV2::new("verifier", Uint128::new(100), NHASH, vec![], None);
        let messages = calculate_verifier_cost_messages(&mock_env(), &verifier)
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
            vec![FeeDestinationV2::new("fee-destination", Uint128::new(100))],
            None,
        );
        let messages = calculate_verifier_cost_messages(&mock_env(), &verifier)
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
            vec![FeeDestinationV2::new("fee-destination", Uint128::new(50))],
            None,
        );
        let messages = calculate_verifier_cost_messages(&mock_env(), &verifier)
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
            vec![
                FeeDestinationV2::new("first", Uint128::new(20)),
                FeeDestinationV2::new("second", Uint128::new(20)),
                FeeDestinationV2::new("third", Uint128::new(40)),
                FeeDestinationV2::new("fourth", Uint128::new(5)),
                FeeDestinationV2::new("fifth", Uint128::new(15)),
            ],
            None,
        );
        let messages = calculate_verifier_cost_messages(&mock_env(), &verifier)
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
