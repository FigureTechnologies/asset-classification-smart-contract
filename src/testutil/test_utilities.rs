use cosmwasm_std::{
    testing::{mock_env, mock_info},
    Coin, Decimal, Env, MessageInfo, Response, Uint128,
};
use provwasm_std::ProvenanceMsg;

use crate::util::aliases::{ContractResponse, DepsMutC};
use crate::{
    contract::instantiate,
    core::{
        asset::{AssetDefinition, ValidatorDetail},
        msg::InitMsg,
    },
};
use crate::{core::msg::AssetDefinitionInput, util::constants::NHASH};

pub const DEFAULT_INFO_NAME: &str = "admin";
pub const DEFAULT_ASSET_TYPE: &str = "test_asset";
pub const DEFAULT_VALIDATOR_ADDRESS: &str = "validatoraddress";
pub const DEFAULT_ONBOARDING_COST: u128 = 1000;
pub const DEFAULT_ONBOARDING_DENOM: &str = NHASH;
pub const DEFAULT_FEE_PERCENT: u64 = 0;
pub const DEFAULT_CONTRACT_BASE_NAME: &str = "asset";
pub fn get_default_asset_definition_input() -> AssetDefinitionInput {
    AssetDefinitionInput {
        asset_type: DEFAULT_ASSET_TYPE.into(),
        validators: vec![ValidatorDetail {
            address: DEFAULT_VALIDATOR_ADDRESS.into(),
            onboarding_cost: Uint128::from(DEFAULT_ONBOARDING_COST),
            onboarding_denom: DEFAULT_ONBOARDING_DENOM.into(),
            fee_percent: Decimal::percent(DEFAULT_FEE_PERCENT),
            fee_destinations: vec![],
        }],
        // Specifying None will cause the underlying code to always choose enabled: true
        enabled: None,
    }
}
pub fn get_default_asset_definition() -> AssetDefinition {
    get_default_asset_definition_input().into()
}

pub fn get_default_asset_definition_inputs() -> Vec<AssetDefinitionInput> {
    vec![get_default_asset_definition_input()]
}

pub fn get_default_asset_definitions() -> Vec<AssetDefinition> {
    get_default_asset_definition_inputs()
        .into_iter()
        .map(|input| AssetDefinition::from(input))
        .collect()
}

pub struct InstArgs {
    pub env: Env,
    pub info: MessageInfo,
    pub base_contract_name: String,
    pub asset_definitions: Vec<AssetDefinitionInput>,
}
impl Default for InstArgs {
    fn default() -> Self {
        InstArgs {
            env: mock_env(),
            info: mock_info(DEFAULT_INFO_NAME, &[]),
            base_contract_name: DEFAULT_CONTRACT_BASE_NAME.into(),
            asset_definitions: get_default_asset_definition_inputs(),
        }
    }
}

pub fn test_instantiate(deps: DepsMutC, args: InstArgs) -> ContractResponse {
    instantiate(
        deps,
        args.env,
        args.info,
        InitMsg {
            base_contract_name: args.base_contract_name,
            asset_definitions: args.asset_definitions,
        },
    )
}

pub fn test_instantiate_success(deps: DepsMutC, args: InstArgs) -> Response<ProvenanceMsg> {
    test_instantiate(deps, args).expect("expected instantiation to succeed")
}

pub fn empty_mock_info() -> MessageInfo {
    mock_info(DEFAULT_INFO_NAME, &[])
}

pub fn mock_info_with_funds(funds: &[Coin]) -> MessageInfo {
    mock_info(DEFAULT_INFO_NAME, funds)
}

pub fn mock_info_with_nhash(amount: u128) -> MessageInfo {
    mock_info_with_funds(&[Coin {
        denom: "nhash".into(),
        amount: Uint128::from(amount),
    }])
}

pub fn single_attribute_for_key<'a, T>(response: &'a Response<T>, key: &'a str) -> &'a str {
    response
        .attributes
        .iter()
        .find(|attr| attr.key.as_str() == key)
        .unwrap()
        .value
        .as_str()
}
