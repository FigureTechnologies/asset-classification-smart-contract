use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::types::access_route::AccessRoute;
use crate::core::types::asset_identifier::AssetIdentifier;
use crate::core::types::asset_onboarding_status::AssetOnboardingStatus;
use crate::service::asset_meta_repository::AssetMetaRepository;
use crate::service::deps_manager::DepsManager;
use crate::service::message_gathering_service::MessageGatheringService;
use crate::util::aliases::{AssetResult, EntryPointResponse};
use crate::util::contract_helpers::check_funds_are_empty;
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::functions::generate_os_gateway_grant_id;

use cosmwasm_std::{MessageInfo, Response};
use os_gateway_contract_attributes::OsGatewayAttributeGenerator;
use result_extensions::ResultExtensions;

/// A transformation of [ExecuteMsg::VerifyAsset](crate::core::msg::ExecuteMsg::VerifyAsset)
/// for ease of use in the underlying [verify_asset](self::verify_asset) function.
///
/// # Parameters
///
/// * `identifier` An instance of the asset identifier enum that helps the contract identify which
/// scope that the requestor is referring to in the request.
/// * `success` A boolean indicating whether or not verification was successful.  A value of `false`
/// either indicates that the underlying data was fetched and it did not meet the requirements for a
/// classified asset, or that a failure occurred during the verification process.  Note: Verifiers
/// should be wary of returning false immediately on a code failure, as this incurs additional cost
/// to the onboarding account.  Instead, it is recommended that verification implement some process
/// that retries logic when exceptions or other code execution issues cause a failed verification.
/// * `message` An optional string describing the result of the verification process.  If omitted,
/// a standard message describing success or failure based on the value of `success` will be
/// displayed in the [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
/// * `access_routes` Like in the [OnboardAsset](self::ExecuteMsg::OnboardAsset) message, this
/// parameter allows the verifier to provide access routes for  the assets that it has successfully
/// fetched from the underlying scope data.  This allows for the verifier to define its own subset
/// of [AccessRoute](crate::core::types::access_route::AccessRoute) values to allow actors with permission
/// to easily fetch asset data from a new location, potentially without any Provenance Blockchain
/// interaction, facilitating the process of data interaction.
#[derive(Clone, PartialEq, Eq)]
pub struct VerifyAssetV1 {
    pub identifier: AssetIdentifier,
    pub asset_type: String,
    pub success: bool,
    pub message: Option<String>,
    pub access_routes: Vec<AccessRoute>,
}
impl VerifyAssetV1 {
    /// Attempts to create an instance of this struct from a provided execute msg.  If the provided
    /// value is not of the [VerifyAsset](crate::core::msg::ExecuteMsg::VerifyAsset)
    /// variant, then an [InvalidMessageType](crate::core::error::ContractError::InvalidMessageType)
    /// error will be returned.
    ///
    /// # Parameters
    ///
    /// * `msg` An execute msg provided by the contract's [execute](crate::contract::execute) function.
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<VerifyAssetV1> {
        match msg {
            ExecuteMsg::VerifyAsset {
                identifier,
                asset_type,
                success,
                message,
                access_routes,
            } => VerifyAssetV1 {
                identifier: identifier.to_asset_identifier()?,
                asset_type,
                success,
                message,
                access_routes: access_routes.unwrap_or_default(),
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::VerifyAsset".to_string(),
            }
            .to_err(),
        }
    }
}

