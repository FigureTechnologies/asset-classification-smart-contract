use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{asset_meta, asset_state_read, AssetMeta};
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::scope_address_utils::get_validate_scope_address;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Env, MessageInfo, Response};
use provwasm_std::ProvenanceQuerier;

#[derive(Clone, Debug, PartialEq)]
pub struct OnboardAssetV1 {
    pub scope_address: String,
    pub asset_type: String,
    pub validator_address: String,
}
impl OnboardAssetV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<OnboardAssetV1> {
        match msg {
            ExecuteMsg::OnboardAsset {
                asset_uuid,
                asset_type,
                scope_address,
                validator_address,
            } => {
                let parsed_address = get_validate_scope_address(asset_uuid, scope_address)?;

                OnboardAssetV1 {
                    scope_address: parsed_address,
                    asset_type,
                    validator_address,
                }
                .to_ok()
            }
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::OnboardAsset".to_string(),
            }
            .to_err(),
        }
    }
}
impl ResultExtensions for OnboardAssetV1 {}

pub fn onboard_asset(
    deps: DepsMutC,
    _env: Env,
    info: MessageInfo,
    msg: OnboardAssetV1,
) -> ContractResponse {
    // get asset state config for type, or error if not present
    let asset_state = match asset_state_read(deps.storage, &msg.asset_type).load() {
        Ok(state) => {
            if !state.enabled {
                return ContractError::AssetTypeDisabled {
                    asset_type: msg.asset_type.to_string(),
                }
                .to_err();
            }
            state
        }
        Err(_) => {
            return ContractError::UnsupportedAssetType {
                asset_type: msg.asset_type.to_string(),
            }
            .to_err()
        }
    };

    // verify perscribed validator is present as a validator in asset state
    let validator_config = match asset_state
        .validators
        .into_iter()
        .find(|validator| validator.address == msg.validator_address)
    {
        Some(validator) => validator,
        None => {
            return ContractError::UnsupportedValidator {
                asset_type: msg.asset_type,
                validator_address: msg.validator_address,
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
    let sent_fee = match info.funds.into_iter().find(|funds| funds.denom == "nhash") {
        Some(funds) => funds,
        None => {
            return ContractError::InvalidFunds(format!(
                "Improper funds supplied for onboarding (expected {}nhash)",
                validator_config.onboarding_cost
            ))
            .to_err()
        }
    };
    if sent_fee.amount != validator_config.onboarding_cost {
        return ContractError::InvalidFunds(format!(
            "Improper fee of {}{} provided (expected {}nhash)",
            sent_fee.amount, sent_fee.denom, validator_config.onboarding_cost
        ))
        .to_err();
    };

    // verify asset (scope) exists
    let scope = match ProvenanceQuerier::new(&deps.querier).get_scope(&msg.scope_address) {
        Err(..) => {
            return ContractError::AssetNotFound {
                scope_address: msg.scope_address,
            }
            .to_err()
        }
        Ok(scope) => scope,
    };

    // verify that the sender of this message is a scope owner
    let sender = info.sender;
    if !scope
        .owners
        .into_iter()
        .any(|owner| owner.address == sender)
    {
        return ContractError::Unauthorized {
            explanation: "sender address does not own the scope".to_string(),
        }
        .to_err();
    }

    // verify asset metadata storage doesn't already contain this asset (i.e. it hasn't already been onboarded)
    let mut asset_storage = asset_meta(deps.storage);
    if let Some(..) = asset_storage
        .may_load(msg.scope_address.as_bytes())
        .unwrap()
    {
        return ContractError::AssetAlreadyOnboarded {
            scope_address: msg.scope_address,
        }
        .to_err();
    }

    // store asset metadata in contract storage, with assigned validator and provided fee (in case fee changes between onboarding and validation)
    if let Err(err) = asset_storage.save(
        msg.scope_address.as_bytes(),
        &AssetMeta::new(
            &msg.scope_address,
            &msg.asset_type,
            &msg.validator_address,
            sent_fee.amount,
        ),
    ) {
        return ContractError::AssetOnboardingError {
            asset_type: msg.asset_type,
            scope_address: msg.scope_address,
            message: err.to_string(),
        }
        .to_err();
    }

    Ok(Response::new().add_attributes(
        EventAttributes::for_asset_event(
            EventType::OnboardAsset,
            &msg.asset_type,
            &msg.scope_address,
        )
        .set_validator(msg.validator_address),
    ))
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::{testing::mock_env, Addr, Coin, Uint128};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{Party, PartyType, Scope};

    use crate::{
        core::{
            error::ContractError,
            state::{asset_meta, asset_meta_read, AssetMeta},
        },
        execute::toggle_asset_definition::{toggle_asset_definition, ToggleAssetDefinitionV1},
        testutil::test_utilities::{
            empty_mock_info, mock_info_with_funds, mock_info_with_nhash, test_instantiate_success,
            InstArgs, DEFAULT_ASSET_TYPE, DEFAULT_INFO_NAME, DEFAULT_ONBOARDING_COST,
            DEFAULT_VALIDATOR_ADDRESS,
        },
        util::constants::{
            ASSET_EVENT_TYPE_KEY, ASSET_SCOPE_ADDRESS_KEY, ASSET_TYPE_KEY, VALIDATOR_ADDRESS_KEY,
        },
    };

    use super::{onboard_asset, OnboardAssetV1};

    #[test]
    fn test_onboard_asset_errors_on_unsupported_asset_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(1000),
            OnboardAssetV1 {
                scope_address: "scope1234".into(),
                asset_type: "bogus".into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.into(),
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
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_disabled_asset_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        toggle_asset_definition(
            deps.as_mut(),
            empty_mock_info(),
            ToggleAssetDefinitionV1::new(DEFAULT_ASSET_TYPE),
        )
        .expect("toggling the asset definition to be disabled should succeed");
        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(1000),
            OnboardAssetV1 {
                scope_address: "scope420".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.into(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, ContractError::AssetTypeDisabled { .. }),
            "the request should be rejected for a disabled asset type",
        );
    }

    #[test]
    fn test_onboard_asset_errors_on_unsupported_validator() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(1000),
            OnboardAssetV1 {
                scope_address: "scope1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string() + "bogus".into(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::UnsupportedValidator {
                asset_type,
                validator_address,
            } => {
                assert_eq!(
                    DEFAULT_ASSET_TYPE, asset_type,
                    "the unsupported validator message should reflect the asset type provided"
                );
                assert_eq!(DEFAULT_VALIDATOR_ADDRESS.to_string() + "bogus".into(), validator_address, "the unsupported validator message should reflect the validator address provided");
            }
            _ => panic!("unexpected error when unsupported validator provided"),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_no_funds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_funds(&[]),
            OnboardAssetV1 {
                scope_address: "scope1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
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
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_extra_funds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_funds(&[
                Coin {
                    denom: "nhash".into(),
                    amount: Uint128::from(123u128),
                },
                Coin {
                    denom: "otherdenom".into(),
                    amount: Uint128::from(2432u128),
                },
            ]),
            OnboardAssetV1 {
                scope_address: "scope1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
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
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_wrong_fund_denom() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_funds(&[Coin {
                denom: "otherdenom".into(),
                amount: Uint128::from(2432u128),
            }]),
            OnboardAssetV1 {
                scope_address: "scope1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
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
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_wrong_fund_amount() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(DEFAULT_ONBOARDING_COST + 1),
            OnboardAssetV1 {
                scope_address: "scope1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
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
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_already_asset_not_found() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                scope_address: "scope1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::AssetNotFound { scope_address } => {
                assert_eq!(
                    "scope1234", scope_address,
                    "the asset not found message should reflect that the asset uuid was not found"
                );
            }
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_already_onboarded_asset() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());

        deps.querier.with_scope(Scope {
            scope_id: "scope1234".to_string(),
            specification_id: "".to_string(),
            owners: [Party {
                address: Addr::unchecked(DEFAULT_INFO_NAME),
                role: PartyType::Owner,
            }]
            .to_vec(),
            data_access: [].to_vec(),
            value_owner_address: Addr::unchecked(""),
        });

        let mut asset_storage = asset_meta(&mut deps.storage);
        asset_storage
            .save(
                b"scope1234",
                &AssetMeta::new(
                    "scope1234".to_string(),
                    "".to_string(),
                    "".to_string(),
                    Uint128::from(123u128),
                ),
            )
            .unwrap();

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                scope_address: "scope1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::AssetAlreadyOnboarded { scope_address } => {
                assert_eq!(
                    "scope1234",
                    scope_address,
                    "the asset already onboarded message should reflect that the asset uuid was already onboarded"
                );
            }
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }

    #[test]
    fn test_onboard_asset_succeeds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());

        deps.querier.with_scope(Scope {
            scope_id: "scope1234".to_string(),
            specification_id: "".to_string(),
            owners: [Party {
                address: Addr::unchecked(DEFAULT_INFO_NAME),
                role: PartyType::Owner,
            }]
            .to_vec(),
            data_access: [].to_vec(),
            value_owner_address: Addr::unchecked(""),
        });

        let result = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                scope_address: "scope1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap();

        let asset_storage = asset_meta_read(&deps.storage);

        let asset_entry = asset_storage.load(b"scope1234").unwrap();

        assert_eq!(
            "scope1234", asset_entry.scope_address,
            "Asset uuid in storage should match what was provided at onboarding"
        );

        assert_eq!(
            0,
            result.messages.len(),
            "Onboarding should not produce any additional messages"
        );

        assert_eq!(
            vec![
                (ASSET_EVENT_TYPE_KEY, "onboard_asset"),
                (ASSET_TYPE_KEY, DEFAULT_ASSET_TYPE),
                (ASSET_SCOPE_ADDRESS_KEY, "scope1234"),
                (VALIDATOR_ADDRESS_KEY, DEFAULT_VALIDATOR_ADDRESS)
            ],
            result.attributes
        );
    }
}
