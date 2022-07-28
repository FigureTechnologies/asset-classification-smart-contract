use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::types::asset_identifier::AssetIdentifier;
use crate::core::types::asset_qualifier::AssetQualifier;
use crate::core::types::serialized_enum::SerializedEnum;
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::util::aliases::AssetResult;
use crate::util::traits::{OptionExtensions, ResultExtensions};
use crate::validation::validate_init_msg::{
    validate_asset_definition, validate_verifier_with_provided_errors,
};

/// The main branch of validation for an execute msg.  Funnels the intercepted value based on variant
/// to one of the various sub-functions in this module.
///
/// # Parameters
///
/// * `msg` An execute msg to process.
pub fn validate_execute_msg(msg: &ExecuteMsg) -> AssetResult<()> {
    match msg {
        ExecuteMsg::OnboardAsset {
            identifier,
            asset_type,
            verifier_address,
            ..
        } => validate_onboard_asset(identifier, asset_type, verifier_address),
        ExecuteMsg::VerifyAsset { identifier, .. } => validate_verify_asset(identifier),
        ExecuteMsg::FinalizeClassification { identifier } => {
            validate_finalize_classification(identifier)
        }
        ExecuteMsg::AddAssetDefinition { asset_definition } => {
            validate_asset_definition(&asset_definition.as_asset_definition()?)
        }
        ExecuteMsg::UpdateAssetDefinition { asset_definition } => {
            validate_asset_definition(&asset_definition.as_asset_definition()?)
        }
        ExecuteMsg::ToggleAssetDefinition { asset_type, .. } => {
            validate_toggle_asset_definition(asset_type)
        }
        ExecuteMsg::AddAssetVerifier {
            asset_type,
            verifier,
        } => validate_asset_verifier_msg(asset_type, verifier),
        ExecuteMsg::UpdateAssetVerifier {
            asset_type,
            verifier,
        } => validate_asset_verifier_msg(asset_type, verifier),
        ExecuteMsg::UpdateAccessRoutes {
            identifier,
            owner_address,
            ..
        } => validate_update_access_routes(identifier, owner_address),
        ExecuteMsg::DeleteAssetDefinition { qualifier } => {
            validate_delete_asset_definition(qualifier)
        }
    }
}

/// Validates the [OnboardAsset](crate::core::msg::ExecuteMsg::OnboardAsset) variant of the
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).  Returning an empty response on success, or an
/// [InvalidMessageFields](crate::core::error::ContractError::InvalidMessageFields) error when
/// invalid fields are found.
///
/// # Parameters
///
/// * `identifier` An [AssetIdentifier](crate::core::types::asset_identifier::AssetIdentifier)
/// encapsulated within a [SerializedEnum](crate::core::types::serialized_enum::SerializedEnum).
/// * `asset_type` The type of asset to onboard, which should refer to an [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
/// stored internally in the contract.
/// * `verifier_address` The bech32 address of a [VerifierDetailV2](crate::core::types::verifier_detail::VerifierDetailV2)
/// held within the target [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
/// for onboarding.
fn validate_onboard_asset(
    identifier: &SerializedEnum,
    asset_type: &str,
    verifier_address: &str,
) -> AssetResult<()> {
    let mut invalid_fields: Vec<String> = vec![];
    if let Some(message) = get_asset_identifier_invalid_message(identifier) {
        invalid_fields.push(message);
    }
    if asset_type.is_empty() {
        invalid_fields.push("asset_type: must not be blank".to_string());
    }
    if verifier_address.is_empty() {
        invalid_fields.push("verifier_address: must not be blank".to_string());
    }
    gen_validation_response("ExecuteMsg::OnboardAsset", invalid_fields)
}

/// Validates the [VerifyAsset](crate::core::msg::ExecuteMsg::VerifyAsset) variant of the
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).  Returning an empty response on success, or an
/// [InvalidMessageFields](crate::core::error::ContractError::InvalidMessageFields) error when
/// invalid fields are found.
///
/// # Parameters
///
/// * `identifier` An [AssetIdentifier](crate::core::types::asset_identifier::AssetIdentifier)
/// encapsulated within a [SerializedEnum](crate::core::types::serialized_enum::SerializedEnum).
fn validate_verify_asset(identifier: &SerializedEnum) -> AssetResult<()> {
    let mut invalid_fields: Vec<String> = vec![];
    if let Some(message) = get_asset_identifier_invalid_message(identifier) {
        invalid_fields.push(message);
    }
    gen_validation_response("ExecuteMsg::VerifyAsset", invalid_fields)
}

