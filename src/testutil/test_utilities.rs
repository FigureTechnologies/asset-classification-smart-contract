use cosmwasm_std::{
    coin,
    testing::{message_info, mock_env},
    to_json_vec, Addr, Coin, DepsMut, Env, MessageInfo, OwnedDeps, Response, Uint128,
};
use provwasm_std::types::provenance::{
    attribute::v1::{
        Attribute, AttributeType, MsgAddAttributeRequest, MsgUpdateAttributeRequest,
        QueryAttributeRequest, QueryAttributeResponse, QueryAttributesRequest,
        QueryAttributesResponse,
    },
    metadata::v1::{
        process::ProcessId, record_input::Source, Party, PartyType, Process, Record, RecordInput,
        RecordInputStatus, RecordOutput, RecordWrapper, RecordsRequest, RecordsResponse,
        ResultStatus, Scope, ScopeRequest, ScopeResponse, ScopeWrapper,
    },
};

use crate::core::types::subsequent_classification_detail::SubsequentClassificationDetail;
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::core::{
    error::ContractError,
    types::asset_definition::{AssetDefinitionInputV3, AssetDefinitionV3},
};
use crate::testutil::test_constants::{DEFAULT_RETRY_COST, DEFAULT_SUBSEQUENT_CLASSIFICATION_COST};
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
use crate::{core::types::access_route::AccessRoute, util::aliases::EntryPointResponse};
use crate::{
    core::types::fee_payment_detail::{FeePayment, FeePaymentDetail},
    util::functions::try_into_add_attribute_request,
};
use crate::{
    core::types::onboarding_cost::OnboardingCost,
    util::functions::try_into_update_attribute_request,
};

use super::test_constants::{
    DEFAULT_ACCESS_ROUTE_NAME, DEFAULT_ACCESS_ROUTE_ROUTE, DEFAULT_ADMIN_ADDRESS,
    DEFAULT_ASSET_TYPE, DEFAULT_ASSET_TYPE_DISPLAY_NAME, DEFAULT_ASSET_UUID,
    DEFAULT_CONTRACT_BASE_NAME, DEFAULT_ENTITY_DETAIL_DESCRIPTION, DEFAULT_ENTITY_DETAIL_HOME_URL,
    DEFAULT_ENTITY_DETAIL_NAME, DEFAULT_ENTITY_DETAIL_SOURCE_URL, DEFAULT_ONBOARDING_COST,
    DEFAULT_ONBOARDING_DENOM, DEFAULT_PROCESS_ADDRESS, DEFAULT_PROCESS_METHOD,
    DEFAULT_PROCESS_NAME, DEFAULT_RECORD_INPUT_NAME, DEFAULT_RECORD_INPUT_SOURCE_ADDRESS,
    DEFAULT_RECORD_NAME, DEFAULT_RECORD_OUTPUT_HASH, DEFAULT_RECORD_SPEC_ADDRESS,
    DEFAULT_SCOPE_ADDRESS, DEFAULT_SCOPE_SPEC_ADDRESS, DEFAULT_SENDER_ADDRESS,
    DEFAULT_SESSION_ADDRESS, DEFAULT_VERIFIER_ADDRESS,
};

pub type MockOwnedDeps = OwnedDeps<
    cosmwasm_std::testing::MockStorage,
    cosmwasm_std::testing::MockApi,
    provwasm_mocks::MockProvenanceQuerier,
    cosmwasm_std::Empty,
>;

