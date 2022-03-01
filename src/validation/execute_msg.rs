use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::util::traits::ResultExtensions;

pub fn validate_execute_msg(msg: &ExecuteMsg) -> Result<(), ContractError> {
    match msg {
        ExecuteMsg::OnboardAsset {
            asset_uuid,
            asset_type,
            scope_address,
            validator_address,
        } => validate_onboard_asset(asset_uuid, asset_type, scope_address, validator_address),
    }
}

fn validate_onboard_asset(
    asset_uuid: &str,
    asset_type: &str,
    scope_address: &str,
    validator_address: &str,
) -> Result<(), ContractError> {
    let mut invalid_fields: Vec<String> = vec![];
    if asset_uuid.is_empty() {
        invalid_fields.push("asset_uuid: must not be blank".to_string());
    }
    if asset_type.is_empty() {
        invalid_fields.push("asset_type: must not be blank".to_string());
    }
    if scope_address.is_empty() {
        invalid_fields.push("scope_address: must not be blank".to_string());
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
