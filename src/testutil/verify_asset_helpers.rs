use cosmwasm_std::{testing::mock_info, MessageInfo};

use crate::{
    core::asset::AssetIdentifier,
    execute::verify_asset::{verify_asset, VerifyAssetV1},
    service::asset_meta_service::AssetMetaService,
    util::{aliases::EntryPointResponse, traits::OptionExtensions},
};

use super::{
    test_constants::{
        DEFAULT_ACCESS_ROUTE, DEFAULT_CONTRACT_BASE_NAME, DEFAULT_SCOPE_ADDRESS,
        DEFAULT_VERIFIER_ADDRESS,
    },
    test_utilities::{intercept_add_attribute, MockOwnedDeps},
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
            success: true,
            message: "Verified asset without errors".to_string().to_some(),
            access_routes: vec![DEFAULT_ACCESS_ROUTE.to_string()],
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
            info: mock_info(DEFAULT_VERIFIER_ADDRESS, &[]),
            contract_base_name: DEFAULT_CONTRACT_BASE_NAME.to_string(),
            verify_asset: TestVerifyAsset::default_verify_asset(),
        }
    }
}

pub fn test_verify_asset(deps: &mut MockOwnedDeps, msg: TestVerifyAsset) -> EntryPointResponse {
    let response = verify_asset(
        AssetMetaService::new(deps.as_mut()),
        msg.info,
        msg.verify_asset,
    );
    intercept_add_attribute(deps, &response, "failure occurred for test_verify_asset");
    response
}