pub fn get_default_asset_definition_input() -> AssetDefinitionInputV3 {
    AssetDefinitionInputV3 {
        asset_type: DEFAULT_ASSET_TYPE.into(),
        display_name: DEFAULT_ASSET_TYPE_DISPLAY_NAME.map(|n| n.to_string()),
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

pub fn get_default_retry_cost() -> OnboardingCost {
    OnboardingCost::new(DEFAULT_RETRY_COST, &[])
}

pub fn get_default_subsequent_classification_detail() -> SubsequentClassificationDetail {
    SubsequentClassificationDetail::new(
        Some(OnboardingCost::new(
            DEFAULT_SUBSEQUENT_CLASSIFICATION_COST,
            &[],
        )),
        &[DEFAULT_ASSET_TYPE],
    )
}

pub fn get_default_verifier_detail() -> VerifierDetailV2 {
    VerifierDetailV2 {
        address: DEFAULT_VERIFIER_ADDRESS.into(),
        onboarding_cost: Uint128::from(DEFAULT_ONBOARDING_COST),
        onboarding_denom: DEFAULT_ONBOARDING_DENOM.into(),
        fee_destinations: vec![],
        entity_detail: get_default_entity_detail().to_some(),
        retry_cost: get_default_retry_cost().to_some(),
        subsequent_classification_detail: get_default_subsequent_classification_detail().to_some(),
    }
}

pub fn get_default_asset_definition() -> AssetDefinitionV3 {
    get_default_asset_definition_input().into_asset_definition()
}

pub fn get_default_asset_definition_inputs() -> Vec<AssetDefinitionInputV3> {
    vec![get_default_asset_definition_input()]
}

pub fn get_default_asset_definitions() -> Vec<AssetDefinitionV3> {
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
    pub asset_definitions: Vec<AssetDefinitionInputV3>,
}
impl Default for InstArgs {
    fn default() -> Self {
        InstArgs {
            env: mock_env(),
            info: message_info(&Addr::unchecked(DEFAULT_ADMIN_ADDRESS), &[]),
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
                    .map(|asset_type| AssetDefinitionInputV3 {
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

pub fn test_instantiate(deps: DepsMut, args: &InstArgs) -> EntryPointResponse {
    instantiate(
        deps,
        args.env.to_owned(),
        args.info.to_owned(),
        InitMsg {
            base_contract_name: args.base_contract_name.to_owned(),
            bind_base_name: args.bind_base_name,
            asset_definitions: args.asset_definitions.to_owned(),
            is_test: Some(args.is_test),
        },
    )
}

pub fn setup_test_suite(deps: &mut MockOwnedDeps, args: &InstArgs) {
    test_instantiate_success(deps.as_mut(), args);
    let default_scope = get_default_scope();
    ScopeRequest::mock_response(
        &mut deps.querier,
        ScopeResponse {
            scope: Some(ScopeWrapper {
                scope: Some(default_scope.clone()),
                scope_id_info: None,
                scope_spec_id_info: None,
            }),
            sessions: vec![],
            records: vec![],
            request: None,
        },
    );
    RecordsRequest::mock_response(&mut deps.querier, get_default_records());
}

/// Sets up mock queries for no attributes to be returned for a given scope address (defaults to the happy path address).
/// This can be called before [test_onboard_asset](crate::testutil::onboard_asset_helpers::test_onboard_asset)
/// if a successful asset onboarding outcome is desired to ensure that there is no existing conflicting attribute
/// on an asset that would prevent its onboarding. [intercept_add_or_update_attribute] will then update the mock attribute
/// query result after onboarding to mark the asset as onboarded.
pub fn setup_no_attribute_response(deps: &mut MockOwnedDeps, address: Option<String>) {
    QueryAttributeRequest::mock_response(
        &mut deps.querier,
        QueryAttributeResponse {
            account: address
                .to_owned()
                .unwrap_or(DEFAULT_SCOPE_ADDRESS.to_string()),
            attributes: vec![],
            pagination: None,
        },
    );
    QueryAttributesRequest::mock_response(
        &mut deps.querier,
        QueryAttributesResponse {
            account: address.unwrap_or(DEFAULT_SCOPE_ADDRESS.to_string()),
            attributes: vec![],
            pagination: None,
        },
    );
}

pub fn test_instantiate_success(deps: DepsMut, args: &InstArgs) -> Response {
    test_instantiate(deps, args).expect("expected instantiation to succeed")
}

pub fn empty_mock_info<S: Into<String>>(sender: S) -> MessageInfo {
    message_info(&Addr::unchecked(sender.into()), &[])
}

pub fn get_default_scope() -> Scope {
    get_duped_scope(
        DEFAULT_SCOPE_ADDRESS,
        DEFAULT_SCOPE_SPEC_ADDRESS,
        DEFAULT_SENDER_ADDRESS,
    )
}

pub fn get_default_records() -> RecordsResponse {
    get_duped_records(
        None,
        DEFAULT_RECORD_NAME,
        DEFAULT_SESSION_ADDRESS,
        DEFAULT_RECORD_SPEC_ADDRESS,
    )
}

pub fn mock_info_with_funds<S: Into<String>>(sender: S, funds: &[Coin]) -> MessageInfo {
    message_info(&Addr::unchecked(sender.into()), funds)
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
    let owner_address: String = owner_address.into();
    Scope {
        scope_id: scope_id.into().into(),
        specification_id: spec_id.into().into(),
        owners: vec![Party {
            address: owner_address.to_owned(),
            role: PartyType::Owner.into(),
            optional: false,
        }],
        data_access: vec![],
        value_owner_address: owner_address,
        require_party_rollup: false,
    }
}

pub fn get_duped_records<S1, S2, S3>(
    scope: Option<ScopeWrapper>,
    record_name: S1,
    session_address: S2,
    record_spec_address: S3,
) -> RecordsResponse
where
    S1: Into<String>,
    S2: Into<String>,
    S3: Into<String>,
{
    RecordsResponse {
        scope,
        sessions: vec![],
        request: None,
        records: vec![RecordWrapper {
            record: Some(Record {
                name: record_name.into(),
                session_id: session_address.into().into(),
                specification_id: record_spec_address.into().into(),
                process: Some(Process {
                    process_id: Some(ProcessId::Address(DEFAULT_PROCESS_ADDRESS.to_string())),
                    method: DEFAULT_PROCESS_METHOD.to_string(),
                    name: DEFAULT_PROCESS_NAME.to_string(),
                }),
                inputs: vec![RecordInput {
                    name: DEFAULT_RECORD_INPUT_NAME.to_string(),
                    type_name: "string".to_string(),
                    source: Some(Source::Hash(
                        DEFAULT_RECORD_INPUT_SOURCE_ADDRESS.to_string(),
                    )),
                    status: RecordInputStatus::Record.into(),
                }],
                outputs: vec![RecordOutput {
                    hash: DEFAULT_RECORD_OUTPUT_HASH.to_string(),
                    status: ResultStatus::Pass.into(),
                }],
            }),
            record_id_info: None,
            record_spec_id_info: None,
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
    ScopeRequest::mock_response(
        &mut deps.querier,
        ScopeResponse {
            scope: Some(ScopeWrapper {
                scope: Some(get_duped_scope(scope_id, spec_id, owner_address)),
                scope_id_info: None,
                scope_spec_id_info: None,
            }),
            sessions: vec![],
            records: vec![],
            request: None,
        },
    );
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
    RecordsRequest::mock_response(
        &mut deps.querier,
        get_duped_records(
            Some(ScopeWrapper {
                scope: Some(scope),
                scope_id_info: None,
                scope_spec_id_info: None,
            }),
            record_name,
            session_address,
            scope_spec_address,
        ),
    );
}

pub fn build_attribute(address: impl Into<String>, attribute: &AssetScopeAttribute) -> Attribute {
    Attribute {
        name: generate_asset_attribute_name(&attribute.asset_type, DEFAULT_CONTRACT_BASE_NAME),
        value: to_json_vec(attribute).expect("failed to convert AssetScopeAttribute to bytes"),
        attribute_type: AttributeType::Json.into(),
        address: address.into(),
        expiration_date: None,
    }
}

/// Sets up mock queries such that querying for all attributes on the given scope returns
/// a single supplied attribute.
pub fn mock_single_scope_attribute<S: Into<String>>(
    deps: &mut MockOwnedDeps,
    attribute: &AssetScopeAttribute,
    scope_address: S,
) {
    let address: String = scope_address.into();
    QueryAttributesRequest::mock_response(
        &mut deps.querier,
        QueryAttributesResponse {
            account: address.to_owned(),
            attributes: vec![build_attribute(address, attribute)],
            pagination: None,
        },
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

struct AddOrUpdateAttributeParams {
    address: Addr,
    name: String,
    value: Vec<u8>,
}

/// Crawls the vector of messages contained in the provided response, and, if an add attribute message
/// is contained therein, will set the attribute in the MockOwnedDeps' ProvenanceMockQuerier, which
/// will cause downstream consumers in the rest of the test structure to see that attribute as the latest
/// value for the given address.
pub fn intercept_add_or_update_attribute<S: Into<String>>(
    deps: &mut MockOwnedDeps,
    response: Response,
    failure_description: S,
) -> EntryPointResponse {
    let failure_msg: String = failure_description.into();

    for m in response.messages.iter() {
        let params: Option<AddOrUpdateAttributeParams> = if let Some(MsgAddAttributeRequest {
            account,
            name,
            value,
            ..
        }) =
            try_into_add_attribute_request(&m.msg)
        {
            Some(AddOrUpdateAttributeParams {
                address: Addr::unchecked(account),
                name,
                value,
            })
        } else if let Some(MsgUpdateAttributeRequest {
            account,
            name,
            update_value,
            ..
        }) = try_into_update_attribute_request(&m.msg)
        {
            Some(AddOrUpdateAttributeParams {
                address: Addr::unchecked(account),
                name,
                value: update_value,
            })
        } else {
            None
        };
        if let Some(AddOrUpdateAttributeParams {
            address,
            name,
            value,
        }) = params
        {
            // inject bound name into provmock querier
            QueryAttributeRequest::mock_response(
                &mut deps.querier,
                QueryAttributeResponse {
                    account: address.to_string(),
                    attributes: vec![Attribute {
                        name: name.to_string(),
                        value,
                        attribute_type: AttributeType::Json.into(),
                        address: address.to_string(),
                        expiration_date: None,
                    }],
                    pagination: None,
                },
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
