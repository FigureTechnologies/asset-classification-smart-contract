use cosmwasm_std::{testing::message_info, Addr, MessageInfo};

use crate::{
    core::types::asset_identifier::AssetIdentifier,
    execute::verify_asset::{verify_asset, VerifyAssetV1},
    service::asset_meta_service::AssetMetaService,
    util::{aliases::EntryPointResponse, traits::OptionExtensions},
};

use super::{
    test_constants::{
        DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME, DEFAULT_SCOPE_ADDRESS,
        DEFAULT_VERIFIER_ADDRESS,
    },
    test_utilities::{get_default_access_routes, intercept_add_or_update_attribute, MockOwnedDeps},
};

pub struct TestVerifyAsset {
    pub info: MessageInfo,
    pub contract_base_name: String,
    pub verify_asset: VerifyAssetV1,
}
impl TestVerifyAsset {
    pub fn default_verify_asset() -> VerifyAssetV1 {
        VerifyAssetV1 {
            identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
            asset_type: DEFAULT_ASSET_TYPE.into(),
            success: true,
            message: "Verified asset without errors".to_string().to_some(),
            access_routes: get_default_access_routes(),
        }
    }

    pub fn default_with_success(success: bool) -> Self {
        TestVerifyAsset {
            verify_asset: VerifyAssetV1 {
                success,
                ..Self::default_verify_asset()
            },
            ..Self::default()
        }
    }
}
impl Default for TestVerifyAsset {
    fn default() -> Self {
        Self {
            info: message_info(&Addr::unchecked(DEFAULT_VERIFIER_ADDRESS), &[]),
            contract_base_name: DEFAULT_CONTRACT_BASE_NAME.to_string(),
            verify_asset: TestVerifyAsset::default_verify_asset(),
        }
    }
}

pub fn test_verify_asset(
    deps: &mut MockOwnedDeps,
    env: &cosmwasm_std::Env,
    msg: TestVerifyAsset,
) -> EntryPointResponse {
    verify_asset(
        env,
        AssetMetaService::new(deps.as_mut()),
        msg.info,
        msg.verify_asset,
    )
    .and_then(|response| {
        intercept_add_or_update_attribute(deps, response, "failure occurred for test_verify_asset")
    })
}
