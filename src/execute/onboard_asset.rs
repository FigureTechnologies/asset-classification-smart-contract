use crate::core::msg::ExecuteMsg;
use crate::core::state::config_read;
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use cosmwasm_std::{CosmosMsg, Env, MessageInfo, Response};
use provwasm_std::ProvenanceMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Type alias to make this less cumbersome
type ProvMsg = CosmosMsg<ProvenanceMsg>;

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
            // TODO:
            // This looks dumb right now because there's only one execution type but there will definitely be others.
            // The default implementation branch here should use the ContractError::InvalidMessageType error
            ExecuteMsg::OnboardAsset {
                asset_uuid,
                asset_type,
                scope_address,
                validator_address,
            } => Ok(OnboardAssetV1 {
                asset_uuid,
                asset_type,
                scope_address,
                validator_address,
            }),
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
    Ok(Response::new())
}

struct FeeChargeDetail {
    fee_charge_message: Option<ProvMsg>,
    fee_refund_message: Option<ProvMsg>,
}

// fn validate_and_get_fee_messages(info: &MessageInfo, state: &State) ->
