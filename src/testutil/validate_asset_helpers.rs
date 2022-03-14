use cosmwasm_std::{testing::mock_info, MessageInfo};

use crate::{
    core::asset::AssetIdentifier,
    execute::validate_asset::{validate_asset, ValidateAssetV1},
    service::asset_meta_service::AssetMetaService,
    util::{aliases::ContractResponse, traits::OptionExtensions},
};

use super::{
    test_constants::{
        DEFAULT_ACCESS_ROUTE, DEFAULT_CONTRACT_BASE_NAME, DEFAULT_SCOPE_ADDRESS,
        DEFAULT_VALIDATOR_ADDRESS,
    },
    test_utilities::{intercept_add_attribute, MockOwnedDeps},
};

pub struct TestValidateAsset {
    pub info: MessageInfo,
    pub contract_base_name: String,
    pub validate_asset: ValidateAssetV1,
}
impl TestValidateAsset {
    pub fn default_validate_asset() -> ValidateAssetV1 {
        ValidateAssetV1 {
            identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
            success: true,
            message: "Validated asset without errors".to_string().to_some(),
            access_routes: vec![DEFAULT_ACCESS_ROUTE.to_string()],
        }
    }

    pub fn default_with_success(success: bool) -> Self {
        TestValidateAsset {
            validate_asset: ValidateAssetV1 {
                success,
                ..Self::default_validate_asset()
            },
            ..Self::default()
        }
    }
}
impl Default for TestValidateAsset {
    fn default() -> Self {
        Self {
            info: mock_info(DEFAULT_VALIDATOR_ADDRESS, &[]),
            contract_base_name: DEFAULT_CONTRACT_BASE_NAME.to_string(),
            validate_asset: TestValidateAsset::default_validate_asset(),
        }
    }
}

pub fn test_validate_asset(deps: &mut MockOwnedDeps, msg: TestValidateAsset) -> ContractResponse {
    let response = validate_asset(
        AssetMetaService::new(deps.as_mut()),
        msg.info,
        msg.validate_asset,
    );
    intercept_add_attribute(deps, &response, "failure occurred for test_validate_asset");
    response
}
