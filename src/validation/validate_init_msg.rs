use crate::core::error::ContractError;
use crate::core::msg::InitMsg;
use crate::core::types::asset_definition::{AssetDefinitionInputV2, AssetDefinitionV2};
use crate::core::types::fee_destination::FeeDestinationV2;
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::util::aliases::AssetResult;
use crate::util::functions::distinct_count_by_property;
use crate::util::scope_address_utils::bech32_string_to_addr;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::Uint128;

/// Validates the integrity of an intercepted [InitMsg](crate::core::msg::InitMsg) and its
/// associated [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2) values.
///
/// # Parameters
///
/// * `msg` The init msg sent during the [instantiation](crate::contract::instantiate) process.
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

/// Validates that an asset definition input value is properly formed, ensuring that all fields
/// are properly set and fees are established correctly.
///
/// # Parameters
///
/// * `input` The asset definition input value to validate for issues.
pub fn validate_asset_definition_input(input: &AssetDefinitionInputV2) -> AssetResult<()> {
    validate_asset_definition(&input.as_asset_definition()?)
}

/// Validates that an asset definition value is properly formed, ensuring that all fields are
/// properly set and fees are established correctly.
///
/// # Parameters
///
/// * `asset_definition` The asset definition value to validate for issues.
pub fn validate_asset_definition(asset_definition: &AssetDefinitionV2) -> AssetResult<()> {
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

/// Validates that a verifier detail is properly formed, ensuring that all fields are properly set
/// and fees are established correctly.
///
/// # Parameters
///
/// * `verifier` The verifier detail value to validate for issues.
pub fn validate_verifier(verifier: &VerifierDetailV2) -> AssetResult<()> {
    validate_verifier_with_provided_errors(verifier, None)
}

/// Validates that a verifier detail is properly formed, ensuring that all fields are properly set
/// and fees are established correctly, with an additional funnel for exiting errors encountered
/// beforehand.
///
/// # Parameters
///
/// * `verifier` The verifier detail value to validate for issues.
/// * `provided_errors` Any existing errors encountered before validation of the verifier detail.
pub fn validate_verifier_with_provided_errors(
    verifier: &VerifierDetailV2,
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

fn validate_asset_definition_input_internal(input: &AssetDefinitionInputV2) -> Vec<String> {
    match input.as_asset_definition() {
        // If the input can properly convert to an actual asset definition, return any invalid fields it contains
        Ok(definition) => validate_asset_definition_internal(&definition),
        // If the input cannot convert, then the scope spec conversion must be invalid. Just return the contract error's description of the problem
        Err(e) => vec![format!("Invalid scope spec identifier provided: {:?}", e)],
    }
}

fn validate_asset_definition_internal(asset_definition: &AssetDefinitionV2) -> Vec<String> {
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

fn validate_verifier_internal(verifier: &VerifierDetailV2) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if bech32_string_to_addr(&verifier.address).is_err() {
        invalid_fields.push("verifier:address: must be a valid address".to_string());
    }
    if verifier.onboarding_denom.is_empty() {
        invalid_fields.push("verifier:onboarding_denom: must not be blank".to_string());
    }
    if !verifier.fee_destinations.is_empty()
        && verifier
            .fee_destinations
            .iter()
            .map(|d| d.fee_amount.u128())
            .sum::<u128>()
            > verifier.onboarding_cost.u128()
    {
        invalid_fields.push(
            "verifier:fee_destinations:fee_amounts must sum to be less than or equal to the onboarding cost".to_string(),
        );
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

fn validate_destination_internal(destination: &FeeDestinationV2) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if bech32_string_to_addr(&destination.address).is_err() {
        invalid_fields.push("fee_destination:address: must be a valid address".to_string());
    }
    if destination.fee_amount == Uint128::zero() {
        invalid_fields.push("fee_destination:fee_amount: must not be zero".to_string());
    }
    invalid_fields
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
pub mod tests {
    use crate::core::error::ContractError;
    use crate::core::msg::InitMsg;
    use crate::core::types::asset_definition::{AssetDefinitionInputV2, AssetDefinitionV2};
    use crate::core::types::entity_detail::EntityDetail;
    use crate::core::types::fee_destination::FeeDestinationV2;
    use crate::core::types::scope_spec_identifier::ScopeSpecIdentifier;
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::testutil::test_utilities::get_default_entity_detail;
    use crate::util::constants::NHASH;
    use crate::util::traits::OptionExtensions;
    use crate::validation::validate_init_msg::{
        validate_asset_definition_input_internal, validate_asset_definition_internal,
        validate_destination_internal, validate_init_msg, validate_verifier_internal,
    };
    use cosmwasm_std::Uint128;

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
            asset_definitions: vec![AssetDefinitionInputV2::new(
                "heloc",
                ScopeSpecIdentifier::address("scopespec1qjy5xyvs5z0prm90w5l36l4dhu4qa3hupt")
                    .to_serialized_enum(),
                vec![VerifierDetailV2::new(
                    "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                    Uint128::new(100),
                    NHASH,
                    vec![FeeDestinationV2::new(
                        "tp16e7gwxzr2g5ktfsa69mhy2qqtwxy3g3eansn95",
                        Uint128::new(100),
                    )],
                    get_default_entity_detail().to_some(),
                )],
                None,
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
                AssetDefinitionInputV2::new(
                    "heloc",
                    ScopeSpecIdentifier::address("scopespec1qjy5xyvs5z0prm90w5l36l4dhu4qa3hupt")
                        .to_serialized_enum(),
                    vec![VerifierDetailV2::new(
                        "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                        Uint128::new(100),
                        NHASH,
                        vec![FeeDestinationV2::new(
                            "tp16e7gwxzr2g5ktfsa69mhy2qqtwxy3g3eansn95",
                            Uint128::new(100),
                        )],
                        get_default_entity_detail().to_some(),
                    )],
                    None,
                    None,
                ),
                AssetDefinitionInputV2::new(
                    "mortgage",
                    ScopeSpecIdentifier::address("scopespec1qj8dy8pg5z0prmy89r9nvxlu7mnquegf86")
                        .to_serialized_enum(),
                    vec![VerifierDetailV2::new(
                        "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                        Uint128::new(500),
                        NHASH,
                        vec![
                            FeeDestinationV2::new(
                                "tp1szfeeasdxjdj55sps0m8835wppkykj5wgkhu2p",
                                Uint128::new(125),
                            ),
                            FeeDestinationV2::new(
                                "tp1m2ar35p73amqxwaxgcya0tckd0nmm9l9xe74l7",
                                Uint128::new(125),
                            ),
                        ],
                        get_default_entity_detail().to_some(),
                    )],
                    None,
                    None,
                ),
                AssetDefinitionInputV2::new(
                    "pl",
                    ScopeSpecIdentifier::address("scopespec1qj4l668j5z0prmy458tk8lrsyv4quyn084")
                        .to_serialized_enum(),
                    vec![
                        VerifierDetailV2::new(
                            "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                            Uint128::new(0),
                            NHASH,
                            vec![],
                            EntityDetail::new(
                                "Freebies",
                                "We validate fo free!",
                                "http://www.yahoo.com/",
                                "https://github.com/kelseyhightower/nocode",
                            )
                            .to_some(),
                        ),
                        VerifierDetailV2::new(
                            "tp1aujf44ge8zydwckk8zwa5g548czys53dkcp2lq",
                            Uint128::new(1000000),
                            NHASH,
                            vec![
                                FeeDestinationV2::new(
                                    "tp1jdcwtaendn9y75jv9dqnmlm7dy8pv4kgu9fs9g",
                                    Uint128::new(250000),
                                ),
                                FeeDestinationV2::new(
                                    "tp16dxelgu5nz7u0ygs3qu8tqzjv7gxq5wqucjclm",
                                    Uint128::new(750000),
                                ),
                            ],
                            get_default_entity_detail().to_some(),
                        ),
                    ],
                    None,
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
                asset_definitions: vec![AssetDefinitionInputV2::new(
                    "heloc",
                    ScopeSpecIdentifier::address("scopespec1q3qgqhtdq9wygn5kjdny9fxjcugqj40jgz")
                        .to_serialized_enum(),
                    vec![VerifierDetailV2::new(
                        "address",
                        Uint128::new(100),
                        NHASH,
                        vec![FeeDestinationV2::new("fee", Uint128::new(100))],
                        get_default_entity_detail().to_some(),
                    )],
                    None,
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
                    AssetDefinitionInputV2::new(
                        "heloc",
                        ScopeSpecIdentifier::address(
                            "scopespec1qsk66j3kgkjyk4985ll8xmx68z9q4xfkjk",
                        )
                        .to_serialized_enum(),
                        vec![],
                        None,
                        None,
                    ),
                    AssetDefinitionInputV2::new(
                        "heloc",
                        ScopeSpecIdentifier::address(
                            "scopespec1q35x472s9tp54t4dcrygrdwdyl0qagw7y2",
                        )
                        .to_serialized_enum(),
                        vec![],
                        None,
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
                asset_definitions: vec![AssetDefinitionInputV2::new(
                    "",
                    ScopeSpecIdentifier::address("scopespec1q3wmtzhy5z0prm928emua4wcgq7sgq0gwn")
                        .to_serialized_enum(),
                    vec![],
                    None,
                    None,
                )],
            },
            "asset_definition:asset_type: must not be blank",
        );
    }

    #[test]
    fn test_valid_asset_definition() {
        let definition = AssetDefinitionV2::new(
            "heloc",
            "scopespec1q3psjkty5z0prmyfqvflyhkvuw6sfx9tnz",
            vec![VerifierDetailV2::new(
                "tp1x24ueqfehs5ye7akkvhf2d67fmfs2zd55tsy2g",
                Uint128::new(100),
                NHASH,
                vec![FeeDestinationV2::new(
                    "tp1pq2yt466fvxrf399atkxrxazptkkmp04x2slew",
                    Uint128::new(100),
                )],
                get_default_entity_detail().to_some(),
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
            &AssetDefinitionV2::new(
                "",
                "scope-spec-address",
                vec![VerifierDetailV2::new(
                    "address",
                    Uint128::new(100),
                    NHASH,
                    vec![FeeDestinationV2::new("fee", Uint128::new(100))],
                    get_default_entity_detail().to_some(),
                )],
            ),
            "asset_definition:asset_type: must not be blank",
        );
    }

    #[test]
    fn test_invalid_asset_definition_scope_spec_address() {
        test_invalid_asset_definition(
            &AssetDefinitionV2::new(
                "heloc",
                "",
                vec![VerifierDetailV2::new(
                    "address",
                    Uint128::new(100),
                    NHASH,
                    vec![FeeDestinationV2::new("fee", Uint128::new(100))],
                    get_default_entity_detail().to_some(),
                )],
            ),
            "asset_definition:scope_spec_address: must not be blank",
        )
    }

    #[test]
    fn test_invalid_asset_definition_empty_verifiers() {
        test_invalid_asset_definition(
            &AssetDefinitionV2::new("mortgage", "scope-spec-address", vec![]),
            "asset_definition:verifiers: at least one verifier must be supplied per asset type",
        );
    }

    #[test]
    fn test_invalid_asset_definition_picks_up_invalid_verifier_scenarios() {
        test_invalid_asset_definition(
            &AssetDefinitionV2::new(
                "",
                "scope-spec-address",
                vec![VerifierDetailV2::new(
                    "",
                    Uint128::new(100),
                    NHASH,
                    vec![FeeDestinationV2::new("fee", Uint128::new(100))],
                    get_default_entity_detail().to_some(),
                )],
            ),
            "verifier:address: must be a valid address",
        );
    }

    #[test]
    fn test_valid_verifier_with_no_fee_destinations() {
        let verifier = VerifierDetailV2::new(
            "tp1aujf44ge8zydwckk8zwa5g548czys53dkcp2lq",
            Uint128::new(100),
            NHASH,
            vec![],
            get_default_entity_detail().to_some(),
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
        let verifier = VerifierDetailV2::new(
            "tp1z28j4v88vz3jyzz286a8627lfsclemk294essy",
            Uint128::new(1000),
            NHASH,
            vec![FeeDestinationV2::new(
                "tp143p2m575fqre9rmaf9tpqwp9ux0mrzv83tdfh6",
                Uint128::new(50),
            )],
            get_default_entity_detail().to_some(),
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
        let verifier = VerifierDetailV2::new(
            "tp16dxelgu5nz7u0ygs3qu8tqzjv7gxq5wqucjclm",
            Uint128::new(2000),
            NHASH,
            vec![
                FeeDestinationV2::new(
                    "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                    Uint128::new(100),
                ),
                FeeDestinationV2::new(
                    "tp16e7gwxzr2g5ktfsa69mhy2qqtwxy3g3eansn95",
                    Uint128::new(1650),
                ),
                FeeDestinationV2::new(
                    "tp1szfeeasdxjdj55sps0m8835wppkykj5wgkhu2p",
                    Uint128::new(50),
                ),
                FeeDestinationV2::new(
                    "tp1m2ar35p73amqxwaxgcya0tckd0nmm9l9xe74l7",
                    Uint128::new(199),
                ),
                FeeDestinationV2::new("tp1aujf44ge8zydwckk8zwa5g548czys53dkcp2lq", Uint128::new(1)),
            ],
            get_default_entity_detail().to_some(),
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
            &VerifierDetailV2::new(
                "",
                Uint128::new(150),
                NHASH,
                vec![],
                get_default_entity_detail().to_some(),
            ),
            "verifier:address: must be a valid address",
        );
    }

    #[test]
    fn test_invalid_verifier_onboarding_denom() {
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                "",
                vec![],
                get_default_entity_detail().to_some(),
            ),
            "verifier:onboarding_denom: must not be blank",
        );
    }

    #[test]
    fn test_invalid_verifier_fee_amount_too_high() {
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(1010),
                NHASH,
                vec![FeeDestinationV2::new("fee", Uint128::new(1011))],
                get_default_entity_detail().to_some(),
            ),
            "verifier:fee_destinations:fee_amounts must sum to be less than or equal to the onboarding cost",
        );
    }

    #[test]
    fn test_invalid_verifier_destinations_contains_duplicate_address() {
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                NHASH,
                vec![
                    FeeDestinationV2::new("fee-guy", Uint128::new(25)),
                    FeeDestinationV2::new("fee-guy", Uint128::new(25)),
                ],
                get_default_entity_detail().to_some(),
            ),
            "verifier:fee_destinations: all fee destinations within a verifier must have unique addresses",
        );
    }

    #[test]
    fn test_invalid_verifier_picks_up_invalid_fee_destination_scenarios() {
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                NHASH,
                vec![FeeDestinationV2::new("", Uint128::new(100))],
                get_default_entity_detail().to_some(),
            ),
            "fee_destination:address: must be a valid address",
        );
    }

    #[test]
    fn test_valid_destination() {
        let destination = FeeDestinationV2::new(
            "tp1362ax9s0gxr5yy636q2p9uuefeg8lhguvu6np5",
            Uint128::new(100),
        );
        assert!(
            validate_destination_internal(&destination).is_empty(),
            "a valid fee destination should pass validation and return no error messages",
        );
    }

    #[test]
    fn test_invalid_destination_address() {
        test_invalid_destination(
            &FeeDestinationV2::new("", Uint128::new(1)),
            "fee_destination:address: must be a valid address",
        );
    }

    #[test]
    fn test_invalid_destination_fee_amount_too_low() {
        test_invalid_destination(
            &FeeDestinationV2::new("good-address", Uint128::zero()),
            "fee_destination:fee_amount: must not be zero",
        );
    }

    #[test]
    fn test_validate_asset_definition_input_internal_bad_scope_spec_identifier() {
        let error_strings = validate_asset_definition_input_internal(&AssetDefinitionInputV2::new(
            "heloc",
            ScopeSpecIdentifier::uuid("not even a real uuid at all").to_serialized_enum(),
            vec![],
            None,
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

    fn test_invalid_asset_definition(definition: &AssetDefinitionV2, expected_message: &str) {
        let results = validate_asset_definition_internal(&definition);
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }

    fn test_invalid_verifier(verifier: &VerifierDetailV2, expected_message: &str) {
        let results = validate_verifier_internal(&verifier);
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }

    fn test_invalid_destination(destination: &FeeDestinationV2, expected_message: &str) {
        let results = validate_destination_internal(&destination);
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }
}
