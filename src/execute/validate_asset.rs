use crate::core::asset::AssetOnboardingStatus;
use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::asset_meta_repository::AssetMetaRepository;
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::message_gathering_service::MessageGatheringService;
use crate::util::scope_address_utils::get_validate_scope_address;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Env, MessageInfo, Response};

#[derive(Clone, PartialEq)]
pub struct ValidateAssetV1 {
    pub scope_address: String,
    pub success: bool,
    pub message: Option<String>,
}
impl ValidateAssetV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<ValidateAssetV1> {
        match msg {
            ExecuteMsg::ValidateAsset {
                asset_uuid,
                scope_address,
                success,
                message,
            } => {
                let scope_address = get_validate_scope_address(asset_uuid, scope_address)?;

                ValidateAssetV1 {
                    scope_address,
                    success,
                    message,
                }
                .to_ok()
            }
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::ValidateAsset".to_string(),
            }
            .to_err(),
        }
    }
}

pub fn validate_asset<T: AssetMetaRepository + MessageGatheringService>(
    deps: DepsMutC,
    _env: Env,
    info: MessageInfo,
    asset_meta_repository: &mut T,
    msg: ValidateAssetV1,
) -> ContractResponse {
    // look up asset in repository
    let meta = asset_meta_repository.get_asset(&deps.as_ref(), msg.scope_address.clone())?;

    // verify sender is requested validator
    if info.sender != meta.validator_address {
        return ContractError::UnathorizedAssetValidator {
            scope_address: msg.scope_address,
            validator_address: info.sender.into(),
            expected_validator_address: meta.validator_address.into_string(),
        }
        .to_err();
    }

    if meta.onboarding_status == AssetOnboardingStatus::Approved {
        return ContractError::AssetAlreadyValidated {
            scope_address: msg.scope_address,
        }
        .to_err();
    }

    asset_meta_repository.validate_asset(
        &deps.as_ref(),
        msg.scope_address.clone(),
        msg.success,
        msg.message,
    )?;

    // construct/emit validation attribute
    Ok(Response::new()
        .add_attributes(
            EventAttributes::for_asset_event(
                EventType::ValidateAsset,
                meta.asset_type,
                msg.scope_address,
            )
            .set_validator(info.sender),
        )
        .add_messages(asset_meta_repository.get_messages()))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::mock_env, Addr, MessageInfo};
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::error::ContractError,
        testutil::{
            onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset},
            test_utilities::{
                mock_info_with_nhash, setup_test_suite, InstArgs, DEFAULT_ONBOARDING_COST,
                DEFAULT_SCOPE_ADDRESS, DEFAULT_VALIDATOR_ADDRESS,
            },
        },
        util::message_gathering_service::MessageGatheringService,
    };

    use super::{validate_asset, ValidateAssetV1};

    #[test]
    fn test_validate_asset_not_found_error() {
        let mut deps = mock_dependencies(&[]);
        let mut repository = setup_test_suite(&mut deps, InstArgs::default());

        let err = validate_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(DEFAULT_ONBOARDING_COST),
            &mut repository,
            ValidateAssetV1 {
                scope_address: DEFAULT_SCOPE_ADDRESS.to_string(),
                success: true,
                message: None,
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
            _ => panic!("unexpected error when non-onboarded asset provided"),
        }
    }

    #[test]
    fn test_validate_asset_wrong_validator_error() {
        let mut deps = mock_dependencies(&[]);
        let mut repository = setup_test_suite(&mut deps, InstArgs::default());

        test_onboard_asset(&mut deps, &mut repository, TestOnboardAsset::default()).unwrap();

        let info = MessageInfo {
            sender: Addr::unchecked("totallybogusvalidatorimposter"),
            ..mock_info_with_nhash(DEFAULT_ONBOARDING_COST)
        };
        let err = validate_asset(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            &mut repository,
            ValidateAssetV1 {
                scope_address: DEFAULT_SCOPE_ADDRESS.to_string(),
                success: true,
                message: None,
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
            _ => panic!("unexpected error when unauthorized validator submits validation"),
        }
    }

    #[test]
    fn test_validate_asset_adds_error_message_on_negative_validation() {
        let mut deps = mock_dependencies(&[]);
        let mut repository = setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, &mut repository, TestOnboardAsset::default()).unwrap();
        repository.drain_messages();

        let info = MessageInfo {
            sender: Addr::unchecked(DEFAULT_VALIDATOR_ADDRESS),
            ..mock_info_with_nhash(DEFAULT_ONBOARDING_COST)
        };

        let result = validate_asset(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            &mut repository,
            ValidateAssetV1 {
                scope_address: DEFAULT_SCOPE_ADDRESS.to_string(),
                success: true,
                message: Some("Your data sucks".to_string()),
            },
        )
        .unwrap();

        assert_eq!(
            2,
            result.messages.len(),
            "validate asset should produce two messages (attribute delete/update combo)"
        );
    }
}
