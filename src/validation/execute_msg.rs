use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::util::traits::ResultExtensions;

pub fn validate_execute_msg(msg: &ExecuteMsg) -> Result<(), ContractError> {
    match msg {
        ExecuteMsg::OnboardAsset { scope_address } => validate_onboard_asset(scope_address),
    }
}

fn validate_onboard_asset(scope_address: &str) -> Result<(), ContractError> {
    let mut invalid_fields: Vec<String> = vec![];
    if scope_address.is_empty() {
        invalid_fields.push("scope_address: must not be blank".to_string());
    }
    if !invalid_fields.is_empty() {
        ContractError::InvalidMessageFields { message_type: "ExecuteMsg::OnboardAsset".to_string(), invalid_fields }.to_err()
    } else {
        Ok(())
    }
}
