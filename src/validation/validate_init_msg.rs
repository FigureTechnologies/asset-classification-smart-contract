use crate::core::error::ContractError;
use crate::core::msg::InitMsg;
use crate::core::state::{AssetDefinition, FeeDestination, ValidatorDetail};
use crate::util::aliases::{ContractResult, DepsC};
use crate::util::functions::{decimal_display_string, distinct_count_by_property};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Decimal, Uint128};
use std::ops::Mul;

pub fn validate_init_msg(msg: &InitMsg, deps: &DepsC) -> ContractResult<()> {
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
        .flat_map(|asset| validate_asset_definition_internal(asset, deps))
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

pub fn validate_asset_definition(
    asset_definition: &AssetDefinition,
    deps: &DepsC,
) -> ContractResult<()> {
    let invalid_fields = validate_asset_definition_internal(asset_definition, deps);
    if !invalid_fields.is_empty() {
        ContractError::InvalidMessageFields {
            message_type: "AssetDefinition".to_string(),
            invalid_fields,
        }
        .to_err()
    } else {
        Ok(())
    }
}

pub fn validate_validator(validator: &ValidatorDetail, deps: &DepsC) -> ContractResult<()> {
    validate_validator_with_provided_errors(validator, deps, None)
}

pub fn validate_validator_with_provided_errors(
    validator: &ValidatorDetail,
    deps: &DepsC,
    provided_errors: Option<Vec<String>>,
) -> ContractResult<()> {
    let mut invalid_fields = validate_validator_internal(validator, deps);
    if let Some(errors) = provided_errors {
        for error in errors {
            invalid_fields.push(error);
        }
    }
    if !invalid_fields.is_empty() {
        ContractError::InvalidMessageFields {
            message_type: "ValidatorDetail".to_string(),
            invalid_fields,
        }
        .to_err()
    } else {
        Ok(())
    }
}

fn validate_asset_definition_internal(
    asset_definition: &AssetDefinition,
    deps: &DepsC,
) -> Vec<String> {
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
        .flat_map(|valid| validate_validator_internal(valid, deps))
        .collect::<Vec<String>>();
    invalid_fields.append(&mut validator_messages);
    invalid_fields
}

fn validate_validator_internal(validator: &ValidatorDetail, deps: &DepsC) -> Vec<String> {
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
    if validator.fee_destinations.is_empty() && validator.fee_percent != Decimal::zero() {
        invalid_fields.push(
            "validator:fee_percent: cannot specify a non-zero fee percent if no fee destinations are supplied"
                .to_string(),
        );
    }
    if !validator.fee_destinations.is_empty() && validator.fee_percent == Decimal::zero() {
        invalid_fields.push("validator:fee_destinations: fee destinations cannot be provided when the fee percent is zero".to_string());
    }
    if !validator.fee_destinations.is_empty()
        && validator
            .fee_destinations
            .iter()
            .map(|d| d.fee_percent)
            .sum::<Decimal>()
            != Decimal::percent(100)
    {
        invalid_fields.push("validator:fee_destinations: fee destinations' fee_percents must always sum to a 100% distribution".to_string());
    }
    if !validator.fee_destinations.is_empty() {
        let destination_sum = validator
            .fee_destinations
            .iter()
            .map(|d| fee_total.mul(d.fee_percent))
            .sum::<Uint128>();
        //panic!("Sum: {}", destination_sum);
        if fee_total != Uint128::zero() && destination_sum != fee_total {
            invalid_fields.push(
                format!(
                    "validator:fee_destinations: fee destinations' fee percents must cleanly sum to the fee_total. Fee total: {}nhash, Destination sum: {}nhash",
                    fee_total,
                    destination_sum,
                )
            )
        }
    }
    if distinct_count_by_property(&validator.fee_destinations, |dest| &dest.address)
        != validator.fee_destinations.len()
    {
        invalid_fields.push("validator:fee_destinations: all fee destinations within a validator must have unique addresses".to_string());
    }
    let mut fee_destination_messages = validator
        .fee_destinations
        .iter()
        .flat_map(|destination| validate_destination_internal(destination, deps))
        .collect::<Vec<String>>();
    invalid_fields.append(&mut fee_destination_messages);
    invalid_fields
}

fn validate_destination_internal(destination: &FeeDestination, deps: &DepsC) -> Vec<String> {
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
    use crate::core::error::ContractError;
    use crate::core::msg::InitMsg;
    use crate::core::state::{AssetDefinition, FeeDestination, ValidatorDetail};
    use crate::validation::validate_init_msg::{
        validate_asset_definition_internal, validate_destination_internal, validate_init_msg,
        validate_validator_internal,
    };
    use cosmwasm_std::{Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_valid_init_msg_no_definitions() {
        test_valid_init_msg(&InitMsg {
            base_contract_name: "asset".to_string(),
            asset_definitions: vec![],
        });
    }

    #[test]
    fn test_valid_init_msg_single_definition() {
        test_valid_init_msg(&InitMsg {
            base_contract_name: "asset".to_string(),
            asset_definitions: vec![AssetDefinition::new(
                "heloc".to_string(),
                vec![ValidatorDetail::new(
                    "address".to_string(),
                    Uint128::new(100),
                    Decimal::percent(100),
                    vec![FeeDestination::new(
                        "fee".to_string(),
                        Decimal::percent(100),
                    )],
                )],
            )],
        });
    }

    #[test]
    fn test_valid_init_msg_multiple_definitions() {
        test_valid_init_msg(&InitMsg {
            base_contract_name: "asset".to_string(),
            asset_definitions: vec![
                AssetDefinition::new(
                    "heloc".to_string(),
                    vec![ValidatorDetail::new(
                        "address".to_string(),
                        Uint128::new(100),
                        Decimal::percent(100),
                        vec![FeeDestination::new(
                            "fee".to_string(),
                            Decimal::percent(100),
                        )],
                    )],
                ),
                AssetDefinition::new(
                    "mortgage".to_string(),
                    vec![ValidatorDetail::new(
                        "address".to_string(),
                        Uint128::new(500),
                        Decimal::percent(50),
                        vec![
                            FeeDestination::new("mort-fees".to_string(), Decimal::percent(50)),
                            FeeDestination::new("other-fee".to_string(), Decimal::percent(50)),
                        ],
                    )],
                ),
                AssetDefinition::new(
                    "pl".to_string(),
                    vec![
                        ValidatorDetail::new(
                            "address".to_string(),
                            Uint128::new(0),
                            Decimal::percent(0),
                            vec![],
                        ),
                        ValidatorDetail::new(
                            "other-validator".to_string(),
                            Uint128::new(1000000),
                            Decimal::percent(100),
                            vec![
                                FeeDestination::new("community".to_string(), Decimal::percent(25)),
                                FeeDestination::new("figure".to_string(), Decimal::percent(75)),
                            ],
                        ),
                    ],
                ),
            ],
        });
    }

    #[test]
    fn test_invalid_init_msg_base_contract_name() {
        test_invalid_init_msg(
            &InitMsg {
                base_contract_name: String::new(),
                asset_definitions: vec![AssetDefinition::new(
                    "heloc".to_string(),
                    vec![ValidatorDetail::new(
                        "address".to_string(),
                        Uint128::new(100),
                        Decimal::percent(100),
                        vec![FeeDestination::new(
                            "fee".to_string(),
                            Decimal::percent(100),
                        )],
                    )],
                )],
            },
            "base_contract_name: must not be blank",
        );
    }

    #[test]
    fn test_invalid_init_msg_duplicate_asset_types() {
        test_invalid_init_msg(
            &InitMsg {
                base_contract_name: String::new(),
                asset_definitions: vec![
                    AssetDefinition::new("heloc".to_string(), vec![]),
                    AssetDefinition::new("heloc".to_string(), vec![]),
                ],
            },
            "asset_definitions: each definition must specify a unique asset type",
        );
    }

    #[test]
    fn test_invalid_init_msg_picks_up_invalid_asset_definition_scenarios() {
        test_invalid_init_msg(
            &InitMsg {
                base_contract_name: "asset".to_string(),
                asset_definitions: vec![AssetDefinition::new(String::new(), vec![])],
            },
            "asset_definition:asset_type: must not be blank",
        );
    }

    #[test]
    fn test_valid_asset_definition() {
        let deps = mock_dependencies(&[]);
        let definition = AssetDefinition::new(
            "heloc".to_string(),
            vec![ValidatorDetail::new(
                "address".to_string(),
                Uint128::new(100),
                Decimal::percent(100),
                vec![FeeDestination::new(
                    "fee".to_string(),
                    Decimal::percent(100),
                )],
            )],
        );
        let response = validate_asset_definition_internal(&definition, &deps.as_ref());
        assert!(
            response.is_empty(),
            "a valid asset definition should pass validation and return no error messages, but got messages: {:?}",
            response,
        );
    }

    #[test]
    fn test_invalid_asset_definition_asset_type() {
        test_invalid_asset_definition(
            &AssetDefinition::new(
                String::new(),
                vec![ValidatorDetail::new(
                    "address".to_string(),
                    Uint128::new(100),
                    Decimal::percent(100),
                    vec![FeeDestination::new(
                        "fee".to_string(),
                        Decimal::percent(100),
                    )],
                )],
            ),
            "asset_definition:asset_type: must not be blank",
        );
    }

    #[test]
    fn test_invalid_asset_definition_empty_validators() {
        test_invalid_asset_definition(
            &AssetDefinition::new("mortgage".to_string(), vec![]),
            "asset_definition:validators: at least one validator must be supplied per asset type",
        );
    }

    #[test]
    fn test_invalid_asset_definition_picks_up_invalid_validator_scenarios() {
        test_invalid_asset_definition(
            &AssetDefinition::new(
                String::new(),
                vec![ValidatorDetail::new(
                    String::new(),
                    Uint128::new(100),
                    Decimal::percent(100),
                    vec![FeeDestination::new(
                        "fee".to_string(),
                        Decimal::percent(100),
                    )],
                )],
            ),
            "validator:address: must be a valid address",
        );
    }

    #[test]
    fn test_valid_validator_with_no_fee_destinations() {
        let deps = mock_dependencies(&[]);
        let validator = ValidatorDetail::new(
            "good-address".to_string(),
            Uint128::new(100),
            Decimal::percent(0),
            vec![],
        );
        let response = validate_validator_internal(&validator, &deps.as_ref());
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
        let response = validate_validator_internal(&validator, &deps.as_ref());
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
        let response = validate_validator_internal(&validator, &deps.as_ref());
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
        // Try to take 1% of 1 nhash, which should end up as zero after decimals are rounded
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
    fn test_invalid_validator_no_fee_destinations_but_fee_percent_provided() {
        test_invalid_validator(
            &ValidatorDetail::new(
                "address".to_string(),
                Uint128::new(150),
                Decimal::percent(100),
                vec![],
            ),
            "validator:fee_percent: cannot specify a non-zero fee percent if no fee destinations are supplied",
        );
    }

    #[test]
    fn test_invalid_validator_provided_fee_destinations_but_fee_percent_zero() {
        test_invalid_validator(
            &ValidatorDetail::new(
                "address".to_string(),
                Uint128::new(150),
                Decimal::percent(0),
                vec![FeeDestination::new(
                    "fee".to_string(),
                    Decimal::percent(100),
                )],
            ),
            "validator:fee_destinations: fee destinations cannot be provided when the fee percent is zero",
        );
    }

    #[test]
    fn test_invalid_validator_fee_destinations_do_not_sum_correctly_single_destination() {
        test_invalid_validator(
            &ValidatorDetail::new(
                "address".to_string(),
                Uint128::new(420),
                Decimal::percent(50),
                vec![FeeDestination::new("first".to_string(), Decimal::percent(99))],
            ),
            "validator:fee_destinations: fee destinations' fee_percents must always sum to a 100% distribution",
        );
    }

    #[test]
    fn test_invalid_validator_fee_destinations_do_not_sum_correctly_multiple_destinations() {
        test_invalid_validator(
            &ValidatorDetail::new(
                "address".to_string(),
                Uint128::new(55),
                Decimal::percent(100),
                vec![
                    FeeDestination::new("first".to_string(), Decimal::percent(33)),
                    FeeDestination::new("second".to_string(), Decimal::percent(33)),
                    FeeDestination::new("third".to_string(), Decimal::percent(33)),
                ],
            ),
            "validator:fee_destinations: fee destinations' fee_percents must always sum to a 100% distribution",
        );
    }

    #[test]
    fn test_invalid_validator_destination_fee_percents_do_not_sum_to_correct_number() {
        test_invalid_validator(
            &ValidatorDetail::new(
                "address".to_string(),
                Uint128::new(100),
                Decimal::percent(20),
                vec![
                    // Trying to split 20 by 99% and by 1% will drop some of the numbers because the decimal places get removed after doing division.
                    // The 99% fee sound end up with 19nhash and the 1% fee should result in 0nhash, resulting in 19nhash as the total
                    FeeDestination::new("first".to_string(), Decimal::percent(99)),
                    FeeDestination::new("second".to_string(), Decimal::percent(1)),
                ],
            ),
            "validator:fee_destinations: fee destinations' fee percents must cleanly sum to the fee_total. Fee total: 20nhash, Destination sum: 19nhash",
        );
    }

    #[test]
    fn test_invalid_validator_destinations_contains_duplicate_address() {
        test_invalid_validator(
            &ValidatorDetail::new(
                "address".to_string(),
                Uint128::new(100),
                Decimal::percent(50),
                vec![
                    FeeDestination::new("fee-guy".to_string(), Decimal::percent(50)),
                    FeeDestination::new("fee-guy".to_string(), Decimal::percent(50)),
                ]
            ),
            "validator:fee_destinations: all fee destinations within a validator must have unique addresses",
        );
    }

    #[test]
    fn test_invalid_validator_picks_up_invalid_fee_destination_scenarios() {
        test_invalid_validator(
            &ValidatorDetail::new(
                "address".to_string(),
                Uint128::new(100),
                Decimal::percent(100),
                vec![FeeDestination::new(String::new(), Decimal::percent(100))],
            ),
            "fee_destination:address: must be a valid address",
        );
    }

    #[test]
    fn test_valid_destination() {
        let deps = mock_dependencies(&[]);
        let destination = FeeDestination::new("good-address".to_string(), Decimal::percent(100));
        assert!(
            validate_destination_internal(&destination, &deps.as_ref()).is_empty(),
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

    fn test_valid_init_msg(msg: &InitMsg) {
        let deps = mock_dependencies(&[]);
        match validate_init_msg(&msg, &deps.as_ref()) {
            Ok(_) => (),
            Err(e) => match e {
                ContractError::InvalidMessageFields { invalid_fields, .. } => panic!(
                    "expected message to be valid, but failed with field messages: {:?}",
                    invalid_fields
                ),

                _ => panic!("unexpected contract error on failure"),
            },
        }
    }

    fn test_invalid_init_msg(msg: &InitMsg, expected_message: &str) {
        let deps = mock_dependencies(&[]);
        match validate_init_msg(&msg, &deps.as_ref()) {
            Ok(_) => panic!("expected init msg to be invalid, but passed validation"),
            Err(e) => match e {
                ContractError::InvalidMessageFields {
                    message_type,
                    invalid_fields,
                } => {
                    assert_eq!(
                        "Instantiate",
                        message_type.as_str(),
                        "expected the invalid message type to be returned correctly"
                    );
                    assert!(
                        invalid_fields.contains(&expected_message.to_string()),
                        "expected error message `{}` was not contained in the response. Contained messages: {:?}",
                        expected_message,
                        invalid_fields,
                    );
                }
                _ => panic!("unexpected error type on init msg validation failure"),
            },
        }
    }

    fn test_invalid_asset_definition(definition: &AssetDefinition, expected_message: &str) {
        let deps = mock_dependencies(&[]);
        let results = validate_asset_definition_internal(&definition, &deps.as_ref());
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }

    fn test_invalid_validator(validator: &ValidatorDetail, expected_message: &str) {
        let deps = mock_dependencies(&[]);
        let results = validate_validator_internal(&validator, &deps.as_ref());
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }

    fn test_invalid_destination(destination: &FeeDestination, expected_message: &str) {
        let deps = mock_dependencies(&[]);
        let results = validate_destination_internal(&destination, &deps.as_ref());
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }
}
