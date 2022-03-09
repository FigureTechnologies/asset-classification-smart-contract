use cosmwasm_std::{
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Addr, Coin, Decimal, Env, MessageInfo, OwnedDeps, Response, Uint128,
};
use provwasm_mocks::ProvenanceMockQuerier;
use provwasm_std::{Party, PartyType, ProvenanceMsg, ProvenanceQuery, Scope};
use serde_json_wasm::to_string;

use crate::{
    contract::instantiate,
    core::{
        asset::{AssetDefinition, AssetOnboardingStatus, ValidatorDetail},
        msg::InitMsg,
    },
    util::{
        asset_meta_repository::AttributeOnlyAssetMeta, functions::generate_asset_attribute_name,
    },
};
use crate::{
    core::asset::AssetScopeAttribute,
    util::aliases::{ContractResponse, DepsMutC},
};
use crate::{core::msg::AssetDefinitionInput, util::constants::NHASH};

pub type MockOwnedDeps = OwnedDeps<MockStorage, MockApi, ProvenanceMockQuerier, ProvenanceQuery>;

pub const DEFAULT_INFO_NAME: &str = "admin";
pub const DEFAULT_ASSET_TYPE: &str = "test_asset";
pub const DEFAULT_SCOPE_ADDRESS: &str = "scope1234";
pub const DEFAULT_SCOPE_SPEC_ADDRESS: &str = "scopespecaddress";
pub const DEFAULT_VALIDATOR_ADDRESS: &str = "validatoraddress";
pub const DEFAULT_ONBOARDING_COST: u128 = 1000;
pub const DEFAULT_ONBOARDING_DENOM: &str = NHASH;
pub const DEFAULT_FEE_PERCENT: u64 = 0;
pub const DEFAULT_CONTRACT_BASE_NAME: &str = "asset";
pub fn get_default_asset_definition_input() -> AssetDefinitionInput {
    AssetDefinitionInput {
        asset_type: DEFAULT_ASSET_TYPE.into(),
        scope_spec_address: DEFAULT_SCOPE_SPEC_ADDRESS.into(),
        validators: vec![get_default_validator_detail()],
        // Specifying None will cause the underlying code to always choose enabled: true
        enabled: None,
    }
}

pub fn get_default_validator_detail() -> ValidatorDetail {
    ValidatorDetail {
        address: DEFAULT_VALIDATOR_ADDRESS.into(),
        onboarding_cost: Uint128::from(DEFAULT_ONBOARDING_COST),
        onboarding_denom: DEFAULT_ONBOARDING_DENOM.into(),
        fee_percent: Decimal::percent(DEFAULT_FEE_PERCENT),
        fee_destinations: vec![],
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

pub fn get_default_asset_scope_attribute() -> AssetScopeAttribute {
    AssetScopeAttribute {
        asset_type: DEFAULT_ASSET_TYPE.to_string(),
        requestor_address: Addr::unchecked(DEFAULT_INFO_NAME.to_string()),
        validator_address: Addr::unchecked(DEFAULT_VALIDATOR_ADDRESS.to_string()),
        onboarding_status: AssetOnboardingStatus::Pending,
        latest_validator_detail: Some(get_default_validator_detail()),
        latest_validation_result: None,
        access_routes: vec![], // todo: add access_routes schtuff
    }
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

pub fn setup_test_suite(deps: &mut MockOwnedDeps, args: InstArgs) -> AttributeOnlyAssetMeta {
    test_instantiate_success(deps.as_mut(), args);
    mock_default_scope(deps);
    AttributeOnlyAssetMeta::new()
}

pub fn test_instantiate_success(deps: DepsMutC, args: InstArgs) -> Response<ProvenanceMsg> {
    test_instantiate(deps, args).expect("expected instantiation to succeed")
}

pub fn empty_mock_info() -> MessageInfo {
    mock_info(DEFAULT_INFO_NAME, &[])
}

pub fn mock_default_scope(deps: &mut MockOwnedDeps) {
    mock_scope(
        deps,
        DEFAULT_SCOPE_ADDRESS,
        DEFAULT_SCOPE_SPEC_ADDRESS,
        DEFAULT_INFO_NAME,
    )
}

pub fn mock_default_scope_attribute(
    deps: &mut MockOwnedDeps,
    scope_address: impl Into<String>,
    attribute: &AssetScopeAttribute,
) {
    mock_scope_attribute(deps, attribute, scope_address);
}

pub fn mock_info_with_funds(funds: &[Coin]) -> MessageInfo {
    mock_info(DEFAULT_INFO_NAME, funds)
}

pub fn mock_info_with_nhash(amount: u128) -> MessageInfo {
    mock_info_with_funds(&[Coin {
        denom: DEFAULT_ONBOARDING_DENOM.into(),
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

pub fn get_duped_scope<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
    scope_id: S1,
    spec_id: S2,
    owner_address: S3,
) -> Scope {
    let owner_address = owner_address.into();
    Scope {
        scope_id: scope_id.into(),
        specification_id: spec_id.into(),
        owners: vec![Party {
            address: Addr::unchecked(&owner_address),
            role: PartyType::Owner,
        }],
        data_access: vec![],
        value_owner_address: Addr::unchecked(owner_address),
    }
}

pub fn mock_scope<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
    deps: &mut MockOwnedDeps,
    scope_id: S1,
    spec_id: S2,
    owner_address: S3,
) {
    deps.querier
        .with_scope(get_duped_scope(scope_id, spec_id, owner_address))
}

pub fn mock_scope_attribute<S: Into<String>>(
    deps: &mut MockOwnedDeps,
    attribute: &AssetScopeAttribute,
    scope_address: S,
) {
    let address: String = scope_address.into();
    deps.querier.with_attributes(
        &address,
        &[(
            &generate_asset_attribute_name(&attribute.asset_type, DEFAULT_CONTRACT_BASE_NAME),
            &to_string(attribute).expect("failed to convert AssetScopeAttribute to json string"),
            "json",
        )],
    );
}
