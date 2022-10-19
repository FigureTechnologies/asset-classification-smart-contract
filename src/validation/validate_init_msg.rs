use crate::core::error::ContractError;
use crate::core::msg::InitMsg;
use crate::core::types::asset_definition::{AssetDefinitionInputV3, AssetDefinitionV3};
use crate::core::types::fee_destination::FeeDestinationV2;
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::util::aliases::AssetResult;
use crate::util::constants::VALID_VERIFIER_DENOMS;
use crate::util::functions::distinct_count_by_property;
use crate::util::scope_address_utils::bech32_string_to_addr;

use crate::core::types::onboarding_cost::OnboardingCost;
use cosmwasm_std::Uint128;
use result_extensions::ResultExtensions;

/// Validates the integrity of an intercepted [InitMsg](crate::core::msg::InitMsg) and its
/// associated [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3) values.
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
pub fn validate_asset_definition_input(input: &AssetDefinitionInputV3) -> AssetResult<()> {
    validate_asset_definition(&input.as_asset_definition())
}

/// Validates that an asset definition value is properly formed, ensuring that all fields are
/// properly set and fees are established correctly.
///
/// # Parameters
///
/// * `asset_definition` The asset definition value to validate for issues.
pub fn validate_asset_definition(asset_definition: &AssetDefinitionV3) -> AssetResult<()> {
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

fn validate_asset_definition_input_internal(input: &AssetDefinitionInputV3) -> Vec<String> {
    validate_asset_definition_internal(&input.as_asset_definition())
}

fn validate_asset_definition_internal(asset_definition: &AssetDefinitionV3) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if asset_definition.asset_type.is_empty() {
        invalid_fields.push("asset_definition:asset_type: must not be blank".to_string());
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
    if !VALID_VERIFIER_DENOMS.contains(&verifier.onboarding_denom.as_str()) {
        invalid_fields.push(format!(
            "verifier:onboarding_denom: must be one of [{}]",
            VALID_VERIFIER_DENOMS.join(", "),
        ));
    }
    invalid_fields.append(&mut validate_onboarding_cost_internal(
        &verifier.get_default_cost(),
        "verifier onboarding costs",
    ));
    // Ensure that retry costs follow the same standards as the root onboarding cost, if provided.
    // Retry costs are optional and should not cause validation issues when not provided.
    if let Some(ref retry_cost) = verifier.retry_cost {
        invalid_fields.append(&mut validate_onboarding_cost_internal(
            retry_cost,
            "verifier retry costs",
        ));
    }
    // Check subsequent detail values, only if provided.  Omission of subsequent classification detail
    // will ensure that the root onboarding costs are used, so a missing value is completely fine.
    if let Some(ref subsequent_detail) = verifier.subsequent_classification_detail {
        // Ensure that the default cost of the subsequent detail follows the same standards as the
        // root onboarding cost.
        if let Some(ref cost) = subsequent_detail.cost {
            invalid_fields.append(&mut validate_onboarding_cost_internal(
                cost,
                "verifier subsequent classification cost",
            ));
        }
        // Ensure that if allowed asset types are specified, they are not empty and do not contain
        // duplicates.  It is perfectly valid for this field to be a None variant, but if supplied
        // it must be held to standards required downstream by fee payment detail generation.
        if let Some(ref allowed_asset_types) = subsequent_detail.allowed_asset_types {
            if allowed_asset_types.is_empty() {
                invalid_fields.push(
                    "verifier subsequent classification: allowed_asset_types must not be empty if provided"
                        .to_string(),
                );
            }
            if distinct_count_by_property(allowed_asset_types, |t| t) != allowed_asset_types.len() {
                invalid_fields.push("verifier subsequent classification: each value in allowed_asset_types must be unique".to_string());
            }
        }
    }
    invalid_fields
}

fn validate_onboarding_cost_internal<S: Into<String>>(
    onboarding_cost: &OnboardingCost,
    source: S,
) -> Vec<String> {
    let source = source.into();
    let mut invalid_fields: Vec<String> = vec![];
    // onboarding cost must be even, as the Provenance Message Fees module takes half and we need to know how much goes into contract escrow exactly
    if onboarding_cost.cost.u128() % 2 != 0 {
        invalid_fields.push(format!(
            "{}: onboarding_cost:cost must be an even number",
            source
        ));
    }
    if !onboarding_cost.fee_destinations.is_empty()
        && onboarding_cost.get_fee_total() > onboarding_cost.cost.u128() / 2
    {
        invalid_fields.push(format!("{}: onboarding_cost:fee_destinations:fee_amounts must sum to be less than or equal to half the onboarding cost", source));
    }
    if distinct_count_by_property(&onboarding_cost.fee_destinations, |dest| &dest.address)
        != onboarding_cost.fee_destinations.len()
    {
        invalid_fields.push(format!(
            "{}: onboarding_cost:fee_destinations: all fee destinations must have unique addresses",
            source
        ));
    }
    let mut fee_destination_messages = onboarding_cost
        .fee_destinations
        .iter()
        .flat_map(|dest| validate_destination_internal(dest, &source))
        .collect::<Vec<String>>();
    invalid_fields.append(&mut fee_destination_messages);
    invalid_fields
}

fn validate_destination_internal<S: Into<String>>(
    destination: &FeeDestinationV2,
    source: S,
) -> Vec<String> {
    let source = source.into();
    let mut invalid_fields: Vec<String> = vec![];
    if bech32_string_to_addr(&destination.address).is_err() {
        invalid_fields.push(format!(
            "{}: fee_destination:address: must be a valid address",
            source
        ));
    }
    if destination.fee_amount == Uint128::zero() {
        invalid_fields.push(format!(
            "{}: fee_destination:fee_amount: must not be zero",
            source
        ));
    }
    invalid_fields
}

#[cfg(test)]
pub mod tests {
    use crate::core::error::ContractError;
    use crate::core::msg::InitMsg;
    use crate::core::types::asset_definition::{AssetDefinitionInputV3, AssetDefinitionV3};
    use crate::core::types::entity_detail::EntityDetail;
    use crate::core::types::fee_destination::FeeDestinationV2;
    use crate::core::types::onboarding_cost::OnboardingCost;
    use crate::core::types::subsequent_classification_detail::SubsequentClassificationDetail;
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::testutil::test_utilities::get_default_entity_detail;
    use crate::util::constants::{NHASH, VALID_VERIFIER_DENOMS};
    use crate::util::traits::OptionExtensions;
    use crate::validation::validate_init_msg::{
        validate_asset_definition_internal, validate_destination_internal, validate_init_msg,
        validate_verifier_internal,
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
            asset_definitions: vec![AssetDefinitionInputV3::new(
                "heloc",
                "Home Equity Line of Credit".to_some(),
                vec![VerifierDetailV2::new(
                    "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                    Uint128::new(200),
                    NHASH,
                    vec![FeeDestinationV2::new(
                        "tp16e7gwxzr2g5ktfsa69mhy2qqtwxy3g3eansn95",
                        100,
                    )],
                    get_default_entity_detail().to_some(),
                    None,
                    None,
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
                AssetDefinitionInputV3::new(
                    "heloc",
                    "Home Equity Line of Credit".to_some(),
                    vec![VerifierDetailV2::new(
                        "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                        Uint128::new(200),
                        NHASH,
                        vec![FeeDestinationV2::new(
                            "tp16e7gwxzr2g5ktfsa69mhy2qqtwxy3g3eansn95",
                            100,
                        )],
                        get_default_entity_detail().to_some(),
                        None,
                        None,
                    )],
                    None,
                    None,
                ),
                AssetDefinitionInputV3::new(
                    "mortgage",
                    "MORTGAGE".to_some(),
                    vec![VerifierDetailV2::new(
                        "tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89",
                        Uint128::new(500),
                        NHASH,
                        vec![
                            FeeDestinationV2::new("tp1szfeeasdxjdj55sps0m8835wppkykj5wgkhu2p", 125),
                            FeeDestinationV2::new("tp1m2ar35p73amqxwaxgcya0tckd0nmm9l9xe74l7", 125),
                        ],
                        get_default_entity_detail().to_some(),
                        None,
                        None,
                    )],
                    None,
                    None,
                ),
                AssetDefinitionInputV3::new(
                    "pl",
                    "Personal Loan".to_some(),
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
                            None,
                            None,
                        ),
                        VerifierDetailV2::new(
                            "tp1aujf44ge8zydwckk8zwa5g548czys53dkcp2lq",
                            Uint128::new(2000000),
                            NHASH,
                            vec![
                                FeeDestinationV2::new(
                                    "tp1jdcwtaendn9y75jv9dqnmlm7dy8pv4kgu9fs9g",
                                    250000,
                                ),
                                FeeDestinationV2::new(
                                    "tp16dxelgu5nz7u0ygs3qu8tqzjv7gxq5wqucjclm",
                                    750000,
                                ),
                            ],
                            get_default_entity_detail().to_some(),
                            None,
                            None,
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
                asset_definitions: vec![AssetDefinitionInputV3::new(
                    "heloc",
                    "Home Equity Line of Credit".to_some(),
                    vec![VerifierDetailV2::new(
                        "address",
                        Uint128::new(100),
                        NHASH,
                        vec![FeeDestinationV2::new("fee", 100)],
                        get_default_entity_detail().to_some(),
                        None,
                        None,
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
                    AssetDefinitionInputV3::new(
                        "heloc",
                        "Home Equity Line of Credit".to_some(),
                        vec![],
                        None,
                        None,
                    ),
                    AssetDefinitionInputV3::new(
                        "heloc",
                        "Home Equity Line of Credit".to_some(),
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
                asset_definitions: vec![AssetDefinitionInputV3::new(
                    "",
                    None::<String>,
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
        let definition = AssetDefinitionV3::new(
            "heloc",
            "Home Equity Line of Credit".to_some(),
            vec![VerifierDetailV2::new(
                "tp1x24ueqfehs5ye7akkvhf2d67fmfs2zd55tsy2g",
                Uint128::new(200),
                NHASH,
                vec![FeeDestinationV2::new(
                    "tp1pq2yt466fvxrf399atkxrxazptkkmp04x2slew",
                    100,
                )],
                get_default_entity_detail().to_some(),
                None,
                None,
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
            &AssetDefinitionV3::new(
                "",
                None::<String>,
                vec![VerifierDetailV2::new(
                    "address",
                    Uint128::new(100),
                    NHASH,
                    vec![FeeDestinationV2::new("fee", 100)],
                    get_default_entity_detail().to_some(),
                    None,
                    None,
                )],
            ),
            "asset_definition:asset_type: must not be blank",
        );
    }

    #[test]
    fn test_invalid_asset_definition_empty_verifiers() {
        test_invalid_asset_definition(
            &AssetDefinitionV3::new("mortgage", "MORTGAGE".to_some(), vec![]),
            "asset_definition:verifiers: at least one verifier must be supplied per asset type",
        );
    }

    #[test]
    fn test_invalid_asset_definition_picks_up_invalid_verifier_scenarios() {
        test_invalid_asset_definition(
            &AssetDefinitionV3::new(
                "",
                None::<String>,
                vec![VerifierDetailV2::new(
                    "",
                    Uint128::new(100),
                    NHASH,
                    vec![FeeDestinationV2::new("fee", 100)],
                    get_default_entity_detail().to_some(),
                    None,
                    None,
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
            None,
            None,
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
                50,
            )],
            get_default_entity_detail().to_some(),
            None,
            None,
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
            Uint128::new(4000),
            NHASH,
            vec![
                FeeDestinationV2::new("tp14evhfcwnj9hz8p49lysp6uvz6ch3lq8r29xv89", 100),
                FeeDestinationV2::new("tp16e7gwxzr2g5ktfsa69mhy2qqtwxy3g3eansn95", 1650),
                FeeDestinationV2::new("tp1szfeeasdxjdj55sps0m8835wppkykj5wgkhu2p", 50),
                FeeDestinationV2::new("tp1m2ar35p73amqxwaxgcya0tckd0nmm9l9xe74l7", 199),
                FeeDestinationV2::new("tp1aujf44ge8zydwckk8zwa5g548czys53dkcp2lq", 1),
            ],
            get_default_entity_detail().to_some(),
            None,
            None,
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
                None,
                None,
            ),
            "verifier:address: must be a valid address",
        );
    }

    #[test]
    fn test_invalid_verifier_onboarding_denom() {
        let expected_error_text = format!(
            "verifier:onboarding_denom: must be one of [{}]",
            VALID_VERIFIER_DENOMS.join(", ")
        );
        // Verify that a blank value produces an error
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                "",
                vec![],
                get_default_entity_detail().to_some(),
                None,
                None,
            ),
            &expected_error_text,
        );
        // Verify that a value not in VALID_VERIFIER_DENOMS produces an error
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                "someotherdenom",
                vec![],
                get_default_entity_detail().to_some(),
                None,
                None,
            ),
            &expected_error_text,
        )
    }

    #[test]
    fn test_invalid_verifier_fee_amount_too_high() {
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(2020),
                NHASH,
                vec![FeeDestinationV2::new("fee", 1011)],
                get_default_entity_detail().to_some(),
                None,
                None,
            ),
            "verifier onboarding costs: onboarding_cost:fee_destinations:fee_amounts must sum to be less than or equal to half the onboarding cost",
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
                    FeeDestinationV2::new("fee-guy", 25),
                    FeeDestinationV2::new("fee-guy", 25),
                ],
                get_default_entity_detail().to_some(),
                None,
                None,
            ),
            "verifier onboarding costs: onboarding_cost:fee_destinations: all fee destinations must have unique addresses",
        );
    }

    #[test]
    fn test_invalid_verifier_retry_cost_values() {
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                NHASH,
                vec![
                    FeeDestinationV2::new("fee-guy", 25),
                    FeeDestinationV2::new("fee-guy", 25),
                ],
                get_default_entity_detail().to_some(),
                OnboardingCost::new(3, &[]).to_some(),
                None,
            ),
            "verifier retry costs: onboarding_cost:cost must be an even number",
        );
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                NHASH,
                vec![
                    FeeDestinationV2::new("fee-guy", 25),
                    FeeDestinationV2::new("fee-guy", 25),
                ],
                get_default_entity_detail().to_some(),
                OnboardingCost::new(4, &[FeeDestinationV2::new("", 2)]).to_some(),
                None,
            ),
            "verifier retry costs: fee_destination:address: must be a valid address",
        );
    }

    #[test]
    fn test_invalid_verifier_subsequent_classification_detail_cost() {
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                NHASH,
                vec![
                    FeeDestinationV2::new("fee-guy", 25),
                    FeeDestinationV2::new("fee-guy", 25),
                ],
                get_default_entity_detail().to_some(),
                None,
                SubsequentClassificationDetail::new::<String>(
                    OnboardingCost::new(3, &[]).to_some(),
                    &[],
                )
                .to_some(),
            ),
            "verifier subsequent classification cost: onboarding_cost:cost must be an even number",
        );
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                NHASH,
                vec![
                    FeeDestinationV2::new("fee-guy", 25),
                    FeeDestinationV2::new("fee-guy", 25),
                ],
                get_default_entity_detail().to_some(),
                None,
                SubsequentClassificationDetail::new::<String>(
                    OnboardingCost::new(4, &[FeeDestinationV2::new("", 2)]).to_some(),
                    &[],
                )
                .to_some(),
            ),
            "verifier subsequent classification cost: fee_destination:address: must be a valid address",
        );
    }

    #[test]
    fn test_invalid_verifier_subsequent_classification_detail_allowed_asset_types() {
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                NHASH,
                vec![
                    FeeDestinationV2::new("fee-guy", 25),
                    FeeDestinationV2::new("fee-guy", 25),
                ],
                get_default_entity_detail().to_some(),
                None,
                SubsequentClassificationDetail {
                    cost: None,
                    allowed_asset_types: vec![].to_some(),
                }
                .to_some(),
            ),
            "verifier subsequent classification: allowed_asset_types must not be empty if provided",
        );
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                NHASH,
                vec![
                    FeeDestinationV2::new("fee-guy", 25),
                    FeeDestinationV2::new("fee-guy", 25),
                ],
                get_default_entity_detail().to_some(),
                None,
                SubsequentClassificationDetail::new(None, &["dragon", "dragon"]).to_some(),
            ),
            "verifier subsequent classification: each value in allowed_asset_types must be unique",
        );
    }

    #[test]
    fn test_invalid_verifier_picks_up_invalid_fee_destination_scenarios() {
        test_invalid_verifier(
            &VerifierDetailV2::new(
                "address",
                Uint128::new(100),
                NHASH,
                vec![FeeDestinationV2::new("", 100)],
                get_default_entity_detail().to_some(),
                None,
                None,
            ),
            "verifier onboarding costs: fee_destination:address: must be a valid address",
        );
    }

    #[test]
    fn test_valid_destination() {
        let destination = FeeDestinationV2::new("tp1362ax9s0gxr5yy636q2p9uuefeg8lhguvu6np5", 100);
        assert!(
            validate_destination_internal(&destination, "test").is_empty(),
            "a valid fee destination should pass validation and return no error messages",
        );
    }

    #[test]
    fn test_invalid_destination_address() {
        test_invalid_destination(
            &FeeDestinationV2::new("", 1),
            "test: fee_destination:address: must be a valid address",
        );
    }

    #[test]
    fn test_invalid_destination_fee_amount_too_low() {
        test_invalid_destination(
            &FeeDestinationV2::new("good-address", 0),
            "test: fee_destination:fee_amount: must not be zero",
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

    fn test_invalid_asset_definition(definition: &AssetDefinitionV3, expected_message: &str) {
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
        let results = validate_destination_internal(&destination, "test");
        assert!(
            results.contains(&expected_message.to_string()),
            "expected error message `{}` was not contained in the response. Contained messages: {:?}",
            expected_message,
            results,
        );
    }
}
