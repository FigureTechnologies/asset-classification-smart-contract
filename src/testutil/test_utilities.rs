use cosmwasm_std::{
    coin, from_binary,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    Addr, Binary, Coin, CosmosMsg, Env, MessageInfo, OwnedDeps, Response, Uint128,
};
use provwasm_mocks::ProvenanceMockQuerier;
use provwasm_std::{
    AttributeMsgParams, Party, PartyType, Process, ProcessId, ProvenanceMsg, ProvenanceMsgParams,
    ProvenanceQuery, Record, RecordInput, RecordInputSource, RecordInputStatus, RecordOutput,
    Records, ResultStatus, Scope,
};
use serde_json_wasm::to_string;

use crate::core::types::fee_payment_detail::{FeePayment, FeePaymentDetail};
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::core::{
    error::ContractError,
    types::asset_definition::{AssetDefinitionInputV2, AssetDefinitionV2},
};
use crate::util::constants::NHASH;
use crate::{
    contract::instantiate,
    core::{
        msg::InitMsg,
        types::{
            access_definition::{AccessDefinition, AccessDefinitionType},
            asset_onboarding_status::AssetOnboardingStatus,
            asset_scope_attribute::AssetScopeAttribute,
            entity_detail::EntityDetail,
        },
    },
    util::{functions::generate_asset_attribute_name, traits::OptionExtensions},
};
use crate::{
    core::types::access_route::AccessRoute,
    util::aliases::{DepsMutC, EntryPointResponse},
};

use super::test_constants::{
    DEFAULT_ACCESS_ROUTE_NAME, DEFAULT_ACCESS_ROUTE_ROUTE, DEFAULT_ADMIN_ADDRESS,
    DEFAULT_ASSET_TYPE, DEFAULT_ASSET_UUID, DEFAULT_CONTRACT_BASE_NAME,
    DEFAULT_ENTITY_DETAIL_DESCRIPTION, DEFAULT_ENTITY_DETAIL_HOME_URL, DEFAULT_ENTITY_DETAIL_NAME,
    DEFAULT_ENTITY_DETAIL_SOURCE_URL, DEFAULT_ONBOARDING_COST, DEFAULT_ONBOARDING_DENOM,
    DEFAULT_PROCESS_ADDRESS, DEFAULT_PROCESS_METHOD, DEFAULT_PROCESS_NAME,
    DEFAULT_RECORD_INPUT_NAME, DEFAULT_RECORD_INPUT_SOURCE_ADDRESS, DEFAULT_RECORD_NAME,
    DEFAULT_RECORD_OUTPUT_HASH, DEFAULT_RECORD_SPEC_ADDRESS, DEFAULT_SCOPE_ADDRESS,
    DEFAULT_SCOPE_SPEC_ADDRESS, DEFAULT_SENDER_ADDRESS, DEFAULT_SESSION_ADDRESS,
    DEFAULT_VERIFIER_ADDRESS,
};

pub type MockOwnedDeps = OwnedDeps<MockStorage, MockApi, ProvenanceMockQuerier, ProvenanceQuery>;

pub fn get_default_asset_definition_input() -> AssetDefinitionInputV2 {
    AssetDefinitionInputV2 {
        asset_type: DEFAULT_ASSET_TYPE.into(),
        verifiers: vec![get_default_verifier_detail()],
        // Specifying None will cause the underlying code to always choose enabled: true
        enabled: None,
        // Specifying None will cause the underlying code to always choose bind_name: true
        bind_name: None,
    }
}

pub fn get_default_entity_detail() -> EntityDetail {
    EntityDetail::new(
        DEFAULT_ENTITY_DETAIL_NAME,
        DEFAULT_ENTITY_DETAIL_DESCRIPTION,
        DEFAULT_ENTITY_DETAIL_HOME_URL,
        DEFAULT_ENTITY_DETAIL_SOURCE_URL,
    )
}

pub fn get_default_verifier_detail() -> VerifierDetailV2 {
    VerifierDetailV2 {
        address: DEFAULT_VERIFIER_ADDRESS.into(),
        onboarding_cost: Uint128::from(DEFAULT_ONBOARDING_COST),
        onboarding_denom: DEFAULT_ONBOARDING_DENOM.into(),
        fee_destinations: vec![],
        entity_detail: get_default_entity_detail().to_some(),
    }
}

pub fn get_default_asset_definition() -> AssetDefinitionV2 {
    get_default_asset_definition_input().into_asset_definition()
}

pub fn get_default_asset_definition_inputs() -> Vec<AssetDefinitionInputV2> {
    vec![get_default_asset_definition_input()]
}

pub fn get_default_asset_definitions() -> Vec<AssetDefinitionV2> {
    get_default_asset_definition_inputs()
        .into_iter()
        .map(|input| input.into_asset_definition())
        .collect()
}

