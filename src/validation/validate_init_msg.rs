use crate::core::error::ContractError;
use crate::core::msg::InitMsg;
use crate::core::state::{AssetDefinition, FeeDestination, ValidatorDetail};
use crate::util::aliases::DepsC;
use crate::util::functions::{decimal_display_string, distinct_count_by_property};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Decimal, Uint128};
use std::ops::Mul;

pub fn validate_init_msg(msg: &InitMsg, deps: &DepsC) -> Result<(), ContractError> {
    let mut invalid_fields: Vec<String> = vec![];
    if msg.base_contract_name.is_empty() {
        invalid_fields.push("base_contract_name: must not be blank".to_string());
    }
    if distinct_count_by_property(&msg.asset_definitions, |def| &def.asset_type)
        != msg.asset_definitions.len()
    {
        invalid_fields.push(
            "asset_definitions: each definition must specify a unique asset type".to_string(),
        );
    }
    let mut asset_messages = msg
        .asset_definitions
        .iter()
        .flat_map(|asset| validate_asset_definition(asset, deps))
        .collect::<Vec<String>>();
    invalid_fields.append(&mut asset_messages);
    if !invalid_fields.is_empty() {
        ContractError::InvalidMessageFields {
            message_type: "Instantiate".to_string(),
            invalid_fields,
        }
        .to_err()
    } else {
        Ok(())
    }
}

fn validate_asset_definition(asset_definition: &AssetDefinition, deps: &DepsC) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if asset_definition.asset_type.is_empty() {
        invalid_fields.push("asset_definition:asset_type: must not be blank".to_string());
    }
    if asset_definition.validators.is_empty() {
        invalid_fields.push(
            "asset_definition:validators: at least one validator must be supplied per asset type"
                .to_string(),
        );
    }
    let mut validator_messages = asset_definition
        .validators
        .iter()
        .flat_map(|valid| validate_validator(valid, deps))
        .collect::<Vec<String>>();
    invalid_fields.append(&mut validator_messages);
    invalid_fields
}

fn validate_validator(validator: &ValidatorDetail, deps: &DepsC) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if deps.api.addr_validate(&validator.address).is_err() {
        invalid_fields.push("validator:address: must be a valid address".to_string());
    }
    if validator.fee_percent > Decimal::percent(100) {
        invalid_fields
            .push("validator:fee_percent: must be less than or equal to 100%".to_string());
    }
    let fee_total = validator.onboarding_cost.mul(validator.fee_percent);
    if validator.fee_percent > Decimal::zero() && fee_total == Uint128::zero() {
        invalid_fields.push(
            format!(
                "validator:fee_percent: non-zero fee percent of {} must cleanly multiply against onboarding cost of {}nhash to produce a non-zero result, but produced zero. Try increasing cost or fee percent",
                decimal_display_string(&validator.fee_percent),
                validator.onboarding_cost,
            )
        );
    }
    if validator.fee_percent != Decimal::percent(100) && fee_total >= validator.onboarding_cost {
        invalid_fields.push(
            format!(
                "validator:fee_percent: fee percent was set to {}, but after multiplying it by the onboarding cost of {}nhash, it resulted in a greater number: {}",
                decimal_display_string(&validator.fee_percent),
                validator.onboarding_cost,
                fee_total,
            )
        );
    }
    if validator.fee_destinations.is_empty() && validator.fee_percent != Decimal::zero() {
        invalid_fields.push(
            "validator:fee_percent: Cannot specify a non-zero fee percent if no fee destinations are supplied"
                .to_string(),
        );
    }
    if !validator.fee_destinations.is_empty() && validator.fee_percent == Decimal::zero() {
        invalid_fields.push("validator:fee_destinations: fee definitions cannot be provided when the fee percent is zero".to_string());
    }
    if !validator.fee_destinations.is_empty()
        && validator
            .fee_destinations
            .iter()
            .map(|d| d.fee_percent)
            .sum::<Decimal>()
            != Decimal::percent(100)
    {
        invalid_fields.push("validator:fee_destinations: Fee destinations' fee_percents must always sum to a 100% distribution".to_string());
    }
    if !validator.fee_destinations.is_empty() {
        let destination_sum = validator
            .fee_destinations
            .iter()
            .map(|d| fee_total.mul(d.fee_percent))
            .sum::<Uint128>();
        if fee_total != Uint128::zero() && destination_sum != fee_total {
            invalid_fields.push(
                format!(
                    "validator:fee_destinations: Fee destinations' fee percents must cleanly sum to the fee_total. Fee total: {}, Destination sum: {}",
                    fee_total,
                    destination_sum,
                )
            )
        }
    }
    let mut fee_destination_messages = validator
        .fee_destinations
        .iter()
        .flat_map(|destination| validate_destination(destination, deps))
        .collect::<Vec<String>>();
    invalid_fields.append(&mut fee_destination_messages);
    invalid_fields
}

