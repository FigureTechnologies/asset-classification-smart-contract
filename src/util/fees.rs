use std::ops::Mul;

use cosmwasm_std::{Addr, CosmosMsg};
use provwasm_std::ProvenanceMsg;

use crate::core::state::ValidatorDetail;

use super::aliases::ContractResult;

pub fn calculate_validator_cost_messages(
    sender: &Addr,
    validator: &ValidatorDetail,
) -> ContractResult<Vec<CosmosMsg<ProvenanceMsg>>> {
    let mut fee_messages = vec![];
    // The total funds disbursed across fees are equal to the cost multiplied by the fee percent
    let fee_total = validator.onboarding_cost.mul(validator.fee_percent);
    println!("Fee total: {}", fee_total);
    // The total finds disbursed to the validator itself is the remainder from subtracting the fee cost from the onboarding cost
    let validator_cost = validator.onboarding_cost - fee_total;
    println!("Validator cost: {}", validator_cost);
    Ok(fee_messages)
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
        );
    }
}
