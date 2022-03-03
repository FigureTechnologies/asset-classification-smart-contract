use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::ValidatorDetail;
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
            validate_asset_definition(asset_definition, deps)
        }
        ExecuteMsg::UpdateAssetDefinition { asset_definition } => {
            validate_asset_definition(asset_definition, deps)
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
