use std::ops::Mul;

use cosmwasm_std::{BankMsg, CosmosMsg, Uint128};

use crate::core::{error::ContractError, state::ValidatorDetail};

use super::{aliases::ContractResult, functions::bank_send, traits::ResultExtensions};

/// This function distributes funds from the sender address to the targets defined by a ValidatorDetail.
/// It breaks down all percentages defined in the validator detail's fee destinations and core onboarding
/// cost to derive a variable sized vector of destination messages.
pub fn calculate_validator_cost_messages(
    validator: &ValidatorDetail,
) -> ContractResult<Vec<CosmosMsg<BankMsg>>> {
    let mut cost_messages = vec![];
    // The total funds disbursed across fees are equal to the cost multiplied by the fee percent
    let fee_total = validator.onboarding_cost.mul(validator.fee_percent);
    let denom = &validator.onboarding_denom;
    // The onboarding_cost is a u128, so attempting to subtract the fee total from it when the fee total is a greater value
    // will result in an unhandled panic.  Detect this potentiality for a misconfigured validator and exit early to prevent
    // future head-scratching (panics are very difficult to debug due to redacted responses)
    if fee_total > validator.onboarding_cost {
        return ContractError::std_err(
            format!("misconfigured validator data! calculated fee total ({}{}) was greater than the total cost of onboarding ({}{})", 
            fee_total,
             denom,
             validator.onboarding_cost,
             denom,
        )).to_err();
    }
    // The total finds disbursed to the validator itself is the remainder from subtracting the fee cost from the onboarding cost
    let validator_cost = validator.onboarding_cost - fee_total;
    // If all the fee totals plus the validator cost do not equate to the onboarding cost, then the validator is misconfigured
    let inner_fee_distribution_sum = validator
        .fee_destinations
        .iter()
        .map(|d| fee_total.mul(d.fee_percent))
        .sum::<Uint128>();
    if validator.onboarding_cost != validator_cost + inner_fee_distribution_sum {
        return ContractError::std_err(
            format!("misconfigured validator data! expected onboarding cost to total {}{}, but total costs were {}{} (validator) + {}{} (fee destination sum) = {}{}",
            validator.onboarding_cost,
            denom,
            validator_cost,
            denom,
            inner_fee_distribution_sum,
            denom,
            validator_cost + inner_fee_distribution_sum,
            denom,
        )).to_err();
    }
    // Append a bank send message from the contract to the validator for the cost if the validator receives funds
    if validator_cost > Uint128::zero() {
        cost_messages.push(bank_send(&validator.address, validator_cost.u128(), denom));
    }
    // Append a message for each destination
    validator.fee_destinations.iter().for_each(|destination| {
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
    use cosmwasm_std::{BankMsg, CosmosMsg, Decimal, StdError, Uint128};

    use crate::{
        core::{
            error::ContractError,
            state::{FeeDestination, ValidatorDetail},
        },
        util::constants::NHASH,
    };

    use super::calculate_validator_cost_messages;

    #[test]
    fn test_invalid_validator_greater_fee_than_onboarding_cost() {
        // This validator tries to send 150% of the fee to the fee destination. NO BUENO!
        let validator = ValidatorDetail::new(
            "address",
            Uint128::new(100),
            NHASH,
            Decimal::percent(150),
            vec![FeeDestination::new("fee", Decimal::percent(100))],
        );
        match calculate_validator_cost_messages(&validator).unwrap_err() {
            ContractError::Std(e) => match e {
                StdError::GenericErr { msg, .. } => {
                    assert_eq!(
                            "misconfigured validator data! calculated fee total (150nhash) was greater than the total cost of onboarding (100nhash)",
                            msg.as_str(),
                            "unexpected error message generated",
                        );
                }
                _ => panic!(
                    "unexpected cosmwasm StdError encountered when providing an invalid validator"
                ),
            },
            _ => panic!("unexpected error encountered when providing a bad validator"),
        }
    }

    #[test]
    fn test_invalid_validator_fee_destination_sum_mismatch() {
        let validator = ValidatorDetail::new(
            "address",
            Uint128::new(100),
            NHASH,
            // Take half of the onboarding cost as a fee, but only request that 50% of that cost to be disbursed
            Decimal::percent(50),
            // All fee destinations should always add up to 100% (as enforced by validation)
            vec![FeeDestination::new("fee", Decimal::percent(50))],
        );
        match calculate_validator_cost_messages(&validator).unwrap_err() {
            ContractError::Std(e) => match e {
                StdError::GenericErr { msg, .. } => {
                    assert_eq!(
                        "misconfigured validator data! expected onboarding cost to total 100nhash, but total costs were 50nhash (validator) + 25nhash (fee destination sum) = 75nhash",
                        msg.as_str(),
                        "unexpected error message generated",
                    );
                }
                _ => panic!(
                    "unexpected cosmwasm StdError encountered when providing an invalid validator"
                ),
            },
            _ => panic!("unepected error encountered when providing a bad validator"),
        }
    }

    #[test]
    fn test_only_send_to_validator() {
        let validator = ValidatorDetail::new(
            "validator",
            Uint128::new(100),
            NHASH,
            Decimal::zero(),
            vec![],
        );
        let messages = calculate_validator_cost_messages(&validator)
            .expect("validation should pass and messages should be returned");
        assert_eq!(
            1,
            messages.len(),
            "expected only a single message to be sent"
        );
        test_messages_contains_send_for_address(
            &messages,
            "validator",
            100,
            NHASH,
            "expected all funds to be sent to the validator",
        );
    }

    #[test]
    fn test_only_send_to_single_fee_destination() {
        let validator = ValidatorDetail::new(
            "validator",
            Uint128::new(100),
            NHASH,
            Decimal::percent(100),
            vec![FeeDestination::new(
                "fee-destination",
                Decimal::percent(100),
            )],
        );
        let messages = calculate_validator_cost_messages(&validator)
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
    fn test_even_split_between_validator_and_single_fee_destination() {
        let validator = ValidatorDetail::new(
            "validator",
            Uint128::new(100),
            NHASH,
            Decimal::percent(50),
            vec![FeeDestination::new(
                "fee-destination",
                Decimal::percent(100),
            )],
        );
        let messages = calculate_validator_cost_messages(&validator)
            .expect("validation should pass and messages should be returned");
        assert_eq!(2, messages.len(), "expected two messages to be sent",);
        test_messages_contains_send_for_address(
            &messages,
            "validator",
            50,
            NHASH,
            "expected half of the funds to be sent to the validator",
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
    fn test_many_fee_destinations_and_some_to_validator() {
        let validator = ValidatorDetail::new(
            "validator",
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
        let messages = calculate_validator_cost_messages(&validator)
            .expect("validation should pass and messages should be returned");
        assert_eq!(6, messages.len(), "expected six messages to be sent");
        test_messages_contains_send_for_address(
            &messages,
            "validator",
            100,
            NHASH,
            "expected half of all funds to be sent to the validator",
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
        messages: &[CosmosMsg<BankMsg>],
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
