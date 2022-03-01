use crate::core::msg::InitMsg;
use cosmwasm_std::{Addr, Decimal, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static STATE_KEY: &[u8] = b"state";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub base_contract_name: String,
}
impl State {
    pub fn for_init_msg(msg: InitMsg) -> State {
        State {
            base_contract_name: msg.base_contract_name,
        }
    }
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, STATE_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, STATE_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetDefinition {
    pub asset_type: String,
    pub validators: Vec<ValidatorDetail>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ValidatorDetail {
    pub address: String,
    pub onboarding_cost: Uint128,
    pub fee_percent: Decimal,
    pub fee_destinations: Vec<FeeDestination>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeeDestination {
    pub address: String,
    pub fee_percent: Decimal,
}

pub fn asset_state<S: Into<String>>(
    storage: &mut dyn Storage,
    asset_type: S,
) -> Singleton<AssetDefinition> {
    singleton(storage, &get_asset_state_key(asset_type))
}

pub fn asset_state_read<S: Into<String>>(
    storage: &mut dyn Storage,
    asset_type: S,
) -> ReadonlySingleton<AssetDefinition> {
    singleton_read(storage, &get_asset_state_key(asset_type))
}

fn get_asset_state_key<S: Into<String>>(asset_type: S) -> Vec<u8> {
    format!("{}_{}", asset_type.into(), "asset")
        .as_bytes()
        .to_vec()
}
