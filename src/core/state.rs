use crate::core::msg::InitMsg;
use cosmwasm_std::{Addr, Decimal, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static STATE_KEY: &[u8] = b"state";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub contract_name: String,
    pub onboarding_cost: Uint128,
    pub fee_collection_address: Addr,
    pub fee_percent: Decimal,
}
impl State {
    pub fn for_init_msg(msg: InitMsg) -> State {
        State {
            contract_name: msg.contract_name,
            onboarding_cost: msg.onboarding_cost,
            fee_collection_address: Addr::unchecked(msg.fee_collection_address.as_str()),
            fee_percent: msg.fee_percent,
        }
    }
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, STATE_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, STATE_KEY)
}