/// Validates the [FinalizeClassification](crate::core::msg::ExecuteMsg::FinalizeClassification)
/// variant of the [ExecuteMsg](crate::core::msg::ExecuteMsg).  Returning an empty response on
/// success, or an [InvalidMessageFields](crate::core::error::ContractError::InvalidMessageFields)
/// error when invalid fields are found.
///
/// # Parameters
///
/// * `identifier` An [AssetIdentifier](crate::core::types::asset_identifier::AssetIdentifier)
/// encapsulated within a [SerializedEnum](crate::core::types::serialized_enum::SerializedEnum).
fn validate_finalize_classification(identifier: &SerializedEnum) -> AssetResult<()> {
    let mut invalid_fields: Vec<String> = vec![];
    if let Some(message) = get_asset_identifier_invalid_message(identifier) {
        invalid_fields.push(message);
    }
    gen_validation_response("ExecuteMsg::FinalizeClassification", invalid_fields)
}

/// Validates the [ToggleAssetDefinition](crate::core::msg::ExecuteMsg::ToggleAssetDefinition) variant of the
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).  Returning an empty response on success, or an
/// [InvalidMessageFields](crate::core::error::ContractError::InvalidMessageFields) error when
/// invalid fields are found.
///
/// # Parameters
///
/// * `asset_type` The type of asset to toggle, which should refer to an [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
/// stored internally in the contract.
fn validate_toggle_asset_definition(asset_type: &str) -> AssetResult<()> {
    let mut invalid_fields: Vec<String> = vec![];
    if asset_type.is_empty() {
        invalid_fields.push("asset_type: must not be blank".to_string());
    }
    gen_validation_response("ExecuteMsg::ToggleAssetDefinition", invalid_fields)
}

/// Validates the [AddAssetVerifier](crate::core::msg::ExecuteMsg::AddAssetVerifier) or [UpdateAssetVerifier](crate::core::msg::ExecuteMsg::UpdateAssetVerifier)
/// variants of the [ExecuteMsg](crate::core::msg::ExecuteMsg).  Returning an empty response on
/// success, or an  [InvalidMessageFields](crate::core::error::ContractError::InvalidMessageFields)
/// error when invalid fields are found.
///
/// # Parameters
///
/// * `asset_type` The type of asset to add or update, which should refer to an [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
/// stored internally in the contract.
/// * `verifier` The verifier detail to add or update.
fn validate_asset_verifier_msg(asset_type: &str, verifier: &VerifierDetailV2) -> AssetResult<()> {
    let errors = if asset_type.is_empty() {
        vec!["asset_type must not be empty".to_string()].to_some()
    } else {
        None
    };
    validate_verifier_with_provided_errors(verifier, errors)
}

/// Validates the [UpdateAccessRoutes](crate::core::msg::ExecuteMsg::UpdateAccessRoutes) variant of the
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).  Returning an empty response on success, or an
/// [InvalidMessageFields](crate::core::error::ContractError::InvalidMessageFields) error when
/// invalid fields are found.
///
/// # Parameters
///
/// * `identifier` An [AssetIdentifier](crate::core::types::asset_identifier::AssetIdentifier)
/// encapsulated within a [SerializedEnum](crate::core::types::serialized_enum::SerializedEnum).
/// * `owner_address` The bech32 address of the account that owns the access routes.
fn validate_update_access_routes(
    identifier: &SerializedEnum,
    owner_address: &str,
) -> AssetResult<()> {
    let mut invalid_fields: Vec<String> = vec![];
    if let Some(message) = get_asset_identifier_invalid_message(identifier) {
        invalid_fields.push(message);
    }
    if owner_address.is_empty() {
        invalid_fields.push("owner_address: must not be blank".to_string());
    }
    gen_validation_response("ExecuteMsg::UpdateAccessRoutes", invalid_fields)
}

/// Validates the [DeleteAssetDefinition](crate::core::msg::ExecuteMsg::DeleteAssetDefinition) variant of the
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).  Returning an empty response on success, or an
/// [InvalidMessageFields](crate::core::error::ContractError::InvalidMessageFields) error when
/// invalid fields are found.
///
/// # Parameters
///
/// * `qualifier` An asset qualifier that can identify the [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
/// to delete.
fn validate_delete_asset_definition(qualifier: &SerializedEnum) -> AssetResult<()> {
    let mut invalid_fields: Vec<String> = vec![];
    if let Some(message) = get_asset_qualifier_invalid_message(qualifier) {
        invalid_fields.push(message);
    }
    gen_validation_response("ExecuteMsg::DeleteAssetDefinition", invalid_fields)
}

