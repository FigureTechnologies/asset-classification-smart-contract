use crate::core::asset::{AssetIdentifier, AssetOnboardingStatus};
use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::service::asset_meta_repository::AssetMetaRepository;
use crate::service::deps_manager::DepsManager;
use crate::service::message_gathering_service::MessageGatheringService;
use crate::util::aliases::{ContractResponse, ContractResult};
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
    let asset_identifiers = msg.identifier.to_identifiers()?;
    // look up asset in repository
    let meta = repository.get_asset(&asset_identifiers.scope_address)?;

    // verify sender is requested validator
    if info.sender != meta.validator_address {
        return ContractError::UnathorizedAssetValidator {
            scope_address: asset_identifiers.scope_address,
            validator_address: info.sender.into(),
            expected_validator_address: meta.validator_address.into_string(),
        }
        .to_err();
    }

    if meta.onboarding_status == AssetOnboardingStatus::Approved {
        return ContractError::AssetAlreadyValidated {
            scope_address: asset_identifiers.scope_address,
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
                &meta.asset_type,
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
            test_constants::{
                DEFAULT_ONBOARDING_COST, DEFAULT_SCOPE_ADDRESS, DEFAULT_VALIDATOR_ADDRESS,
            },
            test_utilities::{mock_info_with_nhash, setup_test_suite, InstArgs},
            validate_asset_helpers::{test_validate_asset, TestValidateAsset},
        },
        util::traits::OptionExtensions,
    };

    use super::{validate_asset, ValidateAssetV1};

    #[test]
    fn test_validate_asset_not_found_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = validate_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_VALIDATOR_ADDRESS, DEFAULT_ONBOARDING_COST),
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

        let info = mock_info_with_nhash(
            "tp129z88fpzthllrdzktw98cck3ypd34wv77nqfyl",
            DEFAULT_ONBOARDING_COST,
        );
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

        let info = mock_info_with_nhash(DEFAULT_VALIDATOR_ADDRESS, DEFAULT_ONBOARDING_COST);

        let result = validate_asset(
            AssetMetaService::new(deps.as_mut()),
            info.clone(),
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
