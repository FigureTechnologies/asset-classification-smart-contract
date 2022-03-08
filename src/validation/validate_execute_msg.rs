use crate::core::asset::ValidatorDetail;
use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::util::aliases::{ContractResult, DepsC};
use crate::util::traits::ResultExtensions;
use crate::validation::validate_init_msg::{
    validate_asset_definition, validate_validator_with_provided_errors,
};

pub fn validate_execute_msg(msg: &ExecuteMsg, deps: &DepsC) -> Result<(), ContractError> {
    match msg {
        ExecuteMsg::OnboardAsset {
            asset_uuid,
            asset_type,
            scope_address,
            validator_address,
        } => validate_onboard_asset(asset_uuid, asset_type, scope_address, validator_address),
        ExecuteMsg::ValidateAsset { asset_uuid, .. } => validate_validate_asset(asset_uuid),
        ExecuteMsg::AddAssetDefinition { asset_definition } => {
            validate_asset_definition(&asset_definition.into(), deps)
        }
        ExecuteMsg::UpdateAssetDefinition { asset_definition } => {
            validate_asset_definition(&asset_definition.into(), deps)
        }
        ExecuteMsg::ToggleAssetDefinition { asset_type, .. } => {
            validate_toggle_asset_definition(asset_type)
        }
        ExecuteMsg::AddAssetValidator {
            asset_type,
            validator,
        } => validate_asset_validator_msg(asset_type, validator, deps),
        ExecuteMsg::UpdateAssetValidator {
            asset_type,
            validator,
        } => validate_asset_validator_msg(asset_type, validator, deps),
    }
}

fn validate_onboard_asset(
    asset_uuid: &Option<String>,
    asset_type: &str,
    scope_address: &Option<String>,
    validator_address: &str,
) -> ContractResult<()> {
    let mut invalid_fields: Vec<String> = vec![];
    if asset_uuid.is_none() && scope_address.is_none() {
        invalid_fields
            .push("asset_uuid: must not be blank if scope_address not provided".to_string());
        invalid_fields
            .push("scope_address: must not be blank if asset_uuid not provided".to_string());
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

fn validate_validate_asset(asset_uuid: &str) -> ContractResult<()> {
    let mut invalid_fields: Vec<String> = vec![];
    if asset_uuid.is_empty() {
        invalid_fields.push("asset_uuid: must not be blank".to_string());
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
    deps: &DepsC,
) -> ContractResult<()> {
    let errors = if asset_type.is_empty() {
        Some(vec!["asset_type must not be empty".to_string()])
    } else {
        None
    };
    validate_validator_with_provided_errors(validator, deps, errors)
}

#[cfg(test)]
mod tests {
    use crate::{core::error::ContractError, util::aliases::ContractResult};

    use super::{
        validate_onboard_asset, validate_toggle_asset_definition, validate_validate_asset,
    };

    #[test]
    fn test_validate_onboard_asset_success() {
        validate_onboard_asset(
            &Some("asset_uuid".to_string()),
            "asset_type",
            &Some("scope_address".to_string()),
            "validator_address",
        )
        .expect("expected validation to pass when all arguments are properly supplied");
    }

    #[test]
    fn test_validate_onboard_asset_invalid_asset_uuid_and_scope_address() {
        let result = validate_onboard_asset(&None, "asset_type", &None, "validator_address");
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::OnboardAsset",
                message_type.as_str(),
                "incorrect message type for error"
            );
            assert_eq!(
                2,
                invalid_fields.len(),
                "two invalid fields should be returned"
            );
            invalid_fields
                .iter()
                .find(|f| {
                    f.as_str() == "asset_uuid: must not be blank if scope_address not provided"
                })
                .expect("asset_uuid error should be included in the response");
            invalid_fields
                .iter()
                .find(|f| {
                    f.as_str() == "scope_address: must not be blank if asset_uuid not provided"
                })
                .expect("scope_address error should be included in the response");
        });
    }

    #[test]
    fn test_validate_onboard_asset_invalid_asset_type() {
        let result = validate_onboard_asset(
            &Some("asset_uuid".to_string()),
            "",
            &Some("scope_address".to_string()),
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
        let result = validate_onboard_asset(
            &Some("asset_uuid".to_string()),
            "asset_type",
            &Some("scope_address".to_string()),
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
                "validator_address: must not be blank",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned",
            );
        });
    }

    #[test]
    fn test_validate_validate_asset_success() {
        validate_validate_asset("asset_uuid")
            .expect("expected the validation to pass when all fields are correctly supplied");
    }

    #[test]
    fn test_validate_validate_asset_invalid_asset_uuid() {
        let result = validate_validate_asset("");
        test_invalid_message_fields(result, |message_type, invalid_fields| {
            assert_eq!(
                "ExecuteMsg::ValidateAsset",
                message_type.as_str(),
                "incorrect message type for error"
            );
            assert_eq!(
                1,
                invalid_fields.len(),
                "expected only a single invalid field to be found"
            );
            assert_eq!(
                "asset_uuid: must not be blank",
                invalid_fields.first().unwrap().as_str(),
                "expected the appropriate error message to be returned"
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
                _ => panic!("unexpected error type encountered"),
            },
        }
    }
}
