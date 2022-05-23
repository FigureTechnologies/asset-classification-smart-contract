use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{config_read_v2, load_asset_definition_v2_by_type};
use crate::core::types::access_route::AccessRoute;
use crate::core::types::asset_identifier::AssetIdentifier;
use crate::core::types::asset_onboarding_status::AssetOnboardingStatus;
use crate::core::types::asset_scope_attribute::AssetScopeAttribute;
use crate::service::asset_meta_repository::AssetMetaRepository;
use crate::service::deps_manager::DepsManager;
use crate::service::message_gathering_service::MessageGatheringService;
use crate::util::aliases::{AssetResult, EntryPointResponse};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::{OptionExtensions, ResultExtensions};
use cosmwasm_std::{MessageInfo, Response};
use provwasm_std::ProvenanceQuerier;

/// A transformation of [ExecuteMsg::OnboardAsset](crate::core::msg::ExecuteMsg::OnboardAsset)
/// for ease of use in the underlying [onboard_asset](self::onboard_asset) function.
///
/// # Parameters
///
/// * `identifier` An instance of the asset identifier enum that helps the contract identify which
/// scope that the requestor is referring to in the request.
/// * `asset_type` [AssetDefinitionV2's](crate::core::types::asset_definition::AssetDefinitionV2) unique
/// [asset_type](crate::core::types::asset_definition::AssetDefinitionV2::asset_type) value.  This
/// value must correspond to an existing type in the contract's internal storage, or the request
/// for onboarding will be rejected.
/// * `verifier_address` The bech32 Provenance Blockchain [address](crate::core::types::verifier_detail::VerifierDetailV2::address)
/// of a [VerifierDetailV2](crate::core::types::verifier_detail::VerifierDetailV2) on the [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
/// referred to by the [asset_type](self::OnboardAssetV1::asset_type) property. If the address does
/// not refer to any existing verifier detail, the request will be rejected.
/// * `access_routes` A vector of access routes to be added to the generated [AssetScopeAttribute's](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
/// [AccessDefinition](crate::core::types::access_definition::AccessDefinition) for the [Requestor](crate::core::types::access_definition::AccessDefinitionType::Requestor)
/// entry.
#[derive(Clone, Debug, PartialEq)]
pub struct OnboardAssetV1 {
    pub identifier: AssetIdentifier,
    pub asset_type: String,
    pub verifier_address: String,
    pub access_routes: Vec<AccessRoute>,
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
            } => OnboardAssetV1 {
                identifier: identifier.to_asset_identifier()?,
                asset_type,
                verifier_address,
                access_routes: access_routes.unwrap_or_default(),
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
    info: MessageInfo,
    msg: OnboardAssetV1,
) -> EntryPointResponse
where
    T: AssetMetaRepository + MessageGatheringService + DepsManager<'a>,
{
    let asset_identifiers = msg.identifier.to_identifiers()?;
    // get asset definition config for type, or error if not present
    let asset_definition = match repository
        .use_deps(|d| load_asset_definition_v2_by_type(d.storage, &msg.asset_type))
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
    let verifier_config = match asset_definition
        .verifiers
        .into_iter()
        .find(|verifier| verifier.address == msg.verifier_address)
    {
        Some(verifier) => verifier,
        None => {
            return ContractError::UnsupportedVerifier {
                asset_type: msg.asset_type,
                verifier_address: msg.verifier_address,
            }
            .to_err()
        }
    };

    // verify sent funds match what is specified in the asset state
    if info.funds.len() != 1 {
        return ContractError::InvalidFunds(
            "Exactly one fund type (of nhash) should be sent".to_string(),
        )
        .to_err();
    }

    let sent_fee = match info.funds.iter().find(|funds| funds.denom == "nhash") {
        Some(funds) => funds,
        None => {
            return ContractError::InvalidFunds(format!(
                "Improper funds supplied for onboarding (expected {}nhash)",
                verifier_config.onboarding_cost
            ))
            .to_err()
        }
    };

    if sent_fee.amount != verifier_config.onboarding_cost {
        return ContractError::InvalidFunds(format!(
            "Improper fee of {}{} provided (expected {}nhash)",
            sent_fee.amount, sent_fee.denom, verifier_config.onboarding_cost
        ))
        .to_err();
    };

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

    // verify scope is of correct spec for provided asset_type
    if scope.specification_id != asset_definition.scope_spec_address {
        return ContractError::AssetSpecMismatch {
            asset_type: msg.asset_type,
            scope_address: asset_identifiers.scope_address,
            scope_spec_address: scope.specification_id,
            expected_scope_spec_address: asset_definition.scope_spec_address,
        }
        .to_err();
    }

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
        verifier_config,
        msg.access_routes,
    )?;

    // check to see if the attribute already exists, and determine if this is a fresh onboard or a subsequent one
    let is_retry = if let Some(scope_attribute) =
        repository.try_get_asset(&asset_identifiers.scope_address)?
    {
        match scope_attribute.onboarding_status {
            // If the attribute indicates that the asset is approved, then it's already fully onboarded and verified
            AssetOnboardingStatus::Approved => {
                return ContractError::AssetAlreadyOnboarded {
                    scope_address: asset_identifiers.scope_address,
                }
                .to_err();
            }
            // If the attribute indicates that the asset is pending, then it's currently waiting for verification
            AssetOnboardingStatus::Pending => {
                // Attributes in pending status should always have a verifier detail on them. Use it in the error message to show
                // which verifier may or may not be misbehaving
                return if let Some(verifier_detail) = scope_attribute.latest_verifier_detail {
                    ContractError::AssetPendingVerification { scope_address: scope_attribute.scope_address, verifier_address: verifier_detail.address }
                } else {
                    // If a verifier detail is not present on the attribute, but the status is pending, then a bug has occurred in the contract somewhere
                    ContractError::generic(format!("scope {} is pending verification, but has no verifier information. this scope needs manual intervention!", scope_attribute.scope_address))
                }.to_err();
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
    repository.onboard_asset(&new_asset_attribute, is_retry)?;

    Ok(Response::new()
        .add_attributes(
            EventAttributes::for_asset_event(
                EventType::OnboardAsset,
                &msg.asset_type,
                &asset_identifiers.scope_address,
            )
            .set_verifier(msg.verifier_address)
            .set_scope_owner(info.sender),
        )
        .add_messages(repository.get_messages()))
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::{from_binary, Coin, CosmosMsg, StdError, SubMsg, Uint128};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{
        AttributeMsgParams, AttributeValueType, Process, ProcessId, ProvenanceMsg,
        ProvenanceMsgParams, Record, Records,
    };

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
                DEFAULT_ONBOARDING_COST, DEFAULT_RECORD_SPEC_ADDRESS, DEFAULT_SCOPE_ADDRESS,
                DEFAULT_SCOPE_SPEC_ADDRESS, DEFAULT_SENDER_ADDRESS, DEFAULT_SESSION_ADDRESS,
                DEFAULT_VERIFIER_ADDRESS,
            },
            test_utilities::{
                empty_mock_info, get_default_access_routes, get_default_scope, get_duped_scope,
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
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, 1000),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: "bogus".into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.into(),
                access_routes: vec![],
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
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, 1000),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.into(),
                access_routes: vec![],
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
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, 1000),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string() + "bogus".into(),
                access_routes: vec![],
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
    fn test_onboard_asset_errors_on_no_funds() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
            },
        )
        .unwrap_err();

        match err {
            ContractError::InvalidFunds(message) => {
                assert_eq!(
                    "Exactly one fund type (of nhash) should be sent", message,
                    "the invalid funds message should reflect invalid amount of funds list"
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_extra_funds() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_funds(
                DEFAULT_SENDER_ADDRESS,
                &[
                    Coin {
                        denom: "nhash".into(),
                        amount: Uint128::from(123u128),
                    },
                    Coin {
                        denom: "otherdenom".into(),
                        amount: Uint128::from(2432u128),
                    },
                ],
            ),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
            },
        )
        .unwrap_err();

        match err {
            ContractError::InvalidFunds(message) => {
                assert_eq!(
                    "Exactly one fund type (of nhash) should be sent", message,
                    "the invalid funds message should reflect invalid amount of funds list"
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_wrong_fund_denom() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_funds(
                DEFAULT_SENDER_ADDRESS,
                &[Coin {
                    denom: "otherdenom".into(),
                    amount: Uint128::from(2432u128),
                }],
            ),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
            },
        )
        .unwrap_err();

        match err {
            ContractError::InvalidFunds(message) => {
                assert_eq!(
                    "Improper funds supplied for onboarding (expected 1000nhash)", message,
                    "the invalid funds message should reflect that improper funds were sent"
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_wrong_fund_amount() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST + 1),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
            },
        )
        .unwrap_err();

        match err {
            ContractError::InvalidFunds(message) => {
                assert_eq!(
                    format!(
                        "Improper fee of {}nhash provided (expected {}nhash)",
                        DEFAULT_ONBOARDING_COST + 1,
                        DEFAULT_ONBOARDING_COST
                    ),
                    message,
                    "the invalid funds message should reflect that improper funds were sent"
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
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(bogus_scope_address),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
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
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
            },
        )
        .unwrap_err();

        match err {
            ContractError::AssetPendingVerification {
                scope_address,
                verifier_address,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS,
                    scope_address,
                    "the asset pending verification message should reflect that the asset address is awaiting verification"
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
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            TestOnboardAsset::default_onboard_asset(),
        )
        .unwrap_err();
        match err {
            ContractError::AssetAlreadyOnboarded { scope_address } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS,
                    scope_address,
                    "the asset already onboarded message should reflect that the asset address was already onboarded",
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
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
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
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
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
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
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
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
            },
        )
        .expect("onboarding should succeed due to test mode being enabled");
    }

    #[test]
    fn test_onboard_asset_errors_on_scope_spec_message_type_configuration_mismatch() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let bogus_scope_spec_address = "specLolWhut".to_string();
        let scope = get_duped_scope(
            DEFAULT_SCOPE_ADDRESS,
            &bogus_scope_spec_address, // not the spec you are looking for/was configured as an asset_type in test_instantate_success
            DEFAULT_SENDER_ADDRESS,
        );
        deps.querier.with_scope(scope.clone());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: vec![],
            },
        )
        .unwrap_err();

        match err {
            ContractError::AssetSpecMismatch {
                asset_type,
                scope_address,
                scope_spec_address,
                expected_scope_spec_address,
            } => {
                assert_eq!(DEFAULT_ASSET_TYPE, asset_type);
                assert_eq!(DEFAULT_SCOPE_ADDRESS, scope_address);
                assert_eq!(bogus_scope_spec_address, scope_spec_address);
                assert_eq!(DEFAULT_SCOPE_SPEC_ADDRESS, expected_scope_spec_address);
            }
            _ => panic!(
                "expected the error to indicate the scope onboarding error, but got: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_succeeds() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let result = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                access_routes: get_default_access_routes(),
            },
        )
        .unwrap();

        assert_eq!(
            1,
            result.messages.len(),
            "Onboarding should produce only one (bind attribute) message"
        );

        let msg = result.messages.first();

        match msg {
            Some(SubMsg {
                msg:
                    CosmosMsg::Custom(ProvenanceMsg {
                        params:
                            ProvenanceMsgParams::Attribute(AttributeMsgParams::AddAttribute {
                                name,
                                value,
                                ..
                            }),
                        ..
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
            _ => panic!("Unexpected message from onboard_asset: {:?}", msg),
        }

        assert_eq!(
            vec![
                (ASSET_EVENT_TYPE_KEY, "onboard_asset"),
                (ASSET_TYPE_KEY, DEFAULT_ASSET_TYPE),
                (ASSET_SCOPE_ADDRESS_KEY, DEFAULT_SCOPE_ADDRESS),
                (VERIFIER_ADDRESS_KEY, DEFAULT_VERIFIER_ADDRESS),
                (SCOPE_OWNER_KEY, DEFAULT_SENDER_ADDRESS)
            ],
            result.attributes
        );
    }

    #[test]
    fn test_onboard_asset_retry_success() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        // Verify the asset to denied status
        test_verify_asset(&mut deps, TestVerifyAsset::default_with_success(false)).unwrap();
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("the default scope address should have an attribute attached to it");
        assert_eq!(
            attribute.onboarding_status,
            AssetOnboardingStatus::Denied,
            "sanity check: the onboarding status should be set to denied after the verifier marks the asset as success = false",
        );
        // Try to do a retry on onboarding
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let attribute = AssetMetaService::new(deps.as_mut())
        .get_asset(DEFAULT_SCOPE_ADDRESS)
        .expect("the default scope address should still contain an attribute after onboarding for a second time");
        assert_eq!(
            attribute.onboarding_status,
            AssetOnboardingStatus::Pending,
            "the onboarding status should now be set to pending after retrying the onboard process",
        );
    }

    #[test]
    fn test_update_attribute_generates_appropriate_messages() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_verify_asset(&mut deps, TestVerifyAsset::default()).unwrap();
        let service = AssetMetaService::new(deps.as_mut());
        let mut attribute = service
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("the attribute should be present after all default steps");
        assert_eq!(AssetOnboardingStatus::Approved, attribute.onboarding_status, "sanity check: the onboarding status should be approved after all proper steps have been completed");
        // Manually override the onboarding status to pending to test
        attribute.onboarding_status = AssetOnboardingStatus::Pending;
        service
            .update_attribute(&attribute)
            .expect("update attribute should work as intended");
        let generated_messages = service.get_messages();
        assert_eq!(
            2,
            generated_messages.len(),
            "the service should generate two messages when updating an asset"
        );
        let target_attribute_name =
            generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME);
        match &generated_messages[0] {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::DeleteAttribute {
                        address,
                        name,
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
            }
            msg => panic!(
                "unexpected first message generated during update attribute: {:?}",
                msg
            ),
        };
        match &generated_messages[1] {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::AddAttribute {
                        address,
                        name,
                        value,
                        value_type,
                    }),
                ..
            }) => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS,
                    address.as_str(),
                    "expected the add attribute message to target the default scope address",
                );
                assert_eq!(
                    &target_attribute_name,
                    name,
                    "expected the default attribute name to be the target used when adding the attribute",
                );
                let added_attribute = from_binary::<AssetScopeAttribute>(value)
                    .expect("expected the attribute value to deserialize to a scope attribute");
                assert_eq!(
                    attribute,
                    added_attribute,
                    "expected the added attribute to directly equate to the value passed into the function",
                );
                assert_eq!(
                    &AttributeValueType::Json,
                    value_type,
                    "expected the value type used to be json",
                );
            }
            msg => panic!(
                "unexpected second message generated during update attribute; {:?}",
                msg,
            ),
        };
    }
}
