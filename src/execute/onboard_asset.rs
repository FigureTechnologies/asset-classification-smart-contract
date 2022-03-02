use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{asset_meta, asset_state_read, AssetMeta};
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Env, MessageInfo, Response};
use provwasm_std::ProvenanceQuerier;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OnboardAssetV1 {
    pub asset_uuid: String,
    pub asset_type: String,
    pub scope_address: String,
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
            } => OnboardAssetV1 {
                asset_uuid,
                asset_type,
                scope_address,
                validator_address,
            }
            .to_ok(),
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
        Ok(state) => state,
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
    let scope = match ProvenanceQuerier::new(&deps.querier).get_scope(&msg.asset_uuid) {
        Err(..) => {
            return ContractError::AssetNotFound {
                asset_uuid: msg.asset_uuid,
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
        return ContractError::Unauthorized.to_err();
    }

    // verify asset metadata storage doesn't already contain this asset (i.e. it hasn't already been onboarded)
    let mut asset_storage = asset_meta(deps.storage);
    if let Some(..) = asset_storage.may_load(&msg.asset_uuid.as_bytes()).unwrap() {
        return ContractError::AssetAlreadyOnboarded {
            asset_uuid: msg.asset_uuid,
        }
        .to_err();
    }

    // store asset metadata in contract storage, with assigned validator and provided fee (in case fee changes between onboarding and validation)
    if let Err(err) = asset_storage.save(
        &msg.asset_uuid.as_bytes(),
        &AssetMeta::new(
            &msg.asset_uuid,
            &msg.asset_type,
            &msg.scope_address,
            &msg.validator_address,
            sent_fee.amount,
        ),
    ) {
        return ContractError::AssetOnboardingError {
            asset_type: msg.asset_type,
            asset_uuid: msg.asset_uuid,
            message: err.to_string(),
        }
        .to_err();
    }

    Ok(Response::new().add_attributes(
        EventAttributes::new(EventType::OnboardAsset, &msg.asset_type, &msg.asset_uuid)
            .set_validator(msg.validator_address),
    ))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::mock_env, Addr, Coin, Uint128};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{Party, PartyType, Scope};

    use crate::{
        core::{
            error::ContractError,
            state::{asset_meta, asset_meta_read, AssetMeta},
        },
        testutil::test_utilities::{
            mock_info_with_funds, mock_info_with_nhash, test_instantiate, InstArgs,
            DEFAULT_ASSET_TYPE, DEFAULT_INFO_NAME, DEFAULT_ONBOARDING_COST,
            DEFAULT_VALIDATOR_ADDRESS,
        },
        util::constants::{
            ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY, ASSET_UUID_KEY, VALIDATOR_ADDRESS_KEY,
        },
    };

    use super::{onboard_asset, OnboardAssetV1};

    #[test]
    fn test_onboard_asset_errors_on_unsupported_asset_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(1000),
            OnboardAssetV1 {
                asset_uuid: "1234".into(),
                asset_type: "bogus".into(),
                scope_address: "scope1234".into(),
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
    fn test_onboard_asset_errors_on_unsupported_validator() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(1000),
            OnboardAssetV1 {
                asset_uuid: "1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                scope_address: "scope1234".into(),
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
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_funds(&[]),
            OnboardAssetV1 {
                asset_uuid: "1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                scope_address: "scope1234".into(),
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
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

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
                asset_uuid: "1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                scope_address: "scope1234".into(),
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
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_funds(&[Coin {
                denom: "otherdenom".into(),
                amount: Uint128::from(2432u128),
            }]),
            OnboardAssetV1 {
                asset_uuid: "1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                scope_address: "scope1234".into(),
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
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(DEFAULT_ONBOARDING_COST + 1),
            OnboardAssetV1 {
                asset_uuid: "1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                scope_address: "scope1234".into(),
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
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                asset_uuid: "1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                scope_address: "scope1234".into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::AssetNotFound { asset_uuid } => {
                assert_eq!(
                    "1234", asset_uuid,
                    "the asset not found message should reflect that the asset uuid was not found"
                );
            }
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_already_onboarded_asset() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        deps.querier.with_scope(Scope {
            scope_id: "1234".to_string(),
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
                b"1234",
                &AssetMeta::new(
                    "1234".to_string(),
                    "".to_string(),
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
                asset_uuid: "1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                scope_address: "scope1234".into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::AssetAlreadyOnboarded { asset_uuid } => {
                assert_eq!(
                    "1234",
                    asset_uuid,
                    "the asset already onboarded message should reflect that the asset uuid was already onboarded"
                );
            }
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }

    #[test]
    fn test_onboard_asset_succeeds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        deps.querier.with_scope(Scope {
            scope_id: "1234".to_string(),
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
                asset_uuid: "1234".into(),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                scope_address: "scope1234".into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap();

        let asset_storage = asset_meta_read(&deps.storage);

        let asset_entry = asset_storage.load(b"1234").unwrap();

        assert_eq!(
            "1234", asset_entry.asset_uuid,
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
                (ASSET_UUID_KEY, "1234"),
                (VALIDATOR_ADDRESS_KEY, DEFAULT_VALIDATOR_ADDRESS)
            ],
            result.attributes
        );
    }
}
