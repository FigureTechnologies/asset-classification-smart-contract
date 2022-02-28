use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OnboardAssetV1 {
    pub scope_address: String,
}
impl OnboardAssetV1 {
    fn from_execute_msg(msg: ExecuteMsg) -> Result<OnboardAssetV1, ContractError> {
        match msg {
            // This looks dumb right now because there's only one execution type but there will definitely be others.
            // The default implementation branch here should use the ContractError::InvalidMessageType error
            ExecuteMsg::OnboardAsset { scope_address } => Ok(OnboardAssetV1 { scope_address }),
        }
    }
}

pub fn onboard_asset() {

}
