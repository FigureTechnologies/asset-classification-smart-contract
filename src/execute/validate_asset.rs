use crate::core::asset::{AssetIdentifier, AssetOnboardingStatus};
use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::service::asset_meta_repository::AssetMetaRepository;
use crate::service::deps_manager::DepsManager;
use crate::service::message_gathering_service::MessageGatheringService;
use crate::util::aliases::{ContractResponse, ContractResult};
use crate::util::contract_helpers::check_funds_are_empty;
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};

#[derive(Clone, PartialEq)]
pub struct ValidateAssetV1 {
    pub identifier: AssetIdentifier,
    pub success: bool,
    pub message: Option<String>,
    pub access_routes: Vec<String>,
}
impl ValidateAssetV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<ValidateAssetV1> {
        match msg {
            ExecuteMsg::ValidateAsset {
                identifier,
                success,
                message,
                access_routes,
            } => ValidateAssetV1 {
                identifier,
                success,
                message,
                access_routes: access_routes.unwrap_or_default(),
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::ValidateAsset".to_string(),
            }
            .to_err(),
        }
    }
}

pub fn validate_asset<'a, T>(
    repository: T,
    info: MessageInfo,
    msg: ValidateAssetV1,
) -> ContractResponse
where
    T: AssetMetaRepository + MessageGatheringService + DepsManager<'a>,
{
    // Ensure the validator does not send funds - this entry point should only move funds TO entities,
    // not receive them for any reason
    check_funds_are_empty(&info)?;

    let asset_identifiers = msg.identifier.to_identifiers()?;
    // look up asset in repository
    let scope_attribute = repository.get_asset(&asset_identifiers.scope_address)?;

    // verify sender is requested validator
    if info.sender != scope_attribute.validator_address {
        return ContractError::UnathorizedAssetValidator {
            scope_address: asset_identifiers.scope_address,
            validator_address: info.sender.into(),
            expected_validator_address: scope_attribute.validator_address.into_string(),
        }
        .to_err();
    }

    // if the status is anything except pending, then validation has already run for the asset.
    // if the status is denied, then the asset can be retried through the onboarding process,
    // but if it was approved, then this route never needs to be run again
    if scope_attribute.onboarding_status != AssetOnboardingStatus::Pending {
        return ContractError::AssetAlreadyValidated {
            scope_address: asset_identifiers.scope_address,
            status: scope_attribute.onboarding_status,
        }
        .to_err();
    }

    repository.validate_asset(
        &asset_identifiers.scope_address,
        msg.success,
        msg.message,
        vec![],
    )?;

    // construct/emit validation attribute
    Ok(Response::new()
        .add_attributes(
            EventAttributes::for_asset_event(
                EventType::ValidateAsset,
                &scope_attribute.asset_type,
                &asset_identifiers.scope_address,
            )
            .set_validator(info.sender),
        )
        .add_messages(repository.get_messages()))
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::{
            asset::{AssetIdentifier, AssetOnboardingStatus},
            error::ContractError,
        },
        service::{
            asset_meta_repository::AssetMetaRepository, asset_meta_service::AssetMetaService,
        },
        testutil::{
            onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset},
            test_constants::{DEFAULT_SCOPE_ADDRESS, DEFAULT_VALIDATOR_ADDRESS},
            test_utilities::{empty_mock_info, mock_info_with_nhash, setup_test_suite, InstArgs},
            validate_asset_helpers::{test_validate_asset, TestValidateAsset},
        },
        util::traits::OptionExtensions,
    };

    use super::{validate_asset, ValidateAssetV1};

    #[test]
    fn test_validate_rejected_for_funds_present() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let err = validate_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_VALIDATOR_ADDRESS, 420),
            TestValidateAsset::default_validate_asset(),
        )
        .unwrap_err();
        assert!(
            matches!(err, ContractError::InvalidFunds(_)),
            "unexpected error type encountered when validating against funds present during validate asset: {:?}",
            err,
        );
    }

    #[test]
    fn test_validate_asset_not_found_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = validate_asset(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_VALIDATOR_ADDRESS),
            ValidateAssetV1 {
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
    fn test_validate_asset_wrong_validator_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();

        let info = empty_mock_info("tp129z88fpzthllrdzktw98cck3ypd34wv77nqfyl");
        let err = validate_asset(
            AssetMetaService::new(deps.as_mut()),
            info.clone(),
            ValidateAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                success: true,
                message: None,
                access_routes: vec![],
            },
        )
        .unwrap_err();

        match err {
            ContractError::UnathorizedAssetValidator {
                scope_address,
                validator_address,
                expected_validator_address,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS, scope_address,
                    "the unauthorized validator message should reflect the scope address"
                );
                assert_eq!(
                    info.sender.to_string(), validator_address,
                    "the unauthorized validator message should reflect the provided (sender) validator address"
                );
                assert_eq!(
                    DEFAULT_VALIDATOR_ADDRESS, expected_validator_address,
                    "the unauthorized validator message should reflect the expected validator address (from onboarding)"
                );
            }
            _ => panic!(
                "unexpected error when unauthorized validator submits validation: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_validate_asset_adds_error_message_on_negative_validation() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();

        let result = validate_asset(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_VALIDATOR_ADDRESS),
            ValidateAssetV1 {
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
            "validate asset should produce three messages (attribute delete/update combo and fee distribution to default validator w/ no additional fee destinations)"
        );
    }

    #[test]
    fn test_validate_errors_on_already_validated_success_true() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_validate_asset(&mut deps, TestValidateAsset::default()).unwrap();
        let err = validate_asset(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_VALIDATOR_ADDRESS),
            TestValidateAsset::default_validate_asset(),
        )
        .unwrap_err();
        match err {
            ContractError::AssetAlreadyValidated {
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
                    "the response message should indicate that the asset was already approved by the validator",
                );
            }
            _ => panic!(
                "unexpected error encountered when submitting duplicate validation: {:?}",
                err
            ),
        };
    }

    #[test]
    fn test_validate_errors_on_already_validated_success_false() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_validate_asset(&mut deps, TestValidateAsset::default_with_success(false)).unwrap();
        let err = validate_asset(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_VALIDATOR_ADDRESS),
            TestValidateAsset::default_validate_asset(),
        )
        .unwrap_err();
        match err {
            ContractError::AssetAlreadyValidated {
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
                    "the response message should indicate that the asset was denied by the validator",
                );
            }
            _ => panic!(
                "unexpected error encountered when submitting duplicate validation: {:?}",
                err
            ),
        };
    }

    #[test]
    fn test_validate_asset_success_true_produces_correct_onboarding_status() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_validate_asset(&mut deps, TestValidateAsset::default()).unwrap();
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
    fn test_validate_asset_success_false_produces_correct_onboarding_status() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_validate_asset(&mut deps, TestValidateAsset::default_with_success(false)).unwrap();
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
