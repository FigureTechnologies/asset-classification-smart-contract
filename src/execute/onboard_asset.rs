use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{config_read, State};
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};

// Type alias to make this less cumbersome
type ProvMsg = CosmosMsg<ProvenanceMsg>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OnboardAssetV1 {
    pub scope_address: String,
    pub asset_uuid: String,
    pub oracle_addresses: Vec<String>,
}
impl OnboardAssetV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<OnboardAssetV1> {
        match msg {
            // TODO:
            // This looks dumb right now because there's only one execution type but there will definitely be others.
            // The default implementation branch here should use the ContractError::InvalidMessageType error
            ExecuteMsg::OnboardAsset { scope_address, asset_uuid, oracle_addresses } => Ok(OnboardAssetV1 { scope_address, asset_uuid, oracle_addresses }),
        }
    }
}

pub fn onboard_asset(
    deps: DepsMutC,
    env: Env,
    info: MessageInfo,
    msg: OnboardAssetV1,
) -> ContractResponse {
    let mut messages: Vec<ProvMsg> = vec![];
    let mut attributes: Vec<ProvMsg> = vec![];
    let state = config_read(deps.storage).load()?;
    let fee
}

struct FeeChargeDetail {
    fee_charge_message: Option<ProvMsg>,
    fee_refund_message: Option<ProvMsg>,

}

fn validate_and_get_fee_messages(info: &MessageInfo, state: &State) ->
