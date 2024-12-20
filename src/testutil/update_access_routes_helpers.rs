use crate::core::types::access_route::AccessRoute;
use crate::core::types::asset_identifier::AssetIdentifier;
use crate::execute::update_access_routes::{update_access_routes, UpdateAccessRoutesV1};
use crate::service::asset_meta_service::AssetMetaService;
use crate::testutil::test_constants::{
    DEFAULT_ADMIN_ADDRESS, DEFAULT_ASSET_TYPE, DEFAULT_SCOPE_ADDRESS, DEFAULT_SENDER_ADDRESS,
};
use crate::testutil::test_utilities::{
    empty_mock_info, intercept_add_or_update_attribute, MockOwnedDeps,
};
use crate::util::aliases::EntryPointResponse;
use crate::util::traits::OptionExtensions;
use cosmwasm_std::MessageInfo;

pub struct TestUpdateAccessRoutes {
    pub info: MessageInfo,
    pub update_access_routes: UpdateAccessRoutesV1,
}
impl TestUpdateAccessRoutes {
    pub fn default_update_access_routes() -> UpdateAccessRoutesV1 {
        UpdateAccessRoutesV1::new(
            AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
            DEFAULT_ASSET_TYPE,
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
    env: &cosmwasm_std::Env,
    msg: TestUpdateAccessRoutes,
) -> EntryPointResponse {
    update_access_routes(
        env,
        AssetMetaService::new(deps.as_mut()),
        msg.info,
        msg.update_access_routes,
    )
    .and_then(|response| {
        intercept_add_or_update_attribute(
            deps,
            response,
            "failure occurred for test_update_access_routes",
        )
    })
}
