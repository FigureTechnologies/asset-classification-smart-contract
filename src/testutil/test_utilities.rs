use cosmwasm_std::{
    testing::{mock_env, mock_info},
    Coin, Decimal, Env, MessageInfo, Response, Uint128,
};
use provwasm_std::ProvenanceMsg;

use crate::util::aliases::{ContractResponse, DepsMutC};
use crate::{
    contract::instantiate,
    core::{
        msg::InitMsg,
        state::{AssetDefinition, ValidatorDetail},
    },
};

pub const DEFAULT_INFO_NAME: &str = "admin";
pub const DEFAULT_ASSET_TYPE: &str = "test_asset";
pub const DEFAULT_VALIDATOR_ADDRESS: &str = "validatoraddress";
pub const DEFAULT_ONBOARDING_COST: u128 = 1000;
pub const DEFAULT_FEE_PERCENT: u64 = 0;
pub const DEFAULT_CONTRACT_BASE_NAME: &str = "asset";
pub fn get_default_asset_definitions() -> Vec<AssetDefinition> {
    [AssetDefinition {
        asset_type: DEFAULT_ASSET_TYPE.into(),
        validators: [ValidatorDetail {
            address: DEFAULT_VALIDATOR_ADDRESS.into(),
            onboarding_cost: Uint128::from(DEFAULT_ONBOARDING_COST),
            fee_percent: Decimal::percent(DEFAULT_FEE_PERCENT),
            fee_destinations: [].to_vec(),
        }]
        .to_vec(),
    }]
    .to_vec()
}

pub struct InstArgs {
    pub env: Env,
    pub info: MessageInfo,
    pub base_contract_name: String,
    pub asset_definitions: Vec<AssetDefinition>,
}
impl Default for InstArgs {
    fn default() -> Self {
        InstArgs {
            env: mock_env(),
            info: mock_info(DEFAULT_INFO_NAME, &[]),
            base_contract_name: DEFAULT_CONTRACT_BASE_NAME.into(),
            asset_definitions: get_default_asset_definitions(),
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

pub fn mock_info_with_nhash(amount: u128) -> MessageInfo {
    mock_info(
        DEFAULT_INFO_NAME,
        &[Coin {
            denom: "nhash".into(),
            amount: Uint128::from(amount),
        }],
    )
}