/// The function used by [execute](crate::contract::execute) when an [ExecuteMsg::OnboardAsset](crate::core::msg::ExecuteMsg::OnboardAsset)
/// message is provided.  An execution route for use by the asset verifier selected by a requestor
/// during the [onboarding](super::onboard_asset::onboard_asset) process to mark a scope as verified
/// or rejected.
///
/// # Parameters
///
/// * `repository` A helper collection of traits that allows complex lookups of scope values and
/// emits messages to construct the process of verification as a collection of messages to produce
/// in the function's result.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` An instance of the verify asset v1 struct, provided by conversion from an
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).
pub fn verify_asset<'a, T>(
    repository: T,
    info: MessageInfo,
    msg: VerifyAssetV1,
) -> EntryPointResponse
where
    T: AssetMetaRepository + MessageGatheringService + DepsManager<'a>,
{
    // Ensure the verifier does not send funds - this entry point should only move funds TO entities,
    // not receive them for any reason
    check_funds_are_empty(&info)?;

    let asset_identifiers = msg.identifier.to_identifiers()?;
    // look up asset in repository
    let scope_attribute =
        repository.get_asset_by_asset_type(&asset_identifiers.scope_address, &msg.asset_type)?;

    // verify sender is requested verifier
    if info.sender != scope_attribute.verifier_address {
        return ContractError::UnauthorizedAssetVerifier {
            scope_address: asset_identifiers.scope_address,
            asset_type: msg.asset_type,
            verifier_address: info.sender.into(),
            expected_verifier_address: scope_attribute.verifier_address.into_string(),
        }
        .to_err();
    }

    // if the status is anything except pending, then verification has already run for the asset.
    // if the status is denied, then the asset can be retried through the onboarding process,
    // but if it was approved, then this route never needs to be run again
    if scope_attribute.onboarding_status != AssetOnboardingStatus::Pending {
        return ContractError::AssetAlreadyVerified {
            scope_address: asset_identifiers.scope_address,
            asset_type: msg.asset_type,
            status: scope_attribute.onboarding_status,
        }
        .to_err();
    }

    let updated_attribute =
        repository.verify_asset(scope_attribute, msg.success, msg.message, msg.access_routes)?;

    // construct/emit verification attributes
    Response::new()
        .add_attributes(
            EventAttributes::for_asset_event(
                EventType::VerifyAsset,
                &updated_attribute.asset_type,
                &asset_identifiers.scope_address,
            )
            .set_verifier(info.sender.as_str())
            .set_new_asset_onboarding_status(&updated_attribute.onboarding_status),
        )
        .add_attributes(
            OsGatewayAttributeGenerator::access_revoke(
                &asset_identifiers.scope_address,
                info.sender.as_str(),
            )
            .with_access_grant_id(generate_os_gateway_grant_id(
                &updated_attribute.asset_type,
                asset_identifiers.scope_address,
            )),
        )
        .add_messages(repository.get_messages())
        .to_ok()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Response, Uint128};
    use os_gateway_contract_attributes::{OS_GATEWAY_EVENT_TYPES, OS_GATEWAY_KEYS};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::ProvenanceMsg;
    use serde_json_wasm::to_string;

    use crate::core::state::may_load_fee_payment_detail;
    use crate::core::types::asset_definition::AssetDefinitionInputV3;
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::execute::onboard_asset::OnboardAssetV1;
    use crate::testutil::msg_utilities::test_no_money_moved_in_response;
    use crate::testutil::test_constants::{
        DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME, DEFAULT_SECONDARY_ASSET_TYPE,
    };
    use crate::testutil::test_utilities::{
        get_default_asset_definition_input, get_default_verifier_detail, single_attribute_for_key,
    };
    use crate::util::constants::{
        ASSET_EVENT_TYPE_KEY, ASSET_SCOPE_ADDRESS_KEY, ASSET_TYPE_KEY,
        NEW_ASSET_ONBOARDING_STATUS_KEY, VERIFIER_ADDRESS_KEY,
    };
    use crate::util::functions::{generate_asset_attribute_name, generate_os_gateway_grant_id};
    use crate::{
        core::{
            error::ContractError,
            types::{
                asset_identifier::AssetIdentifier, asset_onboarding_status::AssetOnboardingStatus,
            },
        },
        service::{
            asset_meta_repository::AssetMetaRepository, asset_meta_service::AssetMetaService,
        },
        testutil::{
            onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset},
            test_constants::{DEFAULT_SCOPE_ADDRESS, DEFAULT_VERIFIER_ADDRESS},
            test_utilities::{empty_mock_info, mock_info_with_nhash, setup_test_suite, InstArgs},
            verify_asset_helpers::{test_verify_asset, TestVerifyAsset},
        },
        util::traits::OptionExtensions,
    };

    use super::{verify_asset, VerifyAssetV1};

    #[test]
    fn test_verify_rejected_for_funds_present() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let err = verify_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_VERIFIER_ADDRESS, 420),
            TestVerifyAsset::default_verify_asset(),
        )
        .unwrap_err();
        assert!(
            matches!(err, ContractError::InvalidFunds(_)),
            "unexpected error type encountered when validating against funds present during verify asset: {:?}",
            err,
        );
    }

    #[test]
    fn test_verify_asset_not_found_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = verify_asset(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_VERIFIER_ADDRESS),
            VerifyAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                success: true,
                message: None,
                access_routes: vec![],
            },
        )
        .unwrap_err();

        match err {
            ContractError::NotFound { explanation } => {
                assert_eq!(
                    format!(
                        "scope at address [{}] did not include an asset scope attribute for asset type [{}]",
                        DEFAULT_SCOPE_ADDRESS,
                        DEFAULT_ASSET_TYPE
                    ),
                    explanation,
                    "the asset not found message should reflect that the asset was not found"
                );
            }
            _ => panic!(
                "unexpected error when non-onboarded asset provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_verify_asset_wrong_verifier_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();

        let info = empty_mock_info("tp129z88fpzthllrdzktw98cck3ypd34wv77nqfyl");
        let err = verify_asset(
            AssetMetaService::new(deps.as_mut()),
            info.clone(),
            VerifyAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                success: true,
                message: None,
                access_routes: vec![],
            },
        )
        .unwrap_err();

        match err {
            ContractError::UnauthorizedAssetVerifier {
                scope_address,
                asset_type,
                verifier_address,
                expected_verifier_address,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS, scope_address,
                    "the unauthorized verifier message should reflect the scope address"
                );
                assert_eq!(
                    DEFAULT_ASSET_TYPE, asset_type,
                    "the unauthorized verifier message should reflect the asset type"
                );
                assert_eq!(
                    info.sender.to_string(), verifier_address,
                    "the unauthorized verifier message should reflect the provided (sender) verifier address"
                );
                assert_eq!(
                    DEFAULT_VERIFIER_ADDRESS, expected_verifier_address,
                    "the unauthorized verifier message should reflect the expected verifier address (from onboarding)"
                );
            }
            _ => panic!(
                "unexpected error when unauthorized verifier submits verification: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_verify_asset_adds_error_message_on_negative_validation() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();

        let result = verify_asset(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_VERIFIER_ADDRESS),
            VerifyAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.to_string(),
                success: true,
                message: "Your data sucks".to_string().to_some(),
                access_routes: vec![],
            },
        )
        .unwrap();

        assert_eq!(
            2,
            result.messages.len(),
            "verify asset should produce two messages: update attribute msg to new status and bank send to default verifier"
        );
    }

    #[test]
    fn test_verify_errors_on_already_verified_success_true() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(
            &mut deps,
            TestOnboardAsset {
                onboard_asset: OnboardAssetV1 {
                    ..TestOnboardAsset::default_onboard_asset()
                },
                ..TestOnboardAsset::default()
            },
        )
        .unwrap();
        test_verify_asset(&mut deps, TestVerifyAsset::default()).unwrap();
        let err = verify_asset(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_VERIFIER_ADDRESS),
            TestVerifyAsset::default_verify_asset(),
        )
        .unwrap_err();
        match err {
            ContractError::AssetAlreadyVerified {
                scope_address,
                asset_type,
                status,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS, scope_address,
                    "the response message should contain the expected scope address",
                );
                assert_eq!(
                    DEFAULT_ASSET_TYPE, asset_type,
                    "the response message should contain the expected asset type",
                );
                assert_eq!(
                    status,
                    AssetOnboardingStatus::Approved,
                    "the response message should indicate that the asset was already approved by the verifier",
                );
            }
            _ => panic!(
                "unexpected error encountered when submitting duplicate verification: {:?}",
                err
            ),
        };
    }

    #[test]
    fn test_verify_errors_on_already_verified_success_false() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_verify_asset(&mut deps, TestVerifyAsset::default_with_success(false)).unwrap();
        let err = verify_asset(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_VERIFIER_ADDRESS),
            TestVerifyAsset::default_verify_asset(),
        )
        .unwrap_err();
        match err {
            ContractError::AssetAlreadyVerified {
                scope_address,
                asset_type,
                status,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS, scope_address,
                    "the response message should contain the expected scope address",
                );
                assert_eq!(
                    DEFAULT_ASSET_TYPE, asset_type,
                    "the response message should contain the expected asset type",
                );
                assert_eq!(
                    status,
                    AssetOnboardingStatus::Denied,
                    "the response message should indicate that the asset was denied by the verifier",
                );
            }
            _ => panic!(
                "unexpected error encountered when submitting duplicate verification: {:?}",
                err
            ),
        };
    }

    #[test]
    fn test_verify_asset_two_pending_verifications_do_not_conflict() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(
            &mut deps,
            InstArgs::default_with_additional_asset_types(vec![DEFAULT_SECONDARY_ASSET_TYPE]),
        );
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let default_attribute = AssetMetaService::new(deps.as_mut())
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .unwrap();
        // onboard asset for secondary class
        test_onboard_asset(
            &mut deps,
            TestOnboardAsset {
                onboard_asset: OnboardAssetV1 {
                    asset_type: DEFAULT_SECONDARY_ASSET_TYPE.into(),
                    ..TestOnboardAsset::default_onboard_asset()
                },
                ..TestOnboardAsset::default()
            },
        )
        .unwrap();
        let default_secondary_attribute = AssetMetaService::new(deps.as_mut())
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_SECONDARY_ASSET_TYPE)
            .unwrap();

        // dumb hack to get both attributes in the ProvenanceMockQuerier... our interception of the AddOrUpdateParams stuff can only
        // set the one attribute in the mock querier at a time, as the existing attributes get overwritten and there is no way to
        // access them in order to append
        deps.querier.with_attributes(
            DEFAULT_SCOPE_ADDRESS,
            &[
                (
                    &generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME),
                    to_string(&default_attribute)
                        .expect("expected the scope attribute to convert to json without error")
                        .as_str(),
                    "json",
                ),
                (
                    &generate_asset_attribute_name(
                        DEFAULT_SECONDARY_ASSET_TYPE,
                        DEFAULT_CONTRACT_BASE_NAME,
                    ),
                    to_string(&default_secondary_attribute)
                        .expect("expected the scope attribute to convert to json without error")
                        .as_str(),
                    "json",
                ),
            ],
        );
        // end dumb hack

        test_verify_asset(&mut deps, TestVerifyAsset::default()).unwrap();
        let updated_default_attribute = AssetMetaService::new(deps.as_mut())
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect("after validating the asset, the scope attribute should be present");
        assert_eq!(
            AssetOnboardingStatus::Approved,
            updated_default_attribute.onboarding_status,
            "the asset should be in approved status after onboarding with a status of success = true",
        );
        assert_eq!(
            None,
            may_load_fee_payment_detail(&deps.storage, DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE),
            "the asset's payment details should be removed after successful onboarding for a specific type"
        );
        may_load_fee_payment_detail(&deps.storage, DEFAULT_SCOPE_ADDRESS, DEFAULT_SECONDARY_ASSET_TYPE).expect("the asset's payment details for an unrelated secondary asset type should be unaffected by onboarding a different type");

        // dumb hack AGAIN to get both the updated attribute and untouched in the ProvenanceMockQuerier... our interception of the AddOrUpdateParams stuff can only
        // set the one attribute in the mock querier at a time, as the existing attributes get overwritten and there is no way to
        // access them in order to append... Not sure if I can really test this situation I guess
        deps.querier.with_attributes(
            DEFAULT_SCOPE_ADDRESS,
            &[
                (
                    &generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME),
                    to_string(&updated_default_attribute)
                        .expect("expected the scope attribute to convert to json without error")
                        .as_str(),
                    "json",
                ),
                (
                    &generate_asset_attribute_name(
                        DEFAULT_SECONDARY_ASSET_TYPE,
                        DEFAULT_CONTRACT_BASE_NAME,
                    ),
                    to_string(&default_secondary_attribute)
                        .expect("expected the scope attribute to convert to json without error")
                        .as_str(),
                    "json",
                ),
            ],
        );
        // end dumb hack AGAIN

        test_verify_asset(
            &mut deps,
            TestVerifyAsset {
                verify_asset: VerifyAssetV1 {
                    asset_type: DEFAULT_SECONDARY_ASSET_TYPE.into(),
                    ..TestVerifyAsset::default_verify_asset()
                },
                ..TestVerifyAsset::default()
            },
        )
        .unwrap();
        assert_eq!(
            None,
            may_load_fee_payment_detail(&deps.storage, DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE),
            "the asset's payment details should still be missing for the initial verification asset type"
        );
        assert_eq!(
            None,
            may_load_fee_payment_detail(&deps.storage, DEFAULT_SCOPE_ADDRESS, DEFAULT_SECONDARY_ASSET_TYPE),
            "the asset's payment details should still be removed for the secondary verification asset type"
        );
    }

    #[test]
    fn test_verify_asset_wrong_asset_type_denied() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(
            &mut deps,
            InstArgs::default_with_additional_asset_types(vec![DEFAULT_SECONDARY_ASSET_TYPE]),
        );
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let err = test_verify_asset(
            &mut deps,
            TestVerifyAsset {
                verify_asset: VerifyAssetV1 {
                    asset_type: DEFAULT_SECONDARY_ASSET_TYPE.to_string(),
                    ..TestVerifyAsset::default_verify_asset()
                },
                ..TestVerifyAsset::default()
            },
        )
        .expect_err("attempting to validate an asset as the wrong type should be denied");
        match err {
                ContractError::NotFound { explanation } => assert_eq!(
                    format!("scope at address [{}] did not include an asset scope attribute for asset type [{}]", DEFAULT_SCOPE_ADDRESS, DEFAULT_SECONDARY_ASSET_TYPE),
                    explanation,
                    "the error message should reflect the scope address the verification was attempted for"
                ),
                e => panic!("unexpected error type {:?}", e)
            }
    }

    #[test]
    fn test_verify_asset_success_true_produces_correct_onboarding_status() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let response = test_verify_asset(&mut deps, TestVerifyAsset::default()).unwrap();
        assert_verify_response_attributes_are_correct(&response, AssetOnboardingStatus::Approved);
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect("after validating the asset, the scope attribute should be present");
        assert_eq!(
            AssetOnboardingStatus::Approved,
            attribute.onboarding_status,
            "the asset should be in approved status after onboarding with a status of success = true",
        );
    }

    #[test]
    fn test_verify_asset_success_false_produces_correct_onboarding_status() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let response =
            test_verify_asset(&mut deps, TestVerifyAsset::default_with_success(false)).unwrap();
        assert_verify_response_attributes_are_correct(&response, AssetOnboardingStatus::Denied);
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect("after validating the asset, the scope attribute should be present");
        assert_eq!(
            AssetOnboardingStatus::Denied,
            attribute.onboarding_status,
            "the asset should be in denied status after onboarding with a status of success = false",
        );
    }

    #[test]
    fn test_verify_asset_does_not_send_funds_when_onboarding_was_free() {
        let mut deps = mock_dependencies(&[]);
        // Setup as normal, but make onboarding free
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
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("onboarding when free should succeed");
        let response = test_verify_asset(&mut deps, TestVerifyAsset::default())
            .expect("verification when onboarding is free should succeed");
        test_no_money_moved_in_response(
            &response,
            "verification after a free onboard should not produce any messages that move funds",
        );
    }

    fn assert_verify_response_attributes_are_correct(
        response: &Response<ProvenanceMsg>,
        expected_onboarding_status: AssetOnboardingStatus,
    ) {
        assert_eq!(
            9,
            response.attributes.len(),
            "the correct number of response attributes should be emitted",
        );
        assert_eq!(
            "verify_asset",
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
            expected_onboarding_status.to_string(),
            single_attribute_for_key(response, NEW_ASSET_ONBOARDING_STATUS_KEY),
            "expected the correct onboarding status to be emitted",
        );
        assert_eq!(
            OS_GATEWAY_EVENT_TYPES.access_revoke,
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
