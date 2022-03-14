use crate::core::asset::AssetIdentifier;
use crate::execute::onboard_asset::{onboard_asset, OnboardAssetV1};
use crate::service::asset_meta_service::AssetMetaService;
use crate::testutil::test_utilities::MockOwnedDeps;
use crate::util::aliases::ContractResponse;
use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{coin, MessageInfo};

use super::test_constants::{
    DEFAULT_ACCESS_ROUTE, DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME, DEFAULT_ONBOARDING_COST,
    DEFAULT_ONBOARDING_DENOM, DEFAULT_SCOPE_ADDRESS, DEFAULT_SENDER_ADDRESS,
    DEFAULT_VALIDATOR_ADDRESS,
};
use super::test_utilities::intercept_add_attribute;

pub struct TestOnboardAsset {
    pub info: MessageInfo,
    pub contract_base_name: String,
    pub onboard_asset: OnboardAssetV1,
}
impl TestOnboardAsset {
    pub fn default_onboard_asset() -> OnboardAssetV1 {
        OnboardAssetV1 {
            identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
            asset_type: DEFAULT_ASSET_TYPE.to_string(),
            validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            access_routes: vec![DEFAULT_ACCESS_ROUTE.to_string()],
        }
    }

    pub fn default_full_sender(sender: &str, amount: u128, denom: &str) -> Self {
        TestOnboardAsset {
            info: mock_info(sender, &[coin(amount, denom)]),
            ..Default::default()
        }
    }

    pub fn default_with_coin(amount: u128, denom: &str) -> Self {
        Self::default_full_sender(DEFAULT_SENDER_ADDRESS, amount, denom)
    }

    pub fn default_with_sender(sender: &str) -> Self {
        Self::default_full_sender(sender, DEFAULT_ONBOARDING_COST, DEFAULT_ONBOARDING_DENOM)
    }

    pub fn default_with_amount(amount: u128) -> Self {
        Self::default_full_sender(DEFAULT_SENDER_ADDRESS, amount, DEFAULT_ONBOARDING_DENOM)
    }

    pub fn default_with_denom(denom: &str) -> Self {
        Self::default_full_sender(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST, denom)
    }
}
impl Default for TestOnboardAsset {
    fn default() -> Self {
        TestOnboardAsset {
            info: mock_info(
                DEFAULT_SENDER_ADDRESS,
                &[coin(
                    DEFAULT_ONBOARDING_COST,
                    DEFAULT_ONBOARDING_DENOM.to_string(),
                )],
            ),
            contract_base_name: DEFAULT_CONTRACT_BASE_NAME.to_string(),
            onboard_asset: TestOnboardAsset::default_onboard_asset(),
        }
    }
}

pub fn test_onboard_asset(deps: &mut MockOwnedDeps, msg: TestOnboardAsset) -> ContractResponse {
    let response = onboard_asset(
        AssetMetaService::new(deps.as_mut()),
        msg.info,
        msg.onboard_asset,
    );
    intercept_add_attribute(deps, &response, "failure occurred for test_onboard_asset");
    response
}
