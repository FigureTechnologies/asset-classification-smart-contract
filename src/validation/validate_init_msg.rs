use crate::core::error::ContractError;
use crate::core::msg::InitMsg;
use crate::core::types::asset_definition::{AssetDefinition, AssetDefinitionInput};
use crate::core::types::fee_destination::FeeDestination;
use crate::core::types::verifier_detail::VerifierDetail;
use crate::util::aliases::AssetResult;
use crate::util::functions::{decimal_display_string, distinct_count_by_property};
use crate::util::scope_address_utils::bech32_string_to_addr;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Decimal, Uint128};
use std::ops::Mul;

pub fn validate_init_msg(msg: &InitMsg) -> AssetResult<()> {
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
        .flat_map(validate_asset_definition_input_internal)
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

pub fn validate_asset_definition_input(input: &AssetDefinitionInput) -> AssetResult<()> {
    validate_asset_definition(&input.as_asset_definition()?)
}

pub fn validate_asset_definition(asset_definition: &AssetDefinition) -> AssetResult<()> {
    let invalid_fields = validate_asset_definition_internal(asset_definition);
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

pub fn validate_verifier(verifier: &VerifierDetail) -> AssetResult<()> {
    validate_verifier_with_provided_errors(verifier, None)
}

pub fn validate_verifier_with_provided_errors(
    verifier: &VerifierDetail,
    provided_errors: Option<Vec<String>>,
) -> AssetResult<()> {
    let mut invalid_fields = validate_verifier_internal(verifier);
    if let Some(errors) = provided_errors {
        for error in errors {
            invalid_fields.push(error);
        }
    }
    if !invalid_fields.is_empty() {
        ContractError::InvalidMessageFields {
            message_type: "VerifierDetail".to_string(),
            invalid_fields,
        }
        .to_err()
    } else {
        Ok(())
    }
}

fn validate_asset_definition_input_internal(input: &AssetDefinitionInput) -> Vec<String> {
    match input.as_asset_definition() {
        // If the input can properly convert to an actual asset definition, return any invalid fields it contains
        Ok(definition) => validate_asset_definition_internal(&definition),
        // If the input cannot convert, then the scope spec conversion must be invalid. Just return the contract error's description of the problem
        Err(e) => vec![format!("Invalid scope spec identifier provided: {:?}", e)],
    }
}

fn validate_asset_definition_internal(asset_definition: &AssetDefinition) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if asset_definition.asset_type.is_empty() {
        invalid_fields.push("asset_definition:asset_type: must not be blank".to_string());
    }
    if asset_definition.scope_spec_address.is_empty() {
        invalid_fields.push("asset_definition:scope_spec_address: must not be blank".to_string());
    }
    if asset_definition.verifiers.is_empty() {
        invalid_fields.push(
            "asset_definition:verifiers: at least one verifier must be supplied per asset type"
                .to_string(),
        );
    }
    let mut verifier_messages = asset_definition
        .verifiers
        .iter()
        .flat_map(validate_verifier_internal)
        .collect::<Vec<String>>();
    invalid_fields.append(&mut verifier_messages);
    invalid_fields
}

fn validate_verifier_internal(verifier: &VerifierDetail) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if bech32_string_to_addr(&verifier.address).is_err() {
        invalid_fields.push("verifier:address: must be a valid address".to_string());
    }
    if verifier.onboarding_denom.is_empty() {
        invalid_fields.push("verifier:onboarding_denom: must not be blank".to_string());
    }
    if verifier.fee_percent > Decimal::percent(100) {
        invalid_fields.push("verifier:fee_percent: must be less than or equal to 100%".to_string());
    }
    let fee_total = verifier.onboarding_cost.mul(verifier.fee_percent);
    if verifier.fee_percent > Decimal::zero() && fee_total == Uint128::zero() {
        invalid_fields.push(
            format!(
                "verifier:fee_percent: non-zero fee percent of {} must cleanly multiply against onboarding cost of {}nhash to produce a non-zero result, but produced zero. Try increasing cost or fee percent",
                decimal_display_string(&verifier.fee_percent),
                verifier.onboarding_cost,
            )
        );
    }
    if verifier.fee_destinations.is_empty() && verifier.fee_percent != Decimal::zero() {
        invalid_fields.push(
            "verifier:fee_percent: cannot specify a non-zero fee percent if no fee destinations are supplied"
                .to_string(),
        );
    }
    if !verifier.fee_destinations.is_empty() && verifier.fee_percent == Decimal::zero() {
        invalid_fields.push("verifier:fee_destinations: fee destinations cannot be provided when the fee percent is zero".to_string());
    }
    if !verifier.fee_destinations.is_empty()
        && verifier
            .fee_destinations
            .iter()
            .map(|d| d.fee_percent)
            .sum::<Decimal>()
            != Decimal::percent(100)
    {
        invalid_fields.push("verifier:fee_destinations: fee destinations' fee_percents must always sum to a 100% distribution".to_string());
    }
    if !verifier.fee_destinations.is_empty() {
        let destination_sum = verifier
            .fee_destinations
            .iter()
            .map(|d| fee_total.mul(d.fee_percent))
            .sum::<Uint128>();
        if fee_total != Uint128::zero() && destination_sum != fee_total {
            invalid_fields.push(
                format!(
                    "verifier:fee_destinations: fee destinations' fee percents must cleanly sum to the fee_total. Fee total: {}nhash, Destination sum: {}nhash",
                    fee_total,
                    destination_sum,
                )
            )
        }
    }
    if distinct_count_by_property(&verifier.fee_destinations, |dest| &dest.address)
        != verifier.fee_destinations.len()
    {
        invalid_fields.push("verifier:fee_destinations: all fee destinations within a verifier must have unique addresses".to_string());
    }
    let mut fee_destination_messages = verifier
        .fee_destinations
        .iter()
        .flat_map(validate_destination_internal)
        .collect::<Vec<String>>();
    invalid_fields.append(&mut fee_destination_messages);
    invalid_fields
}

