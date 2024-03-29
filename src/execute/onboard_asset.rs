use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{config_read_v2, load_asset_definition_by_type_v3};
use crate::core::types::access_route::AccessRoute;
use crate::core::types::asset_identifier::AssetIdentifier;
use crate::core::types::asset_onboarding_status::AssetOnboardingStatus;
use crate::core::types::asset_scope_attribute::AssetScopeAttribute;
use crate::service::asset_meta_repository::AssetMetaRepository;
use crate::service::deps_manager::DepsManager;
use crate::service::message_gathering_service::MessageGatheringService;
use crate::util::aliases::{AssetResult, EntryPointResponse};
use crate::util::contract_helpers::check_funds_are_empty;
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::functions::generate_os_gateway_grant_id;
use crate::util::traits::OptionExtensions;
use cosmwasm_std::{Env, MessageInfo, Response};
use os_gateway_contract_attributes::OsGatewayAttributeGenerator;
use provwasm_std::ProvenanceQuerier;
use result_extensions::ResultExtensions;

/// A transformation of [ExecuteMsg::OnboardAsset](crate::core::msg::ExecuteMsg::OnboardAsset)
/// for ease of use in the underlying [onboard_asset](self::onboard_asset) function.
///
/// # Parameters
///
/// * `identifier` An instance of the asset identifier enum that helps the contract identify which
/// scope that the requestor is referring to in the request.
/// * `asset_type` [AssetDefinitionV3's](crate::core::types::asset_definition::AssetDefinitionV3) unique
/// [asset_type](crate::core::types::asset_definition::AssetDefinitionV3::asset_type) value.  This
/// value must correspond to an existing type in the contract's internal storage, or the request
/// for onboarding will be rejected.
/// * `verifier_address` The bech32 Provenance Blockchain [address](crate::core::types::verifier_detail::VerifierDetailV2::address)
/// of a [VerifierDetailV2](crate::core::types::verifier_detail::VerifierDetailV2) on the [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3)
/// referred to by the [asset_type](self::OnboardAssetV1::asset_type) property. If the address does
/// not refer to any existing verifier detail, the request will be rejected.
/// * `access_routes` A vector of access routes to be added to the generated [AssetScopeAttribute's](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
/// [AccessDefinition](crate::core::types::access_definition::AccessDefinition) for the [Requestor](crate::core::types::access_definition::AccessDefinitionType::Requestor)
/// entry.
/// * `add_os_gateway_permission` An optional parameter that will cause the emitted events to
/// include values that signal to any [Object Store Gateway](https://github.com/FigureTechnologies/object-store-gateway)
/// watching the events that the selected verifier has permission to inspect the identified scope's
/// records via fetch routes.  This behavior defaults to TRUE.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnboardAssetV1 {
    pub identifier: AssetIdentifier,
    pub asset_type: String,
    pub verifier_address: String,
    pub access_routes: Vec<AccessRoute>,
    pub add_os_gateway_permission: bool,
}
impl OnboardAssetV1 {
    /// Attempts to create an instance of this struct from a provided execute msg.  If the provided
    /// value is not of the [OnboardAsset](crate::core::msg::ExecuteMsg::OnboardAsset)
    /// variant, then an [InvalidMessageType](crate::core::error::ContractError::InvalidMessageType)
    /// error will be returned.
    ///
    /// # Parameters
    ///
    /// * `msg` An execute msg provided by the contract's [execute](crate::contract::execute) function.
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<OnboardAssetV1> {
        match msg {
            ExecuteMsg::OnboardAsset {
                identifier,
                asset_type,
                verifier_address,
                access_routes,
                add_os_gateway_permission,
            } => OnboardAssetV1 {
                identifier: identifier.to_asset_identifier()?,
                asset_type,
                verifier_address,
                access_routes: access_routes.unwrap_or_default(),
                add_os_gateway_permission: add_os_gateway_permission.unwrap_or(true),
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::OnboardAsset".to_string(),
            }
            .to_err(),
        }
    }
}