pub fn get_default_access_route() -> AccessRoute {
    AccessRoute::route_and_name(DEFAULT_ACCESS_ROUTE_ROUTE, DEFAULT_ACCESS_ROUTE_NAME)
}

pub fn get_default_access_routes() -> Vec<AccessRoute> {
    vec![get_default_access_route()]
}

pub fn get_default_asset_scope_attribute() -> AssetScopeAttribute {
    AssetScopeAttribute {
        asset_uuid: DEFAULT_ASSET_UUID.to_string(),
        scope_address: DEFAULT_SCOPE_ADDRESS.to_string(),
        asset_type: DEFAULT_ASSET_TYPE.to_string(),
        requestor_address: Addr::unchecked(DEFAULT_SENDER_ADDRESS.to_string()),
        verifier_address: Addr::unchecked(DEFAULT_VERIFIER_ADDRESS.to_string()),
        onboarding_status: AssetOnboardingStatus::Pending,
        latest_verification_result: None,
        access_definitions: vec![AccessDefinition {
            owner_address: DEFAULT_SENDER_ADDRESS.to_string(),
            access_routes: get_default_access_routes(),
            definition_type: AccessDefinitionType::Requestor,
        }],
    }
}

pub struct InstArgs {
    pub env: Env,
    pub info: MessageInfo,
    pub base_contract_name: String,
    pub bind_base_name: bool,
    pub is_test: bool,
    pub asset_definitions: Vec<AssetDefinitionInputV2>,
}
impl Default for InstArgs {
    fn default() -> Self {
        InstArgs {
            env: mock_env(),
            info: mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            base_contract_name: DEFAULT_CONTRACT_BASE_NAME.into(),
            bind_base_name: true,
            // Although this is literally a test framework, we use this to test real interactions.
            // This value should be set to false by default to ensure all test interactions simulate
            // realistic scenarios
            is_test: false,
            asset_definitions: get_default_asset_definition_inputs(),
        }
    }
}

impl InstArgs {
    pub fn default_with_additional_asset_types(asset_types: Vec<&str>) -> Self {
        let default = Self::default();
        Self {
            asset_definitions: vec![
                default.asset_definitions,
                asset_types
                    .iter()
                    .map(|asset_type| AssetDefinitionInputV2 {
                        asset_type: asset_type.to_string(),
                        ..get_default_asset_definition_input()
                    })
                    .collect(),
            ]
            .concat(),
            ..default
        }
    }
}

pub fn test_instantiate(deps: DepsMutC, args: InstArgs) -> EntryPointResponse {
    instantiate(
        deps,
        args.env,
        args.info,
        InitMsg {
            base_contract_name: args.base_contract_name,
            bind_base_name: args.bind_base_name,
            asset_definitions: args.asset_definitions,
            is_test: Some(args.is_test),
        },
    )
}

pub fn setup_test_suite(deps: &mut MockOwnedDeps, args: InstArgs) {
    test_instantiate_success(deps.as_mut(), args);
    let default_scope = get_default_scope();
    deps.querier.with_scope(default_scope.clone());
    deps.querier
        .with_records(default_scope, get_default_records());
}

pub fn test_instantiate_success(deps: DepsMutC, args: InstArgs) -> Response<ProvenanceMsg> {
    test_instantiate(deps, args).expect("expected instantiation to succeed")
}

pub fn empty_mock_info<S: Into<String>>(sender: S) -> MessageInfo {
    mock_info(&sender.into(), &[])
}

pub fn get_default_scope() -> Scope {
    get_duped_scope(
        DEFAULT_SCOPE_ADDRESS,
        DEFAULT_SCOPE_SPEC_ADDRESS,
        DEFAULT_SENDER_ADDRESS,
    )
}

pub fn get_default_records() -> Records {
    get_duped_records(
        DEFAULT_RECORD_NAME,
        DEFAULT_SESSION_ADDRESS,
        DEFAULT_RECORD_SPEC_ADDRESS,
    )
}

pub fn mock_default_scope_attribute(
    deps: &mut MockOwnedDeps,
    scope_address: impl Into<String>,
    attribute: &AssetScopeAttribute,
) {
    mock_scope_attribute(deps, attribute, scope_address);
}

pub fn mock_info_with_funds<S: Into<String>>(sender: S, funds: &[Coin]) -> MessageInfo {
    mock_info(&sender.into(), funds)
}

