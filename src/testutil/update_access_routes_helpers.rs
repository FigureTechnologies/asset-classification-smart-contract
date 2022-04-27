use crate::core::types::access_route::AccessRoute;
use crate::core::types::asset_identifier::AssetIdentifier;
use crate::execute::update_access_routes::{update_access_routes, UpdateAccessRoutesV1};
use crate::service::asset_meta_service::AssetMetaService;
use crate::testutil::test_constants::{
    DEFAULT_ADMIN_ADDRESS, DEFAULT_SCOPE_ADDRESS, DEFAULT_SENDER_ADDRESS,
};
use crate::testutil::test_utilities::{empty_mock_info, intercept_add_attribute, MockOwnedDeps};
use crate::util::aliases::EntryPointResponse;
use crate::util::traits::OptionExtensions;
use cosmwasm_std::testing::mock_info;
use cosmwasm_std::MessageInfo;

pub struct TestUpdateAccessRoutes {
    pub info: MessageInfo,
    pub update_access_routes: UpdateAccessRoutesV1,
}
impl TestUpdateAccessRoutes {
    pub fn default_update_access_routes() -> UpdateAccessRoutesV1 {
        UpdateAccessRoutesV1::new(
            AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
            DEFAULT_SENDER_ADDRESS,
            vec![AccessRoute::new(
                "http://updated.route:8080",
                "new-location".to_some(),
            )],
        )
    }
}
impl Default for TestUpdateAccessRoutes {
    fn default() -> Self {
        Self {
            info: empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            update_access_routes: TestUpdateAccessRoutes::default_update_access_routes(),
        }
    }
}

pub fn test_update_access_routes(
    deps: &mut MockOwnedDeps,
    msg: TestUpdateAccessRoutes,
) -> EntryPointResponse {
    let response = update_access_routes(
        AssetMetaService::new(deps.as_mut()),
        msg.info,
        msg.update_access_routes,
    );
    intercept_add_attribute(
        deps,
        &response,
        "failure occurred for test_update_access_routes",
    );
    response
}
