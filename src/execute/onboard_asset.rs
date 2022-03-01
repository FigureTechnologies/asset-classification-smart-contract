use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Env, MessageInfo};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OnboardAssetV1 {
    pub asset_uuid: String,
    pub asset_type: String,
    pub scope_address: String,
    pub validator_address: String,
}
impl OnboardAssetV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<OnboardAssetV1> {
        match msg {
            ExecuteMsg::OnboardAsset {
                asset_uuid,
                asset_type,
                scope_address,
                validator_address,
            } => OnboardAssetV1 {
                asset_uuid,
                asset_type,
                scope_address,
                validator_address,
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::OnboardAsset".to_string(),
            }
            .to_err(),
        }
    }
}
impl ResultExtensions for OnboardAssetV1 {}

pub fn onboard_asset(
    _deps: DepsMutC,
    _env: Env,
    _info: MessageInfo,
    _msg: OnboardAssetV1,
) -> ContractResponse {
    ContractError::Unimplemented.to_err()
}