fn validate_destination(destination: &FeeDestination, deps: &DepsC) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if deps.api.addr_validate(&destination.address).is_err() {
        invalid_fields.push("fee_destination:address: must be a valid address".to_string());
    }
    if destination.fee_percent > Decimal::percent(100) {
        invalid_fields
            .push("fee_destination:fee_percent: must be less than or equal to 100%".to_string());
    }
    if destination.fee_percent == Decimal::zero() {
        invalid_fields.push("fee_destination:fee_percent: must not be zero".to_string());
    }
    invalid_fields
}

#[cfg(test)]
pub mod tests {
    use crate::core::state::{FeeDestination, ValidatorDetail};
    use crate::validation::validate_init_msg::{validate_destination, validate_validator};
    use cosmwasm_std::{Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_valid_validator_with_no_fee_destinations() {
        let deps = mock_dependencies(&[]);
        let validator = ValidatorDetail::new(
            "good-address".to_string(),
            Uint128::new(100),
            Decimal::percent(0),
            vec![],
        );
        let response = validate_validator(&validator, &deps.as_ref());
        assert!(
            response.is_empty(),
            "a valid validator should pass validation and return no error messages, but got messages: {:?}",
            response,
        );
    }

    #[test]
    fn test_valid_validator_with_single_fee_destination() {
        let deps = mock_dependencies(&[]);
        let validator = ValidatorDetail::new(
            "good-address".to_string(),
            Uint128::new(1000),
            Decimal::percent(50),
            vec![FeeDestination::new(
                "gooder-address".to_string(),
                Decimal::percent(100),
            )],
        );
        let response = validate_validator(&validator, &deps.as_ref());
        assert!(
            response.is_empty(),
            "a valid validator should pass validation and return no error messages, but got messages: {:?}",
            response,
        );
    }

    #[test]
    fn test_valid_validator_with_multiple_fee_destinations() {
        let deps = mock_dependencies(&[]);
        let validator = ValidatorDetail::new(
            "good-address".to_string(),
            Uint128::new(150000),
            Decimal::percent(50),
            vec![
                FeeDestination::new("first".to_string(), Decimal::percent(20)),
                FeeDestination::new("second".to_string(), Decimal::percent(10)),
                FeeDestination::new("third".to_string(), Decimal::percent(30)),
                FeeDestination::new("fourth".to_string(), Decimal::percent(35)),
                FeeDestination::new("fifth".to_string(), Decimal::percent(5)),
            ],
        );
        let response = validate_validator(&validator, &deps.as_ref());
        assert!(
            response.is_empty(),
            "a valid validator should pass validation and return no error messages, but got messages: {:?}",
            response,
        );
    }

    #[test]
    fn test_invalid_validator_address() {
        test_invalid_validator(
            &ValidatorDetail::new(String::new(), Uint128::new(150), Decimal::zero(), vec![]),
            "validator:address: must be a valid address",
        );
    }

    #[test]
    fn test_invalid_validator_fee_percent_too_high() {
        test_invalid_validator(
            &ValidatorDetail::new(
                "address".to_string(),
                Uint128::new(1010),
                Decimal::percent(101),
                vec![FeeDestination::new(
                    "fee".to_string(),
                    Decimal::percent(100),
                )],
            ),
            "validator:fee_percent: must be less than or equal to 100%",
        );
    }

    #[test]
    fn test_invalid_validator_fee_percent_results_in_zero_funds_allocated() {
        // Try to take 1% of 1 nhash, which should end up as zero after decimals are dropped
        test_invalid_validator(
            &ValidatorDetail::new(
                "address".to_string(),
                Uint128::new(1),
                Decimal::percent(1),
                vec![FeeDestination::new(
                    "fee".to_string(),
                    Decimal::percent(100),
                )],
            ),
            "validator:fee_percent: non-zero fee percent of 1% must cleanly multiply against onboarding cost of 1nhash to produce a non-zero result, but produced zero. Try increasing cost or fee percent",
        );
    }

    #[test]
    fn test_valid_destination() {
        let deps = mock_dependencies(&[]);
        let destination = FeeDestination::new("good-address".to_string(), Decimal::percent(100));
        assert!(
            validate_destination(&destination, &deps.as_ref()).is_empty(),
            "a valid fee destination should pass validation and return no error messages",
        );
    }

    #[test]
    fn test_invalid_destination_address() {
        test_invalid_destination(
            &FeeDestination::new(String::new(), Decimal::percent(1)),
            "fee_destination:address: must be a valid address",
        );
    }

    #[test]
    fn test_invalid_destination_fee_percent_too_high() {
        test_invalid_destination(
            &FeeDestination::new("good-address".to_string(), Decimal::percent(101)),
            "fee_destination:fee_percent: must be less than or equal to 100%",
        );
    }

    #[test]
    fn test_invalid_destination_fee_percent_too_low() {
        test_invalid_destination(
            &FeeDestination::new("good-address".to_string(), Decimal::percent(0)),
            "fee_destination:fee_percent: must not be zero",
        );
    }

    fn test_invalid_validator(validator: &ValidatorDetail, expected_message: &str) {
        let deps = mock_dependencies(&[]);
        let results = validate_validator(&validator, &deps.as_ref());
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }

    fn test_invalid_destination(destination: &FeeDestination, expected_message: &str) {
        let deps = mock_dependencies(&[]);
        let results = validate_destination(&destination, &deps.as_ref());
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }
}