/// Validates a serialized enum to ensure that it can convert to a valid [AssetIdentifier](crate::core::types::asset_identifier::AssetIdentifier),
/// returning an optional string that is only populated if an error is present.
///
/// # Parameters
///
/// * `identifier` An [AssetIdentifier](crate::core::types::asset_identifier::AssetIdentifier)
/// encapsulated within a [SerializedEnum](crate::core::types::serialized_enum::SerializedEnum).
fn get_asset_identifier_invalid_message(identifier: &SerializedEnum) -> Option<String> {
    match identifier.to_asset_identifier() {
        Ok(identifier) => match identifier {
            AssetIdentifier::AssetUuid(asset_uuid) => {
                if asset_uuid.is_empty() {
                    "identifier:asset_uuid: must not be blank"
                        .to_string()
                        .to_some()
                } else {
                    None
                }
            }
            AssetIdentifier::ScopeAddress(scope_address) => {
                if scope_address.is_empty() {
                    "identifier:scope_address: must not be blank"
                        .to_string()
                        .to_some()
                } else {
                    None
                }
            }
        },
        Err(e) => match e {
            ContractError::UnexpectedSerializedEnum {
                received_type,
                explanation,
            } => {
                format!("identifier: received type [{received_type}]: {explanation}")
            }
            _ => format!("identifier: received unexpected error message: {e:?}"),
        }
        .to_some(),
    }
}

/// Validates a serialized enum to ensure that it can convert to a valid [AssetQualifier](crate::core::types::asset_qualifier::AssetQualifier),
/// returning an optional string that is only populated if an error is present.
///
/// # Parameters
///
/// * `qualifier` An [AssetQualifier](crate::core::types::asset_qualifier::AssetQualifier)
/// encapsulated within a [SerializedEnum](crate::core::types::serialized_enum::SerializedEnum).
fn get_asset_qualifier_invalid_message(qualifier: &SerializedEnum) -> Option<String> {
    match qualifier.to_asset_qualifier() {
        Ok(asset_qualifier) => match asset_qualifier {
            AssetQualifier::AssetType(asset_type) => {
                if asset_type.is_empty() {
                    "qualifier:asset_type: must not be blank"
                        .to_string()
                        .to_some()
                } else {
                    None
                }
            }
            AssetQualifier::ScopeSpecAddress(scope_spec_address) => {
                if scope_spec_address.is_empty() {
                    "qualifier:scope_spec_address: must not be blank"
                        .to_string()
                        .to_some()
                } else {
                    None
                }
            }
        },
        Err(e) => match e {
            ContractError::UnexpectedSerializedEnum {
                received_type,
                explanation,
            } => {
                format!("qualifier: received type [{received_type}]: {explanation}")
            }
            _ => format!("qualifier: received unexpected error message: {e:?}"),
        }
        .to_some(),
    }
}

