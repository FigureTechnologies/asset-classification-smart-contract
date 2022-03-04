use std::ops::Mul;

use cosmwasm_std::{Addr, BankMsg, CosmosMsg};

use crate::core::{error::ContractError, state::ValidatorDetail};

use super::{aliases::ContractResult, functions::bank_send_nhash, traits::ResultExtensions};

/// This function distributes funds from the sender address to the targets defined by a ValidatorDetail.
/// It breaks down all percentages defined in the validator detail's fee destinations and core onboarding
/// cost to derive a variable sized vector of destination messages.
pub fn calculate_validator_cost_messages(
    sender_address: &Addr,
    validator: &ValidatorDetail,
) -> ContractResult<Vec<CosmosMsg<BankMsg>>> {
    let mut cost_messages = vec![];
    // The total funds disbursed across fees are equal to the cost multiplied by the fee percent
    let fee_total = validator.onboarding_cost.mul(validator.fee_percent);
    // The total finds disbursed to the validator itself is the remainder from subtracting the fee cost from the onboarding cost
    let validator_cost = validator.onboarding_cost - fee_total;
    if validator.onboarding_cost != fee_total + validator_cost {
        return ContractError::std_err(format!("misconfigured validator data! expected onboarding cost to total {}, but total costs were {} (validator) + {} (fees) = {}", validator.onboarding_cost, validator_cost, fee_total, validator_cost + fee_total)).to_err();
    }
    // Append a bank send message from the contract to the validator for the cost
    cost_messages.push(bank_send_nhash(
        sender_address.as_str(),
        validator_cost.u128(),
    ));
    cost_messages.to_ok()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, Decimal, Uint128};

    use crate::core::state::ValidatorDetail;

    use super::calculate_validator_cost_messages;

    #[test]
    fn test_some_math() {
        calculate_validator_cost_messages(
            &Addr::unchecked("No-u"),
            &ValidatorDetail::new(
                "no-u-2".to_string(),
                Uint128::new(150),
                Decimal::percent(50),
                vec![],
            ),
        )
        .unwrap();
    }
}
