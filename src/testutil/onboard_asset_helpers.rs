use crate::core::types::asset_identifier::AssetIdentifier;
use crate::execute::onboard_asset::{onboard_asset, OnboardAssetV1};
use crate::service::asset_meta_service::AssetMetaService;
use crate::testutil::test_utilities::{empty_mock_info, MockOwnedDeps};
use crate::util::aliases::EntryPointResponse;
use cosmwasm_std::testing::{message_info, mock_env};
use cosmwasm_std::{coin, Addr, MessageInfo};

use super::test_constants::{
    DEFAULT_ASSET_TYPE, DEFAULT_ONBOARDING_COST, DEFAULT_ONBOARDING_DENOM, DEFAULT_SCOPE_ADDRESS,
    DEFAULT_SENDER_ADDRESS, DEFAULT_VERIFIER_ADDRESS,
};
use super::test_utilities::{get_default_access_routes, intercept_add_or_update_attribute};

#[derive(Clone)]
pub struct TestOnboardAsset {
    pub info: MessageInfo,
    pub onboard_asset: OnboardAssetV1,
}
impl TestOnboardAsset {
    pub fn default_onboard_asset() -> OnboardAssetV1 {
        OnboardAssetV1 {
            identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
            asset_type: DEFAULT_ASSET_TYPE.to_string(),
            verifier_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
            access_routes: get_default_access_routes(),
            add_os_gateway_permission: true,
        }
    }

    pub fn default_full_sender(sender: &str, amount: u128, denom: &str) -> Self {
        TestOnboardAsset {
            info: message_info(&Addr::unchecked(sender), &[coin(amount, denom)]),
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
            info: empty_mock_info(DEFAULT_SENDER_ADDRESS),
            onboard_asset: TestOnboardAsset::default_onboard_asset(),
        }
    }
}

pub fn test_onboard_asset(deps: &mut MockOwnedDeps, msg: TestOnboardAsset) -> EntryPointResponse {
    onboard_asset(
        AssetMetaService::new(deps.as_mut()),
        mock_env(),
        msg.info,
        msg.onboard_asset,
    )
    .and_then(|response| {
        intercept_add_or_update_attribute(deps, response, "failure occurred for test_onboard_asset")
    })
}
