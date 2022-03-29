use std::ops::Mul;

use cosmwasm_std::{CosmosMsg, Uint128};
use provwasm_std::ProvenanceMsg;

use crate::core::{error::ContractError, types::verifier_detail::VerifierDetail};

use super::{aliases::AssetResult, functions::bank_send, traits::ResultExtensions};

/// This function distributes funds from the sender address to the targets defined by a VerifierDetail.
/// It breaks down all percentages defined in the verifier detail's fee destinations and core onboarding
/// cost to derive a variable sized vector of destination messages.
/// Important: The response type is of ProvenanceMsg, which allows these bank send messages to match the type
/// used for contract execution routes.
pub fn calculate_verifier_cost_messages(
    verifier: &VerifierDetail,
) -> AssetResult<Vec<CosmosMsg<ProvenanceMsg>>> {
    let mut cost_messages: Vec<CosmosMsg<ProvenanceMsg>> = vec![];
    // The total funds disbursed across fees are equal to the cost multiplied by the fee percent
    let fee_total = verifier.onboarding_cost.mul(verifier.fee_percent);
    let denom = &verifier.onboarding_denom;
    // The onboarding_cost is a u128, so attempting to subtract the fee total from it when the fee total is a greater value
    // will result in an unhandled panic.  Detect this potentiality for a misconfigured verifier and exit early to prevent
    // future head-scratching (panics are very difficult to debug due to redacted responses)
    if fee_total > verifier.onboarding_cost {
        return ContractError::generic(
            format!("misconfigured verifier data! calculated fee total ({}{}) was greater than the total cost of onboarding ({}{})", 
            fee_total,
             denom,
             verifier.onboarding_cost,
             denom,
        )).to_err();
    }
    // The total funds disbursed to the verifier itself is the remainder from subtracting the fee cost from the onboarding cost
    let verifier_cost = verifier.onboarding_cost - fee_total;
    // If all the fee totals plus the verifier cost do not equate to the onboarding cost, then the verifier is misconfigured
    let inner_fee_distribution_sum = verifier
        .fee_destinations
        .iter()
        .map(|d| fee_total.mul(d.fee_percent))
        .sum::<Uint128>();
    if verifier.onboarding_cost != verifier_cost + inner_fee_distribution_sum {
        return ContractError::generic(
            format!("misconfigured verifier data! expected onboarding cost to total {}{}, but total costs were {}{} (verifier) + {}{} (fee destination sum) = {}{}",
            verifier.onboarding_cost,
            denom,
            verifier_cost,
            denom,
            inner_fee_distribution_sum,
            denom,
            verifier_cost + inner_fee_distribution_sum,
            denom,
        )).to_err();
    }
    // Append a bank send message from the contract to the verifier for the cost if the verifier receives funds
    if verifier_cost > Uint128::zero() {
        cost_messages.push(bank_send(&verifier.address, verifier_cost.u128(), denom));
    }
    // Append a message for each destination
    verifier.fee_destinations.iter().for_each(|destination| {
        cost_messages.push(bank_send(
            &destination.address,
            fee_total.mul(destination.fee_percent).u128(),
            denom,
        ));
    });
    cost_messages.to_ok()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{BankMsg, CosmosMsg, Decimal, Uint128};
    use provwasm_std::ProvenanceMsg;

    use crate::{
        core::{
            error::ContractError,
            types::{fee_destination::FeeDestination, verifier_detail::VerifierDetail},
        },
        util::constants::NHASH,
    };

    use super::calculate_verifier_cost_messages;

    #[test]
    fn test_invalid_verifier_greater_fee_than_onboarding_cost() {
        // This verifier tries to send 150% of the fee to the fee destination. NO BUENO!
        let verifier = VerifierDetail::new(
            "address",
            Uint128::new(100),
            NHASH,
            Decimal::percent(150),
            vec![FeeDestination::new("fee", Decimal::percent(100))],
        );
        let error = calculate_verifier_cost_messages(&verifier).unwrap_err();
        match error {
            ContractError::GenericError { msg } => {
                assert_eq!(
                    "misconfigured verifier data! calculated fee total (150nhash) was greater than the total cost of onboarding (100nhash)",
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
        let verifier = VerifierDetail::new(
            "address",
            Uint128::new(100),
            NHASH,
            // Take half of the onboarding cost as a fee, but only request that 50% of that cost to be disbursed
            Decimal::percent(50),
            // All fee destinations should always add up to 100% (as enforced by validation)
            vec![FeeDestination::new("fee", Decimal::percent(50))],
        );
        let error = calculate_verifier_cost_messages(&verifier).unwrap_err();
        match error {
            ContractError::GenericError { msg } => {
                assert_eq!(
                    "misconfigured verifier data! expected onboarding cost to total 100nhash, but total costs were 50nhash (verifier) + 25nhash (fee destination sum) = 75nhash",
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
        let verifier = VerifierDetail::new(
            "verifier",
            Uint128::new(100),
            NHASH,
            Decimal::zero(),
            vec![],
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
        let verifier = VerifierDetail::new(
            "verifier",
            Uint128::new(100),
            NHASH,
            Decimal::percent(100),
            vec![FeeDestination::new(
                "fee-destination",
                Decimal::percent(100),
            )],
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
        let verifier = VerifierDetail::new(
            "verifier",
            Uint128::new(100),
            NHASH,
            Decimal::percent(50),
            vec![FeeDestination::new(
                "fee-destination",
                Decimal::percent(100),
            )],
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
        let verifier = VerifierDetail::new(
            "verifier",
            Uint128::new(200),
            NHASH,
            Decimal::percent(50),
            vec![
                FeeDestination::new("first", Decimal::percent(20)),
                FeeDestination::new("second", Decimal::percent(20)),
                FeeDestination::new("third", Decimal::percent(40)),
                FeeDestination::new("fourth", Decimal::percent(5)),
                FeeDestination::new("fifth", Decimal::percent(15)),
            ],
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
            "expected 20 percent of the remainder to be sent to the first fee destination",
        );
        test_messages_contains_send_for_address(
            &messages,
            "second",
            20,
            NHASH,
            "expected 20 percent of the remainder to be sent to the second fee destination",
        );
        test_messages_contains_send_for_address(
            &messages,
            "third",
            40,
            NHASH,
            "expected 40 percent of the remainder to be sent to the third fee destination",
        );
        test_messages_contains_send_for_address(
            &messages,
            "fourth",
            5,
            NHASH,
            "expected 5 percent of the remainder to be sent to the fourth fee destination",
        );
        test_messages_contains_send_for_address(
            &messages,
            "fifth",
            15,
            NHASH,
            "expected 15 percent of the remainder to be sent to the fifth fee destination",
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
