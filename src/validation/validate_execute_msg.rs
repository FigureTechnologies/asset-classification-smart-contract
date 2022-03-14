use crate::core::asset::ValidatorDetail;
use crate::core::error::ContractError;
use crate::core::msg::{AssetIdentifier, ExecuteMsg};
use crate::util::aliases::ContractResult;
use crate::util::traits::{OptionExtensions, ResultExtensions};
use crate::validation::validate_init_msg::{
    validate_asset_definition, validate_validator_with_provided_errors,
};

pub fn validate_execute_msg(msg: &ExecuteMsg) -> Result<(), ContractError> {
    match msg {
        ExecuteMsg::OnboardAsset {
            identifier,
            asset_type,
            validator_address,
            ..
        } => validate_onboard_asset(identifier, asset_type, validator_address),
        ExecuteMsg::ValidateAsset { identifier, .. } => validate_validate_asset(identifier),
        ExecuteMsg::AddAssetDefinition { asset_definition } => {
            validate_asset_definition(&asset_definition.into())
        }
        ExecuteMsg::UpdateAssetDefinition { asset_definition } => {
            validate_asset_definition(&asset_definition.into())
        }
        ExecuteMsg::ToggleAssetDefinition { asset_type, .. } => {
            validate_toggle_asset_definition(asset_type)
        }
        ExecuteMsg::AddAssetValidator {
            asset_type,
            validator,
        } => validate_asset_validator_msg(asset_type, validator),
        ExecuteMsg::UpdateAssetValidator {
            asset_type,
            validator,
        } => validate_asset_validator_msg(asset_type, validator),
    }
}

fn validate_onboard_asset(
    identifier: &AssetIdentifier,
    asset_type: &str,
    validator_address: &str,
) -> ContractResult<()> {
    let mut invalid_fields: Vec<String> = vec![];
    match identifier {
        AssetIdentifier::AssetUuid { asset_uuid } => {
            if asset_uuid.is_empty() {
                invalid_fields.push("identifier:asset_uuid: must not be blank".to_string());
            }
        }
        AssetIdentifier::ScopeAddress { scope_address } => {
            if scope_address.is_empty() {
                invalid_fields.push("identifier:scope_address: must not be blank".to_string());
            }
        }
    }
    if asset_type.is_empty() {
        invalid_fields.push("asset_type: must not be blank".to_string());
    }
    if validator_address.is_empty() {
        invalid_fields.push("validator_address: must not be blank".to_string());
    }
    if !invalid_fields.is_empty() {
        ContractError::InvalidMessageFields {
            message_type: "ExecuteMsg::OnboardAsset".to_string(),
            invalid_fields,
        }
        .to_err()
    } else {
        Ok(())
    }
}

fn validate_validate_asset(identifier: &AssetIdentifier) -> ContractResult<()> {
    let mut invalid_fields: Vec<String> = vec![];
    match identifier {
        AssetIdentifier::AssetUuid { asset_uuid } => {
            if asset_uuid.is_empty() {
                invalid_fields.push("identifier:asset_uuid: must not be blank".to_string());
            }
        }
        AssetIdentifier::ScopeAddress { scope_address } => {
            if scope_address.is_empty() {
                invalid_fields.push("identifier:scope_address: must not be blank".to_string());
            }
        }
    }
    if !invalid_fields.is_empty() {
        ContractError::InvalidMessageFields {
            message_type: "ExecuteMsg::ValidateAsset".to_string(),
            invalid_fields,
        }
        .to_err()
    } else {
        Ok(())
    }
}

fn validate_toggle_asset_definition(asset_type: &str) -> ContractResult<()> {
    let mut invalid_fields: Vec<String> = vec![];
    if asset_type.is_empty() {
        invalid_fields.push("asset_type: must not be blank".to_string());
    }
    if !invalid_fields.is_empty() {
        ContractError::InvalidMessageFields {
            message_type: "ExecuteMsg::ToggleAssetDefinition".to_string(),
            invalid_fields,
        }
        .to_err()
    } else {
        Ok(())
    }
}

fn validate_asset_validator_msg(
    asset_type: &str,
    validator: &ValidatorDetail,
) -> ContractResult<()> {
    let errors = if asset_type.is_empty() {
        vec!["asset_type must not be empty".to_string()].to_some()
    } else {
        None
    };
    validate_validator_with_provided_errors(validator, errors)
}

#[cfg(test)]
mod tests {
    use crate::{
        core::{error::ContractError, msg::AssetIdentifier},
        util::aliases::ContractResult,
    };

    use super::{
        validate_onboard_asset, validate_toggle_asset_definition, validate_validate_asset,
    };

    #[test]
    fn test_validate_onboard_asset_success_for_asset_uuid() {
        validate_onboard_asset(
            &AssetIdentifier::asset_uuid("asset_uuid"),
            "asset_type",
            "validator_address",
        )
        .expect("expected validation to pass when all arguments are properly supplied");
    }

    #[test]
    fn test_validate_onboard_asset_success_for_scope_address() {
        validate_onboard_asset(
            &AssetIdentifier::scope_address("scope_address"),
            "asset_type",
            "validator_address",
        )
        .expect("expected validation to pass when all arguments are properly supplied");
    }

    #[test]
    fn test_validate_onboard_asset_invalid_asset_type() {
        let result = validate_onboard_asset(
            &AssetIdentifier::asset_uuid("asset_uuid"),
            "",
            "validator_address",
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
    fn test_validate_onboard_asset_invalid_validator_address() {
        let result =
            validate_onboard_asset(&AssetIdentifier::asset_uuid("asset_uuid"), "asset_type", "");
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
                "validator_address: must not be blank",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_validate_asset_success_for_asset_uuid() {
        validate_validate_asset(&AssetIdentifier::asset_uuid(
            "4b9601f4-a0ad-11ec-b214-2f7b0096dea6",
        ))
        .expect("expected the validation to pass when all fields are correctly supplied");
    }

    #[test]
    fn test_validate_validate_asset_success_for_scope_address() {
        validate_validate_asset(&AssetIdentifier::scope_address(
            "scope1qps4rfeu5zk3rm9r2gp36dl9r3tq6rpyqd",
        ))
        .expect("expected the validation to pass when all fields are correctly supplied");
    }

    #[test]
    fn test_validate_validate_asset_invalid_asset_uuid() {
        let result = validate_validate_asset(&AssetIdentifier::asset_uuid(""));
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::ValidateAsset",
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
    fn test_validate_validate_asset_invalid_scope_address() {
        let result = validate_validate_asset(&AssetIdentifier::scope_address(""));
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::ValidateAsset",
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

    // Extracts the InvalidMessageFunds error data from a response from one of the functions
    // in this file, allowing a unit test to target the relevant information without as much
    // boilerplate nonsense.
    fn test_invalid_message_fields<F>(result: ContractResult<()>, test_func: F)
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
