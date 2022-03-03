use cosmwasm_std::MessageInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    core::{error::ContractError, msg::ExecuteMsg},
    util::{
        aliases::{ContractResponse, ContractResult, DepsMutC},
        traits::ResultExtensions,
    },
};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct ToggleAssetDefinitionV1 {
    pub asset_type: String,
}
impl ToggleAssetDefinitionV1 {
    pub fn new(asset_type: String) -> Self {
        ToggleAssetDefinitionV1 { asset_type }
    }

    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<ToggleAssetDefinitionV1> {
        match msg {
            ExecuteMsg::ToggleAssetDefinition { asset_type } => {
                ToggleAssetDefinitionV1::new(asset_type).to_ok()
            }
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::ToggleAssetDefinition".to_string(),
            }
            .to_err(),
        }
    }
}
impl ResultExtensions for ToggleAssetDefinitionV1 {}

pub fn toggle_asset_definition(
    deps: DepsMutC,
    info: MessageInfo,
    msg: ToggleAssetDefinitionV1,
) -> ContractResponse {
    ContractError::Unimplemented.to_err()
}
