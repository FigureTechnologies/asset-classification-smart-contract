use crate::core::types::asset_identifier::AssetIdentifier;
use crate::execute::finalize_classification::{finalize_classification, FinalizeClassificationV1};
use crate::service::asset_meta_service::AssetMetaService;
use crate::testutil::test_constants::{DEFAULT_SCOPE_ADDRESS, DEFAULT_SENDER_ADDRESS};
use crate::testutil::test_utilities::{
    empty_mock_info, intercept_add_or_update_attribute, MockOwnedDeps,
};
use crate::util::aliases::EntryPointResponse;
use cosmwasm_std::testing::mock_env;
use cosmwasm_std::MessageInfo;

pub struct TestFinalizeClassification {
    pub info: MessageInfo,
    pub finalize_classification: FinalizeClassificationV1,
}
impl TestFinalizeClassification {
    pub fn default_finalize_classification() -> FinalizeClassificationV1 {
        FinalizeClassificationV1 {
            identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
        }
    }
}
impl Default for TestFinalizeClassification {
    fn default() -> Self {
        Self {
            info: empty_mock_info(DEFAULT_SENDER_ADDRESS),
            finalize_classification: Self::default_finalize_classification(),
        }
    }
}

pub fn test_finalize_classification(
    deps: &mut MockOwnedDeps,
    msg: TestFinalizeClassification,
) -> EntryPointResponse {
    let response = finalize_classification(
        AssetMetaService::new(deps.as_mut()),
        mock_env(),
        msg.info,
        msg.finalize_classification,
    );
    // Allow errors to be returned when they occur, but also intercept the response's attribute
    // writes if they occur
    if response.is_ok() {
        intercept_add_or_update_attribute(
            deps,
            &response,
            "failure occurred for test_finalize_classification",
        );
    }
    response
}
