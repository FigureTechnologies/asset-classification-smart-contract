use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
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

#[derive(Clone, PartialEq)]
pub struct VerifyAssetV1 {
    pub identifier: AssetIdentifier,
    pub success: bool,
    pub message: Option<String>,
    pub access_routes: Vec<String>,
}
impl VerifyAssetV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<VerifyAssetV1> {
        match msg {
            ExecuteMsg::VerifyAsset {
                identifier,
                success,
                message,
                access_routes,
            } => VerifyAssetV1 {
                identifier,
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
        return ContractError::UnathorizedAssetVerifier {
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
    Ok(Response::new()
        .add_attributes(
            EventAttributes::for_asset_event(
                EventType::VerifyAsset,
                &scope_attribute.asset_type,
                &asset_identifiers.scope_address,
            )
            .set_verifier(info.sender),
        )
        .add_messages(repository.get_messages()))
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
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
            ContractError::UnathorizedAssetVerifier {
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
            3,
            result.messages.len(),
            "verify asset should produce three messages (attribute delete/update combo and fee distribution to default verifier w/ no additional fee destinations)"
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