fn validate_destination_internal(destination: &FeeDestination) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if bech32_string_to_addr(&destination.address).is_err() {
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
    use crate::core::types::asset_definition::{AssetDefinition, AssetDefinitionInput};
    use crate::core::types::fee_destination::FeeDestination;
    use crate::core::types::scope_spec_identifier::ScopeSpecIdentifier;
    use crate::core::types::verifier_detail::VerifierDetail;
    use crate::util::constants::NHASH;
    use crate::util::traits::OptionExtensions;
    use crate::validation::validate_init_msg::{
        validate_asset_definition_input_internal, validate_asset_definition_internal,
        validate_destination_internal, validate_init_msg, validate_verifier_internal,
    };
    use cosmwasm_std::{Decimal, Uint128};

    #[test]
    fn test_valid_init_msg_no_definitions() {
        test_valid_init_msg(&InitMsg {
            base_contract_name: "asset".to_string(),
            bind_base_name: true,
            asset_definitions: vec![],
            is_test: false.to_some(),
        });
    }

    #[test]
    fn test_valid_init_msg_single_definition() {
        test_valid_init_msg(&InitMsg {
            base_contract_name: "asset".to_string(),
            bind_base_name: true,
            is_test: false.to_some(),
            asset_definitions: vec![AssetDefinitionInput::new(
                "heloc",
                ScopeSpecIdentifier::address("scopespec1qjy5xyvs5z0prm90w5l36l4dhu4qa3hupt"),
                vec![VerifierDetail::new(
                    "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                    Uint128::new(100),
                    NHASH,
                    Decimal::percent(100),
                    vec![FeeDestination::new(
                        "tp16e7gwxzr2g5ktfsa69mhy2qqtwxy3g3eansn95",
                        Decimal::percent(100),
                    )],
                )],
                None,
            )],
        });
    }

    #[test]
    fn test_valid_init_msg_multiple_definitions() {
        test_valid_init_msg(&InitMsg {
            base_contract_name: "asset".to_string(),
            bind_base_name: true,
            is_test: false.to_some(),
            asset_definitions: vec![
                AssetDefinitionInput::new(
                    "heloc",
                    ScopeSpecIdentifier::address("scopespec1qjy5xyvs5z0prm90w5l36l4dhu4qa3hupt"),
                    vec![VerifierDetail::new(
                        "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                        Uint128::new(100),
                        NHASH,
                        Decimal::percent(100),
                        vec![FeeDestination::new(
                            "tp16e7gwxzr2g5ktfsa69mhy2qqtwxy3g3eansn95",
                            Decimal::percent(100),
                        )],
                    )],
                    None,
                ),
                AssetDefinitionInput::new(
                    "mortgage",
                    ScopeSpecIdentifier::address("scopespec1qj8dy8pg5z0prmy89r9nvxlu7mnquegf86"),
                    vec![VerifierDetail::new(
                        "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                        Uint128::new(500),
                        NHASH,
                        Decimal::percent(50),
                        vec![
                            FeeDestination::new(
                                "tp1szfeeasdxjdj55sps0m8835wppkykj5wgkhu2p",
                                Decimal::percent(50),
                            ),
                            FeeDestination::new(
                                "tp1m2ar35p73amqxwaxgcya0tckd0nmm9l9xe74l7",
                                Decimal::percent(50),
                            ),
                        ],
                    )],
                    None,
                ),
                AssetDefinitionInput::new(
                    "pl",
                    ScopeSpecIdentifier::address("scopespec1qj4l668j5z0prmy458tk8lrsyv4quyn084"),
                    vec![
                        VerifierDetail::new(
                            "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                            Uint128::new(0),
                            NHASH,
                            Decimal::percent(0),
                            vec![],
                        ),
                        VerifierDetail::new(
                            "tp1aujf44ge8zydwckk8zwa5g548czys53dkcp2lq",
                            Uint128::new(1000000),
                            NHASH,
                            Decimal::percent(100),
                            vec![
                                FeeDestination::new(
                                    "tp1jdcwtaendn9y75jv9dqnmlm7dy8pv4kgu9fs9g",
                                    Decimal::percent(25),
                                ),
                                FeeDestination::new(
                                    "tp16dxelgu5nz7u0ygs3qu8tqzjv7gxq5wqucjclm",
                                    Decimal::percent(75),
                                ),
                            ],
                        ),
                    ],
                    None,
                ),
            ],
        });
    }

    #[test]
    fn test_invalid_init_msg_base_contract_name() {
        test_invalid_init_msg(
            &InitMsg {
                base_contract_name: String::new(),
                bind_base_name: true,
                is_test: false.to_some(),
                asset_definitions: vec![AssetDefinitionInput::new(
                    "heloc",
                    ScopeSpecIdentifier::address("scopespec1q3qgqhtdq9wygn5kjdny9fxjcugqj40jgz"),
                    vec![VerifierDetail::new(
                        "address",
                        Uint128::new(100),
                        NHASH,
                        Decimal::percent(100),
                        vec![FeeDestination::new("fee", Decimal::percent(100))],
                    )],
                    None,
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
                bind_base_name: true,
                is_test: false.to_some(),
                asset_definitions: vec![
                    AssetDefinitionInput::new(
                        "heloc",
                        ScopeSpecIdentifier::address(
                            "scopespec1qsk66j3kgkjyk4985ll8xmx68z9q4xfkjk",
                        ),
                        vec![],
                        None,
                    ),
                    AssetDefinitionInput::new(
                        "heloc",
                        ScopeSpecIdentifier::address(
                            "scopespec1q35x472s9tp54t4dcrygrdwdyl0qagw7y2",
                        ),
                        vec![],
                        None,
                    ),
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
                bind_base_name: true,
                is_test: false.to_some(),
                asset_definitions: vec![AssetDefinitionInput::new(
                    "",
                    ScopeSpecIdentifier::address("scopespec1q3wmtzhy5z0prm928emua4wcgq7sgq0gwn"),
                    vec![],
                    None,
                )],
            },
            "asset_definition:asset_type: must not be blank",
        );
    }

    #[test]
    fn test_valid_asset_definition() {
        let definition = AssetDefinition::new(
            "heloc",
            "scopespec1q3psjkty5z0prmyfqvflyhkvuw6sfx9tnz",
            vec![VerifierDetail::new(
                "tp1x24ueqfehs5ye7akkvhf2d67fmfs2zd55tsy2g",
                Uint128::new(100),
                NHASH,
                Decimal::percent(100),
                vec![FeeDestination::new(
                    "tp1pq2yt466fvxrf399atkxrxazptkkmp04x2slew",
                    Decimal::percent(100),
                )],
            )],
        );
        let response = validate_asset_definition_internal(&definition);
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
                "",
                "scope-spec-address",
                vec![VerifierDetail::new(
                    "address",
                    Uint128::new(100),
                    NHASH,
                    Decimal::percent(100),
                    vec![FeeDestination::new("fee", Decimal::percent(100))],
                )],
            ),
            "asset_definition:asset_type: must not be blank",
        );
    }

    #[test]
    fn test_invalid_asset_definition_scope_spec_address() {
        test_invalid_asset_definition(
            &AssetDefinition::new(
                "heloc",
                "",
                vec![VerifierDetail::new(
                    "address",
                    Uint128::new(100),
                    NHASH,
                    Decimal::percent(100),
                    vec![FeeDestination::new("fee", Decimal::percent(100))],
                )],
            ),
            "asset_definition:scope_spec_address: must not be blank",
        )
    }

    #[test]
    fn test_invalid_asset_definition_empty_verifiers() {
        test_invalid_asset_definition(
            &AssetDefinition::new("mortgage", "scope-spec-address", vec![]),
            "asset_definition:verifiers: at least one verifier must be supplied per asset type",
        );
    }

    #[test]
    fn test_invalid_asset_definition_picks_up_invalid_verifier_scenarios() {
        test_invalid_asset_definition(
            &AssetDefinition::new(
                "",
                "scope-spec-address",
                vec![VerifierDetail::new(
                    "",
                    Uint128::new(100),
                    NHASH,
                    Decimal::percent(100),
                    vec![FeeDestination::new("fee", Decimal::percent(100))],
                )],
            ),
            "verifier:address: must be a valid address",
        );
    }

    #[test]
    fn test_valid_verifier_with_no_fee_destinations() {
        let verifier = VerifierDetail::new(
            "tp1aujf44ge8zydwckk8zwa5g548czys53dkcp2lq",
            Uint128::new(100),
            NHASH,
            Decimal::percent(0),
            vec![],
        );
        let response = validate_verifier_internal(&verifier);
        assert!(
            response.is_empty(),
            "a valid verifier should pass validation and return no error messages, but got messages: {:?}",
            response,
        );
    }

    #[test]
    fn test_valid_verifier_with_single_fee_destination() {
        let verifier = VerifierDetail::new(
            "tp1z28j4v88vz3jyzz286a8627lfsclemk294essy",
            Uint128::new(1000),
            NHASH,
            Decimal::percent(50),
            vec![FeeDestination::new(
                "tp143p2m575fqre9rmaf9tpqwp9ux0mrzv83tdfh6",
                Decimal::percent(100),
            )],
        );
        let response = validate_verifier_internal(&verifier);
        assert!(
            response.is_empty(),
            "a valid verifier should pass validation and return no error messages, but got messages: {:?}",
            response,
        );
    }

    #[test]
    fn test_valid_verifier_with_multiple_fee_destinations() {
        let verifier = VerifierDetail::new(
            "tp16dxelgu5nz7u0ygs3qu8tqzjv7gxq5wqucjclm",
            Uint128::new(150000),
            NHASH,
            Decimal::percent(50),
            vec![
                FeeDestination::new(
                    "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                    Decimal::percent(20),
                ),
                FeeDestination::new(
                    "tp16e7gwxzr2g5ktfsa69mhy2qqtwxy3g3eansn95",
                    Decimal::percent(10),
                ),
                FeeDestination::new(
                    "tp1szfeeasdxjdj55sps0m8835wppkykj5wgkhu2p",
                    Decimal::percent(30),
                ),
                FeeDestination::new(
                    "tp1m2ar35p73amqxwaxgcya0tckd0nmm9l9xe74l7",
                    Decimal::percent(35),
                ),
                FeeDestination::new(
                    "tp1aujf44ge8zydwckk8zwa5g548czys53dkcp2lq",
                    Decimal::percent(5),
                ),
            ],
        );
        let response = validate_verifier_internal(&verifier);
        assert!(
            response.is_empty(),
            "a valid verifier should pass validation and return no error messages, but got messages: {:?}",
            response,
        );
    }

    #[test]
    fn test_invalid_verifier_address() {
        test_invalid_verifier(
            &VerifierDetail::new("", Uint128::new(150), NHASH, Decimal::zero(), vec![]),
            "verifier:address: must be a valid address",
        );
    }

    #[test]
    fn test_invalid_verifier_onboarding_denom() {
        test_invalid_verifier(
            &VerifierDetail::new("address", Uint128::new(100), "", Decimal::zero(), vec![]),
            "verifier:onboarding_denom: must not be blank",
        );
    }

    #[test]
    fn test_invalid_verifier_fee_percent_too_high() {
        test_invalid_verifier(
            &VerifierDetail::new(
                "address",
                Uint128::new(1010),
                NHASH,
                Decimal::percent(101),
                vec![FeeDestination::new("fee", Decimal::percent(100))],
            ),
            "verifier:fee_percent: must be less than or equal to 100%",
        );
    }

    #[test]
    fn test_invalid_verifier_fee_percent_results_in_zero_funds_allocated() {
        // Try to take 1% of 1 nhash, which should end up as zero after decimals are rounded
        test_invalid_verifier(
            &VerifierDetail::new(
                "address",
                Uint128::new(1),
                NHASH,
                Decimal::percent(1),
                vec![FeeDestination::new(
                    "fee",
                    Decimal::percent(100),
                )],
            ),
            "verifier:fee_percent: non-zero fee percent of 1% must cleanly multiply against onboarding cost of 1nhash to produce a non-zero result, but produced zero. Try increasing cost or fee percent",
        );
    }

    #[test]
    fn test_invalid_verifier_no_fee_destinations_but_fee_percent_provided() {
        test_invalid_verifier(
            &VerifierDetail::new(
                "address",
                Uint128::new(150),
                NHASH,
                Decimal::percent(100),
                vec![],
            ),
            "verifier:fee_percent: cannot specify a non-zero fee percent if no fee destinations are supplied",
        );
    }

    #[test]
    fn test_invalid_verifier_provided_fee_destinations_but_fee_percent_zero() {
        test_invalid_verifier(
            &VerifierDetail::new(
                "address",
                Uint128::new(150),
                NHASH,
                Decimal::percent(0),
                vec![FeeDestination::new(
                    "fee",
                    Decimal::percent(100),
                )],
            ),
            "verifier:fee_destinations: fee destinations cannot be provided when the fee percent is zero",
        );
    }

    #[test]
    fn test_invalid_verifier_fee_destinations_do_not_sum_correctly_single_destination() {
        test_invalid_verifier(
            &VerifierDetail::new(
                "address",
                Uint128::new(420),
                NHASH,
                Decimal::percent(50),
                vec![FeeDestination::new("first", Decimal::percent(99))],
            ),
            "verifier:fee_destinations: fee destinations' fee_percents must always sum to a 100% distribution",
        );
    }

    #[test]
    fn test_invalid_verifier_fee_destinations_do_not_sum_correctly_multiple_destinations() {
        test_invalid_verifier(
            &VerifierDetail::new(
                "address",
                Uint128::new(55),
                NHASH,
                Decimal::percent(100),
                vec![
                    FeeDestination::new("first", Decimal::percent(33)),
                    FeeDestination::new("second", Decimal::percent(33)),
                    FeeDestination::new("third", Decimal::percent(33)),
                ],
            ),
            "verifier:fee_destinations: fee destinations' fee_percents must always sum to a 100% distribution",
        );
    }

    #[test]
    fn test_invalid_verifier_destination_fee_percents_do_not_sum_to_correct_number() {
        test_invalid_verifier(
            &VerifierDetail::new(
                "address",
                Uint128::new(100),
                NHASH,
                Decimal::percent(20),
                vec![
                    // Trying to split 20 by 99% and by 1% will drop some of the numbers because the decimal places get removed after doing division.
                    // The 99% fee sound end up with 19nhash and the 1% fee should result in 0nhash, resulting in 19nhash as the total
                    FeeDestination::new("first", Decimal::percent(99)),
                    FeeDestination::new("second", Decimal::percent(1)),
                ],
            ),
            "verifier:fee_destinations: fee destinations' fee percents must cleanly sum to the fee_total. Fee total: 20nhash, Destination sum: 19nhash",
        );
    }

    #[test]
    fn test_invalid_verifier_destinations_contains_duplicate_address() {
        test_invalid_verifier(
            &VerifierDetail::new(
                "address",
                Uint128::new(100),
                NHASH,
                Decimal::percent(50),
                vec![
                    FeeDestination::new("fee-guy", Decimal::percent(50)),
                    FeeDestination::new("fee-guy", Decimal::percent(50)),
                ]
            ),
            "verifier:fee_destinations: all fee destinations within a verifier must have unique addresses",
        );
    }

    #[test]
    fn test_invalid_verifier_picks_up_invalid_fee_destination_scenarios() {
        test_invalid_verifier(
            &VerifierDetail::new(
                "address",
                Uint128::new(100),
                NHASH,
                Decimal::percent(100),
                vec![FeeDestination::new("", Decimal::percent(100))],
            ),
            "fee_destination:address: must be a valid address",
        );
    }

    #[test]
    fn test_valid_destination() {
        let destination = FeeDestination::new(
            "tp1362ax9s0gxr5yy636q2p9uuefeg8lhguvu6np5",
            Decimal::percent(100),
        );
        assert!(
            validate_destination_internal(&destination).is_empty(),
            "a valid fee destination should pass validation and return no error messages",
        );
    }

    #[test]
    fn test_invalid_destination_address() {
        test_invalid_destination(
            &FeeDestination::new("", Decimal::percent(1)),
            "fee_destination:address: must be a valid address",
        );
    }

    #[test]
    fn test_invalid_destination_fee_percent_too_high() {
        test_invalid_destination(
            &FeeDestination::new("good-address", Decimal::percent(101)),
            "fee_destination:fee_percent: must be less than or equal to 100%",
        );
    }

    #[test]
    fn test_invalid_destination_fee_percent_too_low() {
        test_invalid_destination(
            &FeeDestination::new("good-address", Decimal::percent(0)),
            "fee_destination:fee_percent: must not be zero",
        );
    }

    #[test]
    fn test_validate_asset_definition_input_internal_bad_scope_spec_identifier() {
        let error_strings = validate_asset_definition_input_internal(&AssetDefinitionInput::new(
            "heloc",
            ScopeSpecIdentifier::uuid("not even a real uuid at all"),
            vec![],
            None,
        ));
        assert_eq!(
            1, error_strings.len(),
            "only one error should be returned when the definition input cannot be converted into an asset definition",
        );
        assert!(
            error_strings.first().unwrap().as_str().contains("Invalid scope spec identifier provided: "),
            "unexpected error contents. should contain information about invalid scope spec conversion",
        );
    }

    fn test_valid_init_msg(msg: &InitMsg) {
        match validate_init_msg(&msg) {
            Ok(_) => (),
            Err(e) => match e {
                ContractError::InvalidMessageFields { invalid_fields, .. } => panic!(
                    "expected message to be valid, but failed with field messages: {:?}",
                    invalid_fields
                ),

                _ => panic!("unexpected contract error on failure: {:?}", e),
            },
        }
    }

    fn test_invalid_init_msg(msg: &InitMsg, expected_message: &str) {
        match validate_init_msg(&msg) {
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
                _ => panic!(
                    "unexpected error type on init msg validation failure: {:?}",
                    e
                ),
            },
        }
    }

    fn test_invalid_asset_definition(definition: &AssetDefinition, expected_message: &str) {
        let results = validate_asset_definition_internal(&definition);
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }

    fn test_invalid_verifier(verifier: &VerifierDetail, expected_message: &str) {
        let results = validate_verifier_internal(&verifier);
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }

    fn test_invalid_destination(destination: &FeeDestination, expected_message: &str) {
        let results = validate_destination_internal(&destination);
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }
}
