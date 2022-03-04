use crate::core::msg::{AssetDefinitionInput, InitMsg};
use cosmwasm_std::{Addr, Decimal, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static STATE_KEY: &[u8] = b"state";
pub static ASSET_META_KEY: &[u8] = b"asset_meta";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub base_contract_name: String,
    pub admin: Addr,
}
impl State {
    pub fn new(msg: InitMsg, admin: Addr) -> State {
        State {
            base_contract_name: msg.base_contract_name,
            admin,
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
    pub enabled: bool,
}
impl AssetDefinition {
    pub fn new(asset_type: String, validators: Vec<ValidatorDetail>) -> Self {
        AssetDefinition {
            asset_type,
            validators,
            enabled: true,
        }
    }
}
impl From<AssetDefinitionInput> for AssetDefinition {
    fn from(input: AssetDefinitionInput) -> Self {
        Self {
            asset_type: input.asset_type,
            validators: input.validators,
            enabled: input.enabled.unwrap_or(true),
        }
    }
}
impl From<&AssetDefinitionInput> for AssetDefinition {
    fn from(input: &AssetDefinitionInput) -> Self {
        AssetDefinition {
            asset_type: input.asset_type.clone(),
            validators: input.validators.clone(),
            enabled: input.enabled.unwrap_or(true),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ValidatorDetail {
    pub address: String,
    pub onboarding_cost: Uint128,
    pub fee_percent: Decimal,
    pub fee_destinations: Vec<FeeDestination>,
}
impl ValidatorDetail {
    pub fn new(
        address: String,
        onboarding_cost: Uint128,
        fee_percent: Decimal,
        fee_destinations: Vec<FeeDestination>,
    ) -> Self {
        ValidatorDetail {
            address,
            onboarding_cost,
            fee_percent,
            fee_destinations,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeeDestination {
    pub address: String,
    pub fee_percent: Decimal,
}
impl FeeDestination {
    pub fn new(address: String, fee_percent: Decimal) -> Self {
        FeeDestination {
            address,
            fee_percent,
        }
    }
}

pub fn asset_state<S: Into<String>>(
    storage: &mut dyn Storage,
    asset_type: S,
) -> Singleton<AssetDefinition> {
    singleton(storage, &get_asset_state_key(asset_type))
}

pub fn asset_state_read<S: Into<String>>(
    storage: &dyn Storage,
    asset_type: S,
) -> ReadonlySingleton<AssetDefinition> {
    singleton_read(storage, &get_asset_state_key(asset_type))
}

fn get_asset_state_key<S: Into<String>>(asset_type: S) -> Vec<u8> {
    format!("{}_{}", asset_type.into(), "asset")
        .as_bytes()
        .to_vec()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetMeta {
    pub scope_address: String,
    pub asset_type: String,
    pub validator_address: String,
    pub onboarding_fee: Uint128,
}
impl AssetMeta {
    pub fn new<T1: Into<String>, T2: Into<String>, T3: Into<String>>(
        scope_address: T1,
        asset_type: T2,
        validator_address: T3,
        onboarding_fee: Uint128,
    ) -> Self {
        AssetMeta {
            scope_address: scope_address.into(),
            asset_type: asset_type.into(),
            validator_address: validator_address.into(),
            onboarding_fee,
        }
    }
}

pub fn asset_meta(storage: &mut dyn Storage) -> Bucket<AssetMeta> {
    bucket(storage, ASSET_META_KEY)
}

pub fn asset_meta_read(storage: &dyn Storage) -> ReadonlyBucket<AssetMeta> {
    bucket_read(storage, ASSET_META_KEY)
}