/// Takes the invalid fields produced by the various validation functions, and, if they are not
/// empty, returns an [InvalidMessageFields](crate::core::error::ContractError::InvalidMessageFields)
/// error.  If they are empty, returns an empty response.
///
/// # Parameters
///
/// * `message_type` A free-form string that defines the [ExecuteMsg](crate::core::msg::ExecuteMsg)
/// variant that is being validated.
/// * `invalid_fields` A vector containing error messages produced by a validation function.  If
/// empty, this indicates that validation found no issue with input.
fn gen_validation_response<S: Into<String>>(
    message_type: S,
    invalid_fields: Vec<String>,
) -> AssetResult<()> {
    if !invalid_fields.is_empty() {
        ContractError::InvalidMessageFields {
            message_type: message_type.into(),
            invalid_fields,
        }
        .to_err()
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::types::asset_qualifier::AssetQualifier;
    use crate::core::types::serialized_enum::SerializedEnum;
    use crate::validation::validate_execute_msg::{
        validate_delete_asset_definition, validate_finalize_classification,
        validate_update_access_routes,
    };
    use crate::{
        core::{error::ContractError, types::asset_identifier::AssetIdentifier},
        util::aliases::AssetResult,
    };

    use super::{validate_onboard_asset, validate_toggle_asset_definition, validate_verify_asset};

    #[test]
    fn test_validate_onboard_asset_success_for_asset_uuid() {
        validate_onboard_asset(
            &AssetIdentifier::asset_uuid("asset_uuid").to_serialized_enum(),
            "asset_type",
            "verifier_address",
        )
        .expect("expected validation to pass when all arguments are properly supplied");
    }

    #[test]
    fn test_validate_onboard_asset_success_for_scope_address() {
        validate_onboard_asset(
            &AssetIdentifier::scope_address("scope_address").to_serialized_enum(),
            "asset_type",
            "verifier_address",
        )
        .expect("expected validation to pass when all arguments are properly supplied");
    }

    #[test]
    fn test_validate_onboard_asset_invalid_asset_type() {
        let result = validate_onboard_asset(
            &AssetIdentifier::asset_uuid("asset_uuid").to_serialized_enum(),
            "",
            "verifier_address",
        );
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::OnboardAsset",
                message_type.as_str(),
                "incorrect message type for error"
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found"
            );
            assert_eq!(
                "asset_type: must not be blank",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_onboard_asset_invalid_verifier_address() {
        let result = validate_onboard_asset(
            &AssetIdentifier::asset_uuid("asset_uuid").to_serialized_enum(),
            "asset_type",
            "",
        );
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::OnboardAsset",
                message_type.as_str(),
                "incorrect message type for error"
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found"
            );
            assert_eq!(
                "verifier_address: must not be blank",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_onboard_asset_invalid_identifier() {
        let result = validate_onboard_asset(
            &SerializedEnum::new("incorrect_variant", "value"),
            "asset_type",
            "verifier_address",
        );
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::OnboardAsset",
                message_type.as_str(),
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "identifier: received type [incorrect_variant]: Invalid AssetIdentifier. Expected one of [asset_uuid, scope_address]",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_verify_asset_success_for_asset_uuid() {
        validate_verify_asset(
            &AssetIdentifier::asset_uuid("4b9601f4-a0ad-11ec-b214-2f7b0096dea6")
                .to_serialized_enum(),
        )
        .expect("expected the validation to pass when all fields are correctly supplied");
    }

    #[test]
    fn test_validate_verify_asset_success_for_scope_address() {
        validate_verify_asset(
            &AssetIdentifier::scope_address("scope1qps4rfeu5zk3rm9r2gp36dl9r3tq6rpyqd")
                .to_serialized_enum(),
        )
        .expect("expected the validation to pass when all fields are correctly supplied");
    }

    #[test]
    fn test_validate_verify_asset_invalid_asset_uuid() {
        let result = validate_verify_asset(&AssetIdentifier::asset_uuid("").to_serialized_enum());
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::VerifyAsset",
                message_type.as_str(),
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "identifier:asset_uuid: must not be blank",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_verify_asset_invalid_scope_address() {
        let result =
            validate_verify_asset(&AssetIdentifier::scope_address("").to_serialized_enum());
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::VerifyAsset",
                message_type.as_str(),
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "identifier:scope_address: must not be blank",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_verify_asset_invalid_identifier() {
        let result = validate_verify_asset(&SerializedEnum::new("incompatible_variant", "value"));
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::VerifyAsset",
                message_type.as_str(),
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "identifier: received type [incompatible_variant]: Invalid AssetIdentifier. Expected one of [asset_uuid, scope_address]",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_finalize_classification_success_for_asset_uuid() {
        validate_finalize_classification(
            &AssetIdentifier::asset_uuid("8fb0cc16-0eb7-11ed-b200-8fe726e4a262")
                .to_serialized_enum(),
        )
        .expect("expected the validation to pass when all fields are correctly supplied");
    }

    #[test]
    fn test_validate_finalize_classification_success_for_scope_address() {
        validate_finalize_classification(
            &AssetIdentifier::scope_address("scope1qzwvh5y7p6m3rmd4dk9a7legsc5suyx3cu")
                .to_serialized_enum(),
        )
        .expect("expected the validation to pass when all fields are correctly supplied");
    }

    #[test]
    fn test_validate_finalize_classification_invalid_asset_uuid() {
        let result =
            validate_finalize_classification(&AssetIdentifier::asset_uuid("").to_serialized_enum());
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::FinalizeClassification",
                message_type.as_str(),
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "identifier:asset_uuid: must not be blank",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_finalize_classification_invalid_scope_address() {
        let result = validate_finalize_classification(
            &AssetIdentifier::scope_address("").to_serialized_enum(),
        );
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::FinalizeClassification",
                message_type.as_str(),
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "identifier:scope_address: must not be blank",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_finalize_classification_invalid_identifier() {
        let result =
            validate_finalize_classification(&SerializedEnum::new("incompatible_variant", "value"));
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::FinalizeClassification",
                message_type.as_str(),
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "identifier: received type [incompatible_variant]: Invalid AssetIdentifier. Expected one of [asset_uuid, scope_address]",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_toggle_asset_definition_success() {
        validate_toggle_asset_definition("asset_type")
            .expect("expected the validation to pass when all fields are correctly supplied");
    }

    #[test]
    fn test_validate_toggle_asset_definition_invalid_asset_type() {
        let result = validate_toggle_asset_definition("");
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::ToggleAssetDefinition",
                message_type.as_str(),
                "incorrect message type for error"
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found"
            );
            assert_eq!(
                "asset_type: must not be blank",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned"
            );
        });
    }

    #[test]
    fn test_validate_update_access_routes_invalid_identifier_asset_uuid() {
        let result = validate_update_access_routes(
            &AssetIdentifier::asset_uuid("").to_serialized_enum(),
            "owner address",
        );
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::UpdateAccessRoutes", message_type,
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "identifier:asset_uuid: must not be blank",
                invalid_fields.first().unwrap(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_update_access_routes_invalid_identifier_scope_address() {
        let result = validate_update_access_routes(
            &AssetIdentifier::scope_address("").to_serialized_enum(),
            "owner address",
        );
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::UpdateAccessRoutes", message_type,
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "identifier:scope_address: must not be blank",
                invalid_fields.first().unwrap(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_update_access_routes_invalid_owner_address() {
        let result = validate_update_access_routes(
            &AssetIdentifier::scope_address("scope address").to_serialized_enum(),
            "",
        );
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::UpdateAccessRoutes", message_type,
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "owner_address: must not be blank",
                invalid_fields.first().unwrap(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_update_access_routes_invalid_identifier() {
        let result = validate_update_access_routes(
            &SerializedEnum::new("weird_variant", "value"),
            "owner_address",
        );
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::UpdateAccessRoutes",
                message_type.as_str(),
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "identifier: received type [weird_variant]: Invalid AssetIdentifier. Expected one of [asset_uuid, scope_address]",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_delete_asset_definition_successes() {
        validate_delete_asset_definition(&AssetQualifier::asset_type("heloc").to_serialized_enum())
            .expect("expected the validation for asset type qualifier to pass");
        validate_delete_asset_definition(
            &AssetQualifier::scope_spec_address("address").to_serialized_enum(),
        )
        .expect("expected the validation for scope spec address qualifier to pass");
    }

    #[test]
    fn test_validate_delete_asset_definition_invalid_serialized_enum() {
        let result = validate_delete_asset_definition(&SerializedEnum {
            r#type: "invalid_type".to_string(),
            value: "heloc".to_string(),
        });
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::DeleteAssetDefinition", message_type,
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "qualifier: received type [invalid_type]: Invalid AssetQualifier. Expected one of [asset_type, scope_spec_address]",
                invalid_fields.first().unwrap(),
                "expected the appropriate error message to be returned"
            );
        });
    }

    #[test]
    fn test_validate_delete_asset_definition_invalid_asset_type() {
        let result =
            validate_delete_asset_definition(&AssetQualifier::asset_type("").to_serialized_enum());
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::DeleteAssetDefinition", message_type,
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "qualifier:asset_type: must not be blank",
                invalid_fields.first().unwrap(),
                "expected the appropriate error message to be returned"
            );
        });
    }

    #[test]
    fn test_validate_delete_asset_definition_invalid_scope_spec_address() {
        let result = validate_delete_asset_definition(
            &AssetQualifier::scope_spec_address("").to_serialized_enum(),
        );
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::DeleteAssetDefinition", message_type,
                "incorrect message type for error",
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found",
            );
            assert_eq!(
                "qualifier:scope_spec_address: must not be blank",
                invalid_fields.first().unwrap(),
                "expected the appropriate error message to be returned"
            );
        });
    }

    // Extracts the InvalidMessageFunds error data from a response from one of the functions
    // in this file, allowing a unit test to target the relevant information without as much
    // boilerplate nonsense.
    fn test_invalid_message_fields<F>(result: AssetResult<()>, test_func: F)
    where
        F: Fn(String, Vec<String>) -> (),
    {
        match result {
            Ok(_) => panic!("expected the result to be an error"),
            Err(e) => match e {
                ContractError::InvalidMessageFields {
                    message_type,
                    invalid_fields,
                } => test_func(message_type, invalid_fields),
                _ => panic!("unexpected error type encountered: {:?}", e),
            },
        }
    }
}