/// The function used by [execute](crate::contract::execute) when an [ExecuteMsg::OnboardAsset](crate::core::msg::ExecuteMsg::OnboardAsset)
/// message is provided.  Attempts to verify that a provided Provenance Blockchain Metadata Scope is
/// properly formed on a basic level, and then adds an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
/// to it as a Provenance Blockchain Attribute.
///
/// # Parameters
///
/// * `repository` A helper collection of traits that allows complex lookups of scope values and
/// emits messages to construct the process of onboarding as a collection of messages to produce
/// in the function's result.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` An instance of the onboard asset v1 struct, provided by conversion from an
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).
pub fn onboard_asset<'a, T>(
    repository: T,
    env: Env,
    info: MessageInfo,
    msg: OnboardAssetV1,
) -> EntryPointResponse
where
    T: AssetMetaRepository + MessageGatheringService + DepsManager<'a>,
{
    let asset_identifiers = msg.identifier.to_identifiers()?;
    // get asset definition config for type, or error if not present
    let asset_definition = match repository
        .use_deps(|d| load_asset_definition_by_type_v3(d.storage, &msg.asset_type))
    {
        Ok(state) => {
            if !state.enabled {
                return ContractError::AssetTypeDisabled {
                    asset_type: msg.asset_type,
                }
                .to_err();
            }
            state
        }
        Err(_) => {
            return ContractError::UnsupportedAssetType {
                asset_type: msg.asset_type,
            }
            .to_err()
        }
    };

    // verify prescribed verifier is present as a verifier in asset definition
    let verifier_config = asset_definition.get_verifier_detail(&msg.verifier_address)?;

    // verify no funds are sent, as msg fee handles fees
    check_funds_are_empty(&info)?;

    // verify asset (scope) exists
    let scope = match repository.use_deps(|d| {
        ProvenanceQuerier::new(&d.querier).get_scope(&asset_identifiers.scope_address)
    }) {
        Err(..) => {
            return ContractError::AssetNotFound {
                scope_address: asset_identifiers.scope_address,
            }
            .to_err()
        }
        Ok(scope) => scope,
    };

    let state = repository.use_deps(|deps| config_read_v2(deps.storage).load())?;

    // verify that the sender of this message is a scope owner
    if !scope
        .owners
        .iter()
        .any(|owner| owner.address == info.sender)
    {
        return ContractError::Unauthorized {
            explanation: "sender address does not own the scope".to_string(),
        }
        .to_err();
    }

    // no need to verify records during a test run - this check makes testing the contract a pretty lengthy process
    if !state.is_test {
        // pull scope records for validation - if no records exist on the scope, the querier will produce an error here
        let records = repository
            .use_deps(|d| ProvenanceQuerier::new(&d.querier).get_records(&scope.scope_id))?
            .records;

        // verify scope has at least one record that is not empty
        if !records.into_iter().any(|record| !record.outputs.is_empty()) {
            return ContractError::InvalidScope {
                explanation: format!(
                    "cannot onboard scope [{}]. scope must have at least one non-empty record",
                    scope.scope_id,
                ),
            }
            .to_err();
        }
    }

    let new_asset_attribute = AssetScopeAttribute::new(
        &msg.identifier,
        &msg.asset_type,
        &info.sender,
        &msg.verifier_address,
        AssetOnboardingStatus::Pending.to_some(),
        msg.access_routes,
    )?;

    // check to see if the attribute already exists, and determine if this is a fresh onboard or a subsequent one
    let is_retry = if let Some(existing_attribute) =
        repository.try_get_asset_by_asset_type(&asset_identifiers.scope_address, &msg.asset_type)?
    {
        match existing_attribute.onboarding_status {
            // If the attribute indicates that the asset is approved, then it's already fully onboarded and verified
            AssetOnboardingStatus::Approved => {
                return ContractError::AssetAlreadyOnboarded {
                    scope_address: asset_identifiers.scope_address,
                    asset_type: msg.asset_type,
                }
                .to_err();
            }
            // If the attribute indicates that the asset is pending, then it's currently waiting for verification
            AssetOnboardingStatus::Pending => {
                return ContractError::AssetPendingVerification {
                    scope_address: existing_attribute.scope_address,
                    asset_type: msg.asset_type,
                    verifier_address: existing_attribute.verifier_address.to_string(),
                }
                .to_err()
            }
            // If the attribute indicates that the asset is pending, then it's been denied by a verifier, and this is a secondary
            // attempt to onboard the asset
            AssetOnboardingStatus::Denied => true,
        }
    } else {
        // If no scope attribute exists, it's safe to simply add the attribute to the scope
        false
    };

    // store asset metadata in contract storage, with assigned verifier and provided fee (in case fee changes between onboarding and verification)
    repository.onboard_asset(&env, &new_asset_attribute, &verifier_config, is_retry)?;

    let response = Response::new()
        .add_attributes(
            EventAttributes::for_asset_event(
                EventType::OnboardAsset,
                &msg.asset_type,
                &asset_identifiers.scope_address,
            )
            .set_verifier(&msg.verifier_address)
            .set_scope_owner(info.sender)
            .set_new_asset_onboarding_status(&new_asset_attribute.onboarding_status),
        )
        .add_messages(repository.get_messages());
    let response = if msg.add_os_gateway_permission {
        response.add_attributes(
            OsGatewayAttributeGenerator::access_grant(
                &asset_identifiers.scope_address,
                msg.verifier_address,
            )
            .with_access_grant_id(generate_os_gateway_grant_id(
                msg.asset_type,
                asset_identifiers.scope_address,
            )),
        )
    } else {
        response
    };
    response.to_ok()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, from_binary, CosmosMsg, Response, StdError, Uint128};
    use os_gateway_contract_attributes::{OS_GATEWAY_EVENT_TYPES, OS_GATEWAY_KEYS};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{
        AttributeMsgParams, AttributeValueType, MsgFeesMsgParams, Process, ProcessId,
        ProvenanceMsg, ProvenanceMsgParams, Record, Records,
    };

    use crate::contract::execute;
    use crate::core::msg::ExecuteMsg::OnboardAsset;
    use crate::core::state::{load_asset_definition_by_type_v3, load_fee_payment_detail};
    use crate::core::types::asset_definition::{AssetDefinitionInputV3, AssetDefinitionV3};
    use crate::core::types::fee_destination::FeeDestinationV2;
    use crate::core::types::fee_payment_detail::FeePaymentDetail;
    use crate::core::types::onboarding_cost::OnboardingCost;
    use crate::core::types::subsequent_classification_detail::SubsequentClassificationDetail;
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::execute::add_asset_definition::{add_asset_definition, AddAssetDefinitionV1};
    use crate::execute::add_asset_verifier::{add_asset_verifier, AddAssetVerifierV1};
    use crate::testutil::msg_utilities::{
        test_aggregate_msg_fees_are_charged, test_no_money_moved_in_response,
    };
    use crate::testutil::test_constants::{
        DEFAULT_ONBOARDING_COST, DEFAULT_RETRY_COST, DEFAULT_SECONDARY_ASSET_TYPE,
    };
    use crate::testutil::test_utilities::{
        assert_single_item, get_default_asset_definition_input, get_default_verifier_detail,
        single_attribute_for_key,
    };
    use crate::util::constants::{NEW_ASSET_ONBOARDING_STATUS_KEY, NHASH};
    use crate::util::functions::generate_os_gateway_grant_id;
    use crate::util::traits::OptionExtensions;
    use crate::{
        core::{
            error::ContractError,
            types::{
                access_definition::{AccessDefinition, AccessDefinitionType},
                asset_identifier::AssetIdentifier,
                asset_onboarding_status::AssetOnboardingStatus,
                asset_scope_attribute::AssetScopeAttribute,
            },
        },
        execute::toggle_asset_definition::{toggle_asset_definition, ToggleAssetDefinitionV1},
        service::{
            asset_meta_repository::AssetMetaRepository, asset_meta_service::AssetMetaService,
            message_gathering_service::MessageGatheringService,
        },
        testutil::{
            onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset},
            test_constants::{
                DEFAULT_ADMIN_ADDRESS, DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME,
                DEFAULT_RECORD_SPEC_ADDRESS, DEFAULT_SCOPE_ADDRESS, DEFAULT_SENDER_ADDRESS,
                DEFAULT_SESSION_ADDRESS, DEFAULT_VERIFIER_ADDRESS,
            },
            test_utilities::{
                empty_mock_info, get_default_access_routes, get_default_scope,
                mock_info_with_funds, mock_info_with_nhash, setup_test_suite,
                test_instantiate_success, InstArgs,
            },
            verify_asset_helpers::{test_verify_asset, TestVerifyAsset},
        },
        util::{
            constants::{
                ASSET_EVENT_TYPE_KEY, ASSET_SCOPE_ADDRESS_KEY, ASSET_TYPE_KEY, SCOPE_OWNER_KEY,
                VERIFIER_ADDRESS_KEY,
            },
            functions::generate_asset_attribute_name,
        },
    };

    use super::{onboard_asset, OnboardAssetV1};

    #[test]
    fn test_onboard_asset_errors_on_unsupported_asset_type() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, 1000),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: "bogus".into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.into(),
                access_routes: vec![],
                add_os_gateway_permission: false,
            },
        )
        .unwrap_err();

        match err {
            ContractError::UnsupportedAssetType { asset_type } => {
                assert_eq!(
                    "bogus", asset_type,
                    "the unsupported asset type message should reflect the type provided"
                )
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_disabled_asset_type() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        toggle_asset_definition(
            deps.as_mut(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ToggleAssetDefinitionV1::new(DEFAULT_ASSET_TYPE, false),
        )
        .expect("toggling the asset definition to be disabled should succeed");
        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, 1000),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.into(),
                access_routes: vec![],
                add_os_gateway_permission: false,
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, ContractError::AssetTypeDisabled { .. }),
            "the request should be rejected for a disabled asset type, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_onboard_asset_errors_on_unsupported_verifier() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string() + "bogus".into(),
                access_routes: vec![],
                add_os_gateway_permission: false,
            },
        )
        .unwrap_err();

        match err {
            ContractError::UnsupportedVerifier {
                asset_type,
                verifier_address,
            } => {
                assert_eq!(
                    DEFAULT_ASSET_TYPE, asset_type,
                    "the unsupported verifier message should reflect the asset type provided"
                );
                assert_eq!(
                    DEFAULT_VERIFIER_ADDRESS.to_string() + "bogus".into(),
                    verifier_address,
                    "the unsupported verifier message should reflect the verifier address provided"
                );
            }
            _ => panic!(
                "unexpected error when unsupported verifier provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_funds_provided() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            mock_info_with_funds(DEFAULT_SENDER_ADDRESS, &coins(100, NHASH)),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
                add_os_gateway_permission: false,
            },
        )
        .unwrap_err();

        match err {
            ContractError::InvalidFunds(message) => {
                assert_eq!(
                    "route requires no funds be present", message,
                    "the error should indicate that no funds should be sent when onboarding an asset",
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_asset_not_found() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        // Some random scope address unrelated to the default scope address, which is mocked during setup_test_suite
        let bogus_scope_address = "scope1qp9szrgvvpy5ph5fmxrzs2euyltssfc3lu";

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(bogus_scope_address),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
                add_os_gateway_permission: false,
            },
        )
        .unwrap_err();

        match err {
            ContractError::AssetNotFound { scope_address } => {
                assert_eq!(
                    bogus_scope_address,
                    scope_address.as_str(),
                    "the asset not found message should reflect that the asset uuid was not found"
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_asset_pending_status() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
                add_os_gateway_permission: false,
            },
        )
        .unwrap_err();

        match err {
            ContractError::AssetPendingVerification {
                scope_address,
                asset_type,
                verifier_address,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS,
                    scope_address,
                    "the asset pending verification message should reflect that the asset address is awaiting verification"
                );
                assert_eq!(
                    DEFAULT_ASSET_TYPE,
                    asset_type,
                    "the asset pending verification message should reflect the asset type for which the asset address is awaiting verification"
                );
                assert_eq!(
                    DEFAULT_VERIFIER_ADDRESS,
                    verifier_address,
                    "the asset pending verification message should reflect that the asset is waiting to be verified by the default verifier",
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_asset_approved_status() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_verify_asset(&mut deps, TestVerifyAsset::default()).unwrap();
        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            TestOnboardAsset::default_onboard_asset(),
        )
        .unwrap_err();
        match err {
            ContractError::AssetAlreadyOnboarded {
                scope_address,
                asset_type,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS,
                    scope_address,
                    "the asset already onboarded message should reflect that the asset address was already onboarded",
                );
                assert_eq!(
                    DEFAULT_ASSET_TYPE,
                    asset_type,
                    "the asset already onboarded message should reflect the asswet type for which the asset address was already onboarded",
                );
            }
            _ => panic!(
                "unexpected error encountered when trying to board a verified asset: {:?}",
                err
            ),
        };
    }

    #[test]
    fn test_onboard_asset_errors_on_no_records() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        // Setup the default scope as the result value of a scope query, but don't establish any records
        deps.querier.with_scope(get_default_scope());
        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
                add_os_gateway_permission: false,
            },
        )
        .unwrap_err();
        match err {
            ContractError::Std(e) => match e {
                StdError::GenericErr { msg, .. } => {
                    assert!(
                        msg.contains("Querier system error"),
                        "the message should denote that the querier failed",
                    );
                    assert!(
                        msg.contains("metadata not found"),
                        "the message should denote that the issue was related to metadata",
                    );
                    assert!(
                        msg.contains("get_records"),
                        "the message should denote that the issue was related to records",
                    );
                },
                _ => panic!("unexpected StdError encountered when onboarding a scope with no records: {:?}", e),
            },
            _ => panic!("expected the provenance querier to return an error when no records are present for the scope, but got error: {:?}", err),
        };
    }

    #[test]
    fn test_onboard_asset_succeeds_on_no_records_in_test_mode() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(
            deps.as_mut(),
            InstArgs {
                is_test: true,
                ..Default::default()
            },
        );
        // Setup the default scope as the result value of a scope query, but don't establish any records
        deps.querier.with_scope(get_default_scope());
        onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
                add_os_gateway_permission: false,
            },
        )
        .expect("onboarding should succeed due to test mode being enabled");
    }

    #[test]
    fn test_onboard_asset_errors_on_empty_records() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        // Setup the default scope and add a record, but make sure the record is not formed properly
        let scope = get_default_scope();
        deps.querier.with_scope(scope.clone());
        deps.querier.with_records(
            scope,
            Records {
                records: vec![Record {
                    name: "record-name".to_string(),
                    session_id: DEFAULT_SESSION_ADDRESS.to_string(),
                    specification_id: DEFAULT_RECORD_SPEC_ADDRESS.to_string(),
                    process: Process {
                        process_id: ProcessId::Address {
                            address: String::new(),
                        },
                        method: String::new(),
                        name: String::new(),
                    },
                    inputs: vec![],
                    outputs: vec![],
                }],
            },
        );
        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
                add_os_gateway_permission: false,
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, ContractError::InvalidScope { .. }),
            "expected the error to indicate that the scope was invalid for records, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_onboard_asset_succeeds_for_empty_records_in_test_mode() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(
            deps.as_mut(),
            InstArgs {
                is_test: true,
                ..Default::default()
            },
        );
        // Setup the default scope and add a record, but make sure the record is not formed properly
        let scope = get_default_scope();
        deps.querier.with_scope(scope.clone());
        deps.querier.with_records(
            scope,
            Records {
                records: vec![Record {
                    name: "record-name".to_string(),
                    session_id: DEFAULT_SESSION_ADDRESS.to_string(),
                    specification_id: DEFAULT_RECORD_SPEC_ADDRESS.to_string(),
                    process: Process {
                        process_id: ProcessId::Address {
                            address: String::new(),
                        },
                        method: String::new(),
                        name: String::new(),
                    },
                    inputs: vec![],
                    outputs: vec![],
                }],
            },
        );
        onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
                add_os_gateway_permission: false,
            },
        )
        .expect("onboarding should succeed due to test mode being enabled");
    }

    #[test]
    fn test_onboard_asset_success() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let result = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_env(),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: get_default_access_routes(),
                add_os_gateway_permission: false,
            },
        )
        .unwrap();

        let fee_payment_result = load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        );

        if let Err(_) = fee_payment_result {
            panic!("fee payment detail should be stored for onboarded asset")
        }

        assert_eq!(
            2,
            result.messages.len(),
            "Onboarding should produce the correct number of messages"
        );

        result.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::AddAttribute {
                        name, value, ..
                    }),
                ..
            }) => {
                assert_eq!(
                    &generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME),
                    name,
                    "bound asset name should match what is expected for the asset_type"
                );
                let deserialized: AssetScopeAttribute = from_binary(value).unwrap();
                assert_eq!(
                    DEFAULT_ASSET_TYPE.to_string(),
                    deserialized.asset_type,
                    "Asset type in attribute should match what was provided at onboarding"
                );
                assert_eq!(
                    AssetOnboardingStatus::Pending,
                    deserialized.onboarding_status,
                    "Onboarding status should initially be Pending"
                );
                assert_eq!(
                    1,
                    deserialized.access_definitions.len(),
                    "Provided access route should be set upon onboarding"
                );
                assert_eq!(
                    &AccessDefinition {
                        owner_address: DEFAULT_SENDER_ADDRESS.to_string(),
                        access_routes: get_default_access_routes(),
                        definition_type: AccessDefinitionType::Requestor,
                    },
                    deserialized.access_definitions.first().unwrap(),
                    "Proper access route should be set upon onboarding"
                );
            }
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::MsgFees(MsgFeesMsgParams::AssessCustomFee {
                        amount,
                        name,
                        from,
                        recipient,
                    }),
                ..
            }) => {
                assert_eq!(
                    DEFAULT_ONBOARDING_COST,
                    amount.amount.u128(),
                    "double the default verifier cost should be included in the fee msg to account for the provenance cut",
                );
                assert!(
                    name.is_some(),
                    "the fee message should include a fee name",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    from.as_str(),
                    "the fee message should always be sent from the contract's address",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    recipient.to_owned().expect("a recipient address should be set on the custom fee").as_str(),
                    "the contract's address should be the recipient of the fee",
                );
            }
            msg => panic!("Unexpected message from onboard_asset: {:?}", msg),
        });
        assert_onboard_response_attributes_are_correct(&result, false);
    }

    #[test]
    fn test_onboarding_asset_with_free_onboarding_cost() {
        let mut deps = mock_dependencies(&[]);
        // Set up the contract as normal, but make onboarding free
        setup_test_suite(
            &mut deps,
            InstArgs {
                asset_definitions: vec![AssetDefinitionInputV3 {
                    verifiers: vec![VerifierDetailV2 {
                        onboarding_cost: Uint128::zero(),
                        ..get_default_verifier_detail()
                    }],
                    ..get_default_asset_definition_input()
                }],
                ..InstArgs::default()
            },
        );
        let response = test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_no_money_moved_in_response(
            &response,
            "no funds should be sent when onboarding with a free onboarding cost",
        );
    }

    #[test]
    fn test_onboard_asset_retry_success() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let payment_detail_before_retry = load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect("a fee payment detail should be stored for the asset after onboarding");
        test_verify_asset(&mut deps, TestVerifyAsset::default_with_success(false)).unwrap();
        load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect_err("no fee payment detail should be present after success=false verification");
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect("the default scope address should have an attribute attached to it");
        assert_eq!(
            attribute.onboarding_status,
            AssetOnboardingStatus::Denied,
            "sanity check: the onboarding status should be set to denied after the verifier marks the asset as success = false",
        );
        let response = test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        assert_eq!(
            2,
            response.messages.len(),
            "two messages should be emitted in the retry. One for an attribute update and one for a message fee",
        );
        assert!(
            matches!(
                response.messages[0].msg,
                CosmosMsg::Custom(ProvenanceMsg {
                    params: ProvenanceMsgParams::Attribute(
                        AttributeMsgParams::UpdateAttribute { .. }
                    ),
                    ..
                })
            ),
            "the first emitted message should update the attribute",
        );
        assert!(
            matches!(
                response.messages[1].msg,
                CosmosMsg::Custom(ProvenanceMsg {
                    params: ProvenanceMsgParams::MsgFees(MsgFeesMsgParams::AssessCustomFee { .. }),
                    ..
                })
            ),
            "the second emitted message should be the message fee"
        );
        test_aggregate_msg_fees_are_charged(
            &response,
            DEFAULT_RETRY_COST,
            "the retry amount should be used because the the same verifier was used",
        );
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect("the default scope address should still contain an attribute after onboarding for a second time");
        assert_eq!(
            attribute.onboarding_status,
            AssetOnboardingStatus::Pending,
            "the onboarding status should now be set to pending after retrying the onboard process",
        );
        let payment_detail_after_retry = load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect("a fee payment detail should still be present after updating");
        assert_ne!(
            payment_detail_before_retry, payment_detail_after_retry,
            "the payment details should be different after retrying",
        );
        let default_definition =
            load_asset_definition_by_type_v3(deps.as_ref().storage, DEFAULT_ASSET_TYPE)
                .expect("the default asset type should have an asset definition");
        let default_verifier = assert_single_item(
            &default_definition.verifiers,
            "expected a single verifier to be set on the default asset definition",
        );
        let expected_payment_detail_after_retry = FeePaymentDetail::new(
            DEFAULT_SCOPE_ADDRESS,
            &default_verifier,
            // Proves that this retry used the retry fees in the default verifier
            true,
            DEFAULT_ASSET_TYPE,
            &[attribute],
        )
        .expect("Payment detail should be generated without issue");
        assert_eq!(
            expected_payment_detail_after_retry, payment_detail_after_retry,
            "the payment detail after retry should be generated using the correct retry values",
        );
        assert_eq!(
            default_verifier
                .retry_cost
                .expect("the default verifier should have a retry cost")
                .cost
                .u128(),
            payment_detail_after_retry.sum_costs(),
            "the payments in the generated detail should sum to the defined retry cost divided by two to account for provenance fee handling",
        );
    }

    #[test]
    fn test_onboard_asset_retry_success_changing_verifiers() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        let other_verifier = VerifierDetailV2::new(
            "tp17szfvgwgx9c9kwvyp9megryft3zm77am6x9gal",
            Uint128::new(300),
            NHASH,
            vec![
                FeeDestinationV2::new("feeperson1", 100),
                FeeDestinationV2::new("feeperson2", 50),
            ],
            None,
            // This other verifier has a super hefty retry cost, but this value should not be used
            // because the first failed verification was with a different verifier
            OnboardingCost::new(40000, &[FeeDestinationV2::new("bad_fee", 2000)]).to_some(),
            None,
        );
        add_asset_verifier(
            deps.as_mut(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            AddAssetVerifierV1 {
                asset_type: DEFAULT_ASSET_TYPE.to_string(),
                verifier: other_verifier.clone(),
            },
        )
        .expect("adding the second verifier should succeed without error");
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let payment_detail_before = load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect("a fee payment detail should be stored for the asset after onboarding");
        test_verify_asset(&mut deps, TestVerifyAsset::default_with_success(false)).unwrap();
        load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect_err("no fee payment detail should be present after success=false verification");
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect("the default scope address should have an attribute attached to it");
        assert_eq!(
            attribute.onboarding_status,
            AssetOnboardingStatus::Denied,
            "sanity check: the onboarding status should be set to denied after the verifier marks the asset as success = false",
        );
        let response = test_onboard_asset(
            &mut deps,
            TestOnboardAsset {
                onboard_asset: OnboardAssetV1 {
                    verifier_address: other_verifier.address.clone(),
                    ..TestOnboardAsset::default_onboard_asset()
                },
                ..TestOnboardAsset::default()
            },
        )
        .unwrap();
        assert_eq!(
            2,
            response.messages.len(),
            "two messages should be emitted in the retry. One for an attribute update and one for a message fee",
        );
        assert!(
            matches!(
                response.messages[0].msg,
                CosmosMsg::Custom(ProvenanceMsg {
                    params: ProvenanceMsgParams::Attribute(
                        AttributeMsgParams::UpdateAttribute { .. }
                    ),
                    ..
                })
            ),
            "the first emitted message should update the attribute",
        );
        assert!(
            matches!(
                response.messages[1].msg,
                CosmosMsg::Custom(ProvenanceMsg {
                    params: ProvenanceMsgParams::MsgFees(MsgFeesMsgParams::AssessCustomFee { .. }),
                    ..
                })
            ),
            "the second emitted message should be the message fee"
        );
        test_aggregate_msg_fees_are_charged(
            &response,
            300,
            "the retry amount should not be used because a different verifier was used",
        );
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect("the default scope address should still contain an attribute after onboarding for a second time");
        assert_eq!(
            attribute.onboarding_status,
            AssetOnboardingStatus::Pending,
            "the onboarding status should now be set to pending after retrying the onboard process",
        );
        assert_eq!(
            attribute.verifier_address, other_verifier.address,
            "the attribute should be updated to the other verifier's address",
        );
        let payment_detail_after = load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect("a fee payment detail should be stored for the asset");
        assert_ne!(
            payment_detail_before, payment_detail_after,
            "the payment details should not match due to changing verifiers",
        );
        assert_eq!(
            // Proves that this subsequent retry using a different verifier will not load the
            // retry fees, because retries should only execute when using the same verifier
            FeePaymentDetail::new(DEFAULT_SCOPE_ADDRESS, &other_verifier, false, DEFAULT_ASSET_TYPE, &[])
                .expect("the other verifier should be successfully converted to a fee payment detail"),
            payment_detail_after,
            "the fee payment detail after the retry should equate to the new verifier's fee definitions",
        );
    }

    #[test]
    fn test_onboarding_asset_retry_success_with_free_retries() {
        let mut deps = mock_dependencies(&[]);
        // Set up the contract as normal, but make retries free
        setup_test_suite(
            &mut deps,
            InstArgs {
                asset_definitions: vec![AssetDefinitionInputV3 {
                    verifiers: vec![VerifierDetailV2 {
                        retry_cost: OnboardingCost::new(0, &[]).to_some(),
                        ..get_default_verifier_detail()
                    }],
                    ..get_default_asset_definition_input()
                }],
                ..InstArgs::default()
            },
        );
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_verify_asset(&mut deps, TestVerifyAsset::default_with_success(false)).unwrap();
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect("the default scope address should have an attribute attached to it");
        assert_eq!(
            attribute.onboarding_status,
            AssetOnboardingStatus::Denied,
            "sanity check: the onboarding status should be set to denied after the verifier marks the asset as success = false",
        );
        let response = test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_no_money_moved_in_response(
            &response,
            "no funds should be sent when onboarding as a retry with no retry cost",
        );
    }

    #[test]
    fn test_onboard_asset_as_subsequent_type_uses_subsequent_classification_fees() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        let secondary_verifier = VerifierDetailV2::new(
            DEFAULT_VERIFIER_ADDRESS,
            Uint128::new(300),
            NHASH,
            vec![
                FeeDestinationV2::new("feeperson1", 100),
                FeeDestinationV2::new("feeperson2", 50),
            ],
            None,
            None,
            SubsequentClassificationDetail::new(
                OnboardingCost::new(600, &[]).to_some(),
                &[DEFAULT_ASSET_TYPE],
            )
            .to_some(),
        );
        let secondary_asset_definition = AssetDefinitionV3::new(
            DEFAULT_SECONDARY_ASSET_TYPE,
            Some("secondary asset"),
            vec![secondary_verifier.clone()],
        );
        add_asset_definition(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            AddAssetDefinitionV1 {
                asset_definition: secondary_asset_definition.clone(),
                bind_name: Some(false),
            },
        )
        .expect("adding the secondary asset definition should succeed");
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("onboarding as the default asset type should succeed");
        let existing_scope_attribute = AssetMetaService::new(deps.as_mut())
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect("the existing asset type should have an asset scope attribute");
        let subsequent_response = test_onboard_asset(
            &mut deps,
            TestOnboardAsset {
                onboard_asset: OnboardAssetV1 {
                    asset_type: DEFAULT_SECONDARY_ASSET_TYPE.to_string(),
                    ..TestOnboardAsset::default_onboard_asset()
                },
                ..TestOnboardAsset::default()
            },
        )
        .expect("onboarding the subsequent asset type should succeed");
        test_aggregate_msg_fees_are_charged(
            &subsequent_response,
            600,
            "the subsequent onboarding cost should be used as the msg fee",
        );
        let secondary_payment_detail = load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_SECONDARY_ASSET_TYPE,
        )
        .expect("a fee payment detail should be available for the subsequent asset");
        assert_eq!(
            1,
            secondary_payment_detail.payments.len(),
            "only one payment should be emitted for the secondary payment detail, proving that subsequent detail was used",
        );
        let expected_fee_payment_detail = FeePaymentDetail::new(
            DEFAULT_SCOPE_ADDRESS,
            &secondary_verifier,
            false,
            DEFAULT_SECONDARY_ASSET_TYPE,
            &[existing_scope_attribute],
        )
        .expect("fee payment detail generation using the correct values should succeed");
        assert_eq!(
            expected_fee_payment_detail,
            secondary_payment_detail,
            "the subsequent classification detail should have been used to generate the correct fee",
        );
    }

    #[test]
    fn test_onboard_asset_as_subsequent_non_applicable_type_uses_default_fees() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        let secondary_verifier = VerifierDetailV2::new(
            DEFAULT_VERIFIER_ADDRESS,
            Uint128::new(300),
            NHASH,
            vec![],
            None,
            None,
            SubsequentClassificationDetail::new(
                OnboardingCost::new(
                    600,
                    &[FeeDestinationV2::new("should-not-be-encountered", 10)],
                )
                .to_some(),
                &["some-other-asset-type"],
            )
            .to_some(),
        );
        let secondary_asset_definition = AssetDefinitionV3::new(
            DEFAULT_SECONDARY_ASSET_TYPE,
            Some("secondary asset"),
            vec![secondary_verifier],
        );
        add_asset_definition(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            AddAssetDefinitionV1 {
                asset_definition: secondary_asset_definition.clone(),
                bind_name: Some(false),
            },
        )
        .expect("adding the secondary asset definition should succeed");
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("onboarding as the default asset type should succeed");
        test_onboard_asset(
            &mut deps,
            TestOnboardAsset {
                onboard_asset: OnboardAssetV1 {
                    asset_type: DEFAULT_SECONDARY_ASSET_TYPE.to_string(),
                    ..TestOnboardAsset::default_onboard_asset()
                },
                ..TestOnboardAsset::default()
            },
        )
        .expect("onboarding as a non-applicable subsequent type should succeed");
        let subsequent_onboard_fee_detail = load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_SECONDARY_ASSET_TYPE,
        )
        .expect("a fee detail should be available after onboarding a subsequent asset type");
        assert_eq!(
            1,
            subsequent_onboard_fee_detail.payments.len(),
            "only one fee payment should be generated when defaulting to the original verifier costs",
        );
        let payment_detail = subsequent_onboard_fee_detail.payments.first().unwrap();
        assert_eq!(
            300,
            payment_detail.amount.amount.u128(),
            "the payment amount should be 300, which is the entirety of the onboarding cost",
        );
        assert_eq!(
            DEFAULT_VERIFIER_ADDRESS,
            payment_detail.recipient.as_str(),
            "the payment should be sent to the verifier",
        );
    }

    #[test]
    fn test_onboarding_asset_as_subsequent_type_with_free_subsequent_cost() {
        let mut deps = mock_dependencies(&[]);
        // Set up the contract as normal, but make subsequent onboards free
        setup_test_suite(&mut deps, InstArgs::default());
        let secondary_verifier = VerifierDetailV2::new(
            DEFAULT_VERIFIER_ADDRESS,
            Uint128::new(300),
            NHASH,
            vec![
                FeeDestinationV2::new("feeperson1", 100),
                FeeDestinationV2::new("feeperson2", 50),
            ],
            None,
            None,
            SubsequentClassificationDetail::new(
                OnboardingCost::new(0, &[]).to_some(),
                &[DEFAULT_ASSET_TYPE],
            )
            .to_some(),
        );
        let secondary_asset_definition = AssetDefinitionV3::new(
            DEFAULT_SECONDARY_ASSET_TYPE,
            Some("secondary asset"),
            vec![secondary_verifier.clone()],
        );
        add_asset_definition(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            AddAssetDefinitionV1 {
                asset_definition: secondary_asset_definition.clone(),
                bind_name: Some(false),
            },
        )
        .expect("adding the secondary asset definition should succeed");
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("onboarding as the default asset type should succeed");
        let response = test_onboard_asset(
            &mut deps,
            TestOnboardAsset {
                onboard_asset: OnboardAssetV1 {
                    asset_type: DEFAULT_SECONDARY_ASSET_TYPE.to_string(),
                    ..TestOnboardAsset::default_onboard_asset()
                },
                ..TestOnboardAsset::default()
            },
        )
        .expect("onboarding the subsequent asset type should succeed");
        test_no_money_moved_in_response(
            &response,
            "no funds should be sent when onboarding a subsequent type with free subsequent onboards",
        );
    }

    #[test]
    fn test_update_attribute_generates_appropriate_messages() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_verify_asset(&mut deps, TestVerifyAsset::default()).unwrap();
        let service = AssetMetaService::new(deps.as_mut());
        let original_attribute = service
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect("the attribute should be present after all default steps");
        assert_eq!(
            AssetOnboardingStatus::Approved,
            original_attribute.onboarding_status,
            "sanity check: the onboarding status should be approved after all proper steps have been completed",
        );
        let mut updated_attribute = original_attribute.clone();
        // Manually override the onboarding status to pending to test
        updated_attribute.onboarding_status = AssetOnboardingStatus::Pending;
        service
            .update_attribute(&updated_attribute)
            .expect("update attribute should work as intended");
        let generated_messages = service.get_messages();
        assert_eq!(
            1,
            generated_messages.len(),
            "the service should generate one message when updating an attribute"
        );
        let target_attribute_name =
            generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME);
        match &generated_messages[0] {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::UpdateAttribute {
                        address,
                        name,
                        original_value,
                        original_value_type,
                        update_value,
                        update_value_type,
                    }),
                ..
            }) => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS,
                    address.as_str(),
                    "expected the delete attribute message to target the default scope address",
                );
                assert_eq!(
                    &target_attribute_name,
                    name,
                    "expected the default attribute name to be the target used when deleting the attribute",
                );
                assert_eq!(
                    original_attribute,
                    from_binary(original_value)
                        .expect("the original_value should deserialize to an AssetScopeAttribute"),
                    "the original_value binary should reflect the original state of the attribute",
                );
                assert_eq!(
                    &AttributeValueType::Json,
                    original_value_type,
                    "the original_value_type should always be json",
                );
                assert_eq!(
                    updated_attribute,
                    from_binary(update_value)
                        .expect("the update_value should deserialize to an AssetScopeAttribute"),
                    "the update_value binary should reflect the updated state of the attribute",
                );
                assert_eq!(
                    &AttributeValueType::Json,
                    update_value_type,
                    "the update_value_type should always be json",
                );
            }
            msg => panic!(
                "unexpected first message generated during update attribute: {:?}",
                msg
            ),
        };
    }

    #[test]
    fn test_onboard_with_object_store_gateway_permissions() {
        let get_onboard_result = |permission_spec: Option<bool>| {
            let mut deps = mock_dependencies(&[]);
            setup_test_suite(&mut deps, InstArgs::default());
            execute(
                deps.as_mut(),
                mock_env(),
                empty_mock_info(DEFAULT_SENDER_ADDRESS),
                OnboardAsset {
                    identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS)
                        .to_serialized_enum(),
                    asset_type: DEFAULT_ASSET_TYPE.into(),
                    verifier_address: DEFAULT_VERIFIER_ADDRESS.into(),
                    access_routes: get_default_access_routes().to_some(),
                    add_os_gateway_permission: permission_spec,
                },
            )
        };

        // Proves that omitting the permission param will default to true and produce all expected
        // os gateway permission attributes
        let default_response = get_onboard_result(None).expect(
            "onboarding should succeed with good params and default os gateway permissions",
        );
        assert_onboard_response_attributes_are_correct(&default_response, true);

        // Proves that explicitly providing the permission param as true will produce all expected os
        // gateway permission attributes
        let explicit_true_response = get_onboard_result(true.to_some()).expect(
            "onboarding should succeed with good params and explicit true os gateway permissions",
        );
        assert_onboard_response_attributes_are_correct(&explicit_true_response, true);

        // Proves that explicitly providing the permission param as false will omit all the os
        // gateway permission attributes
        let explicit_false_response = get_onboard_result(false.to_some()).expect(
            "onboarding should success with good params and explicit false os gateway permissions",
        );
        assert_onboard_response_attributes_are_correct(&explicit_false_response, false);
    }

    fn assert_onboard_response_attributes_are_correct(
        response: &Response<ProvenanceMsg>,
        expect_os_gateway_values: bool,
    ) {
        assert_eq!(
            6 + if expect_os_gateway_values { 4 } else { 0 },
            response.attributes.len(),
            "the correct number of response attributes should be emitted",
        );
        assert_eq!(
            "onboard_asset",
            single_attribute_for_key(response, ASSET_EVENT_TYPE_KEY),
            "the correct event type attribute should be emitted",
        );
        assert_eq!(
            DEFAULT_ASSET_TYPE,
            single_attribute_for_key(response, ASSET_TYPE_KEY),
            "the correct asset type attribute should be emitted",
        );
        assert_eq!(
            DEFAULT_SCOPE_ADDRESS,
            single_attribute_for_key(response, ASSET_SCOPE_ADDRESS_KEY),
            "the correct scope address attribute should be emitted",
        );
        assert_eq!(
            DEFAULT_VERIFIER_ADDRESS,
            single_attribute_for_key(response, VERIFIER_ADDRESS_KEY),
            "the correct verifier address attribute should be emitted",
        );
        assert_eq!(
            DEFAULT_SENDER_ADDRESS,
            single_attribute_for_key(response, SCOPE_OWNER_KEY),
            "the correct scope owner address attribute should be emitted",
        );
        assert_eq!(
            AssetOnboardingStatus::Pending.to_string(),
            single_attribute_for_key(response, NEW_ASSET_ONBOARDING_STATUS_KEY),
            "the new onboarding status after a successful onboard should always be pending",
        );
        if !expect_os_gateway_values {
            return;
        }
        assert_eq!(
            OS_GATEWAY_EVENT_TYPES.access_grant,
            single_attribute_for_key(response, OS_GATEWAY_KEYS.event_type),
            "the correct object store gateway event type attribute should be emitted",
        );
        assert_eq!(
            DEFAULT_SCOPE_ADDRESS,
            single_attribute_for_key(response, OS_GATEWAY_KEYS.scope_address),
            "the correct object store gateway scope address attribute should be emitted",
        );
        assert_eq!(
            DEFAULT_VERIFIER_ADDRESS,
            single_attribute_for_key(response, OS_GATEWAY_KEYS.target_account),
            "the correct object store gateway target account address attribute should be emitted",
        );
        assert_eq!(
            generate_os_gateway_grant_id(DEFAULT_ASSET_TYPE, DEFAULT_SCOPE_ADDRESS),
            single_attribute_for_key(response, OS_GATEWAY_KEYS.access_grant_id),
            "the correct object store gateway access grant id attribute should be emitted",
        );
    }
}
