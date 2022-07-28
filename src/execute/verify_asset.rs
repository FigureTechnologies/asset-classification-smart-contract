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
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};

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
#[derive(Clone, PartialEq)]
pub struct VerifyAssetV1 {
    pub identifier: AssetIdentifier,
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
                success,
                message,
                access_routes,
            } => VerifyAssetV1 {
                identifier: identifier.to_asset_identifier()?,
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
    let scope_attribute = repository.get_asset(&asset_identifiers.scope_address)?;

    // verify sender is requested verifier
    if info.sender != scope_attribute.verifier_address {
        return ContractError::UnauthorizedAssetVerifier {
            scope_address: asset_identifiers.scope_address,
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
            status: scope_attribute.onboarding_status,
        }
        .to_err();
    }

    repository.verify_asset(
        &asset_identifiers.scope_address,
        msg.success,
        msg.message,
        msg.access_routes,
    )?;

    // construct/emit verification attribute
    Response::new()
        .add_attributes(
            EventAttributes::for_asset_event(
                EventType::VerifyAsset,
                &scope_attribute.asset_type,
                &asset_identifiers.scope_address,
            )
            .set_verifier(info.sender),
        )
        .add_messages(repository.get_messages())
        .to_ok()
}

#[cfg(test)]
mod tests {
    use provwasm_mocks::mock_dependencies;

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
                        "scope at address [{}] did not include an asset scope attribute",
                        DEFAULT_SCOPE_ADDRESS
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
                success: true,
                message: None,
                access_routes: vec![],
            },
        )
        .unwrap_err();

        match err {
            ContractError::UnauthorizedAssetVerifier {
                scope_address,
                verifier_address,
                expected_verifier_address,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS, scope_address,
                    "the unauthorized verifier message should reflect the scope address"
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
                success: true,
                message: "Your data sucks".to_string().to_some(),
                access_routes: vec![],
            },
        )
        .unwrap();

        assert_eq!(
            2,
            result.messages.len(),
            "verify asset should produce two messages (update attribute msg and fee distribution to default verifier w/ no additional fee destinations)"
        );
    }

    #[test]
    fn test_verify_errors_on_already_verified_success_true() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
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
                status,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS, scope_address,
                    "the response message should contain the expected scope address",
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
                status,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS, scope_address,
                    "the response message should contain the expected scope address",
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
    fn test_verify_asset_success_true_produces_correct_onboarding_status() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_verify_asset(&mut deps, TestVerifyAsset::default()).unwrap();
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset(DEFAULT_SCOPE_ADDRESS)
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
        test_verify_asset(&mut deps, TestVerifyAsset::default_with_success(false)).unwrap();
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("after validating the asset, the scope attribute should be present");
        assert_eq!(
            AssetOnboardingStatus::Denied,
            attribute.onboarding_status,
            "the asset should be in denied status after onboarding with a status of success = false",
        );
    }
}
