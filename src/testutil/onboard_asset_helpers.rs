use crate::core::asset::AssetScopeAttribute;
use crate::core::error::ContractError;
use crate::execute::onboard_asset::{onboard_asset, OnboardAssetV1};
use crate::testutil::test_utilities::{
    MockOwnedDeps, DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME, DEFAULT_INFO_NAME,
    DEFAULT_ONBOARDING_DENOM, DEFAULT_SCOPE_ADDRESS, DEFAULT_VALIDATOR_ADDRESS,
};
use crate::util::asset_meta_repository::AssetMetaRepository;
use crate::util::message_gathering_service::MessageGatheringService;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{coin, from_binary, CosmosMsg, Env, MessageInfo, Response};
use provwasm_std::ProvenanceMsg;
use serde_json_wasm::to_string;

use super::test_utilities::DEFAULT_ONBOARDING_COST;

pub struct TestOnboardAsset {
    pub env: Env,
    pub info: MessageInfo,
    pub contract_base_name: String,
    pub onboard_asset: OnboardAssetV1,
}
impl TestOnboardAsset {
    pub fn default_onboard_asset() -> OnboardAssetV1 {
        OnboardAssetV1 {
            asset_type: DEFAULT_ASSET_TYPE.to_string(),
            scope_address: DEFAULT_SCOPE_ADDRESS.to_string(),
            validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
        }
    }

    pub fn default_full_sender(sender: &str, amount: u128, denom: &str) -> Self {
        TestOnboardAsset {
            info: mock_info(sender, &[coin(amount, denom)]),
            ..Default::default()
        }
    }

    pub fn default_with_coin(amount: u128, denom: &str) -> Self {
        Self::default_full_sender(DEFAULT_INFO_NAME, amount, denom)
    }

    pub fn default_with_sender(sender: &str) -> Self {
        Self::default_full_sender(sender, DEFAULT_ONBOARDING_COST, DEFAULT_ONBOARDING_DENOM)
    }

    pub fn default_with_amount(amount: u128) -> Self {
        Self::default_full_sender(DEFAULT_INFO_NAME, amount, DEFAULT_ONBOARDING_DENOM)
    }

    pub fn default_with_denom(denom: &str) -> Self {
        Self::default_full_sender(DEFAULT_INFO_NAME, DEFAULT_ONBOARDING_COST, denom)
    }
}
impl Default for TestOnboardAsset {
    fn default() -> Self {
        TestOnboardAsset {
            info: mock_info(
                DEFAULT_INFO_NAME,
                &[coin(
                    DEFAULT_ONBOARDING_COST,
                    DEFAULT_ONBOARDING_DENOM.to_string(),
                )],
            ),
            contract_base_name: DEFAULT_CONTRACT_BASE_NAME.to_string(),
            onboard_asset: TestOnboardAsset::default_onboard_asset(),
            env: mock_env(),
        }
    }
}

pub fn test_onboard_asset<T: AssetMetaRepository + MessageGatheringService>(
    deps: &mut MockOwnedDeps,
    asset_meta_repository: &mut T,
    msg: TestOnboardAsset,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    let response = onboard_asset(
        deps.as_mut(),
        msg.env,
        msg.info,
        asset_meta_repository,
        msg.onboard_asset,
    );
    // todo: move this into some sort of helper function?
    asset_meta_repository
        .get_messages()
        .iter()
        .for_each(|m| match m {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    provwasm_std::ProvenanceMsgParams::Attribute(
                        provwasm_std::AttributeMsgParams::AddAttribute {
                            address,
                            name,
                            value,
                            ..
                        },
                    ),
                ..
            }) => {
                // inject bound name into provmock querier
                let deserialized: AssetScopeAttribute = from_binary(value).unwrap();
                deps.querier.with_attributes(
                    address.as_str(),
                    &[(
                        name.as_str(),
                        to_string(&deserialized).unwrap().as_str(),
                        "json",
                    )],
                )
            }
            _ => panic!("Unexpected message type from onboard_asset call in test_onboard_asset"),
        });
    response
}