pub fn mock_info_with_nhash<S: Into<String>>(sender: S, amount: u128) -> MessageInfo {
    mock_info_with_funds(
        sender,
        &[Coin {
            denom: DEFAULT_ONBOARDING_DENOM.into(),
            amount: Uint128::from(amount),
        }],
    )
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

pub fn get_duped_records<S1, S2, S3>(
    record_name: S1,
    session_address: S2,
    record_spec_address: S3,
) -> Records
where
    S1: Into<String>,
    S2: Into<String>,
    S3: Into<String>,
{
    Records {
        records: vec![Record {
            name: record_name.into(),
            session_id: session_address.into(),
            specification_id: record_spec_address.into(),
            process: Process {
                process_id: ProcessId::Address {
                    address: DEFAULT_PROCESS_ADDRESS.to_string(),
                },
                method: DEFAULT_PROCESS_METHOD.to_string(),
                name: DEFAULT_PROCESS_NAME.to_string(),
            },
            inputs: vec![RecordInput {
                name: DEFAULT_RECORD_INPUT_NAME.to_string(),
                type_name: "string".to_string(),
                source: RecordInputSource::Record {
                    record_id: DEFAULT_RECORD_INPUT_SOURCE_ADDRESS.to_string(),
                },
                status: RecordInputStatus::Record,
            }],
            outputs: vec![RecordOutput {
                hash: DEFAULT_RECORD_OUTPUT_HASH.to_string(),
                status: ResultStatus::Pass,
            }],
        }],
    }
}

pub fn get_duped_fee_payment_detail<S: Into<String>>(scope_address: S) -> FeePaymentDetail {
    FeePaymentDetail {
        scope_address: scope_address.into(),
        payments: vec![
            FeePayment {
                amount: coin(150, NHASH),
                name: "Fee for admin".to_string(),
                recipient: Addr::unchecked(DEFAULT_ADMIN_ADDRESS),
            },
            FeePayment {
                amount: coin(250, NHASH),
                name: "Fee for verifier".to_string(),
                recipient: Addr::unchecked(DEFAULT_VERIFIER_ADDRESS),
            },
        ],
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

pub fn mock_record<S1, S2, S3>(
    deps: &mut MockOwnedDeps,
    scope: Scope,
    record_name: S1,
    session_address: S2,
    scope_spec_address: S3,
) where
    S1: Into<String>,
    S2: Into<String>,
    S3: Into<String>,
{
    deps.querier.with_records(
        scope,
        get_duped_records(record_name, session_address, scope_spec_address),
    )
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

pub fn assert_single_item<T: Clone, S: Into<String>>(slice: &[T], message: S) -> T {
    assert_eq!(1, slice.len(), "{}", message.into());
    slice.first().unwrap().clone()
}

pub fn assert_single_item_by<T: Clone, S: Into<String>, F: FnMut(&&T) -> bool>(
    slice: &[T],
    message: S,
    predicate: F,
) -> T {
    let filtered_slice = slice.into_iter().filter(predicate).collect::<Vec<&T>>();
    assert_eq!(1, filtered_slice.len(), "{}", message.into());
    filtered_slice.first().unwrap().clone().to_owned()
}

struct AddOrUpdateAttributeParams<'a> {
    address: &'a Addr,
    name: &'a String,
    binary: &'a Binary,
}
impl<'a> AddOrUpdateAttributeParams<'a> {
    pub fn new(address: &'a Addr, name: &'a String, binary: &'a Binary) -> Self {
        Self {
            address,
            name,
            binary,
        }
    }
}

/// Crawls the vector of messages contained in the provided response, and, if an add attribute message
/// is contained therein, will set the attribute in the MockOwnedDeps' ProvenanceMockQuerier, which
/// will cause downstream consumers in the rest of the test structure to see that attribute as the latest
/// value for the given address.
pub fn intercept_add_or_update_attribute<S: Into<String>>(
    deps: &mut MockOwnedDeps,
    response: Response<ProvenanceMsg>,
    failure_description: S,
) -> EntryPointResponse {
    let failure_msg: String = failure_description.into();

    for m in response.messages.iter() {
        let params = match &m.msg {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::AddAttribute {
                        address,
                        name,
                        value,
                        ..
                    }),
                ..
            }) => Some(AddOrUpdateAttributeParams::new(address, name, value)),
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::UpdateAttribute {
                        address,
                        name,
                        update_value,
                        ..
                    }),
                ..
            }) => Some(AddOrUpdateAttributeParams::new(address, name, update_value)),
            _ => None,
        };
        if let Some(AddOrUpdateAttributeParams {
            address,
            name,
            binary,
        }) = params
        {
            // inject bound name into provmock querier
            let deserialized = from_binary::<AssetScopeAttribute>(binary).unwrap();
            deps.querier.with_attributes(
                address.as_str(),
                &[(
                    name.as_str(),
                    to_string(&deserialized)
                        .expect("expected the scope attribute to convert to json without error")
                        .as_str(),
                    "json",
                )],
            );
            // After finding the an add or update attribute message, exit to avoid panics
            return Ok(response);
        }
    }
    Err(ContractError::generic(format!(
        "{}: message provided did not contain an add attribute message. Full response: {:?}",
        failure_msg, response
    )))
}
