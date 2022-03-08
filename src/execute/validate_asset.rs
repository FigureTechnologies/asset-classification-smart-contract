use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Env, MessageInfo};

#[derive(Clone, PartialEq)]
pub struct ValidateAssetV1 {
    pub asset_uuid: String,
    pub approve: bool,
}
impl ValidateAssetV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<ValidateAssetV1> {
        match msg {
            ExecuteMsg::ValidateAsset {
                asset_uuid,
                approve,
            } => ValidateAssetV1 {
                asset_uuid,
                approve,
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::ValidateAsset".to_string(),
            }
            .to_err(),
        }
    }
}

pub fn validate_asset(
    _deps: DepsMutC,
    _env: Env,
    _info: MessageInfo,
    _msg: ValidateAssetV1,
) -> ContractResponse {
    ContractError::Unimplemented.to_err()
}
