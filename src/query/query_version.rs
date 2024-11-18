use cosmwasm_std::{to_json_binary, Binary, Deps};
use result_extensions::ResultExtensions;

use crate::{migrate::version_info::get_version_info, util::aliases::AssetResult};

/// Pulls the version info for the contract out of the version store.
/// On a success, serializes the value to a cosmwasm Binary and responds with Ok.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
pub fn query_version(deps: &Deps) -> AssetResult<Binary> {
    to_json_binary(&get_version_info(deps.storage)?)?.to_ok()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::from_json;
    use provwasm_mocks::mock_provenance_dependencies;

    use crate::{
        migrate::version_info::{VersionInfoV1, CONTRACT_NAME, CONTRACT_VERSION},
        testutil::test_utilities::{test_instantiate_success, InstArgs},
    };

    use super::query_version;

    #[test]
    fn test_default_instantiate_and_fetch_version() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let version_bin = query_version(&deps.as_ref()).expect("failed to receive version info");
        let version_info = from_json::<VersionInfoV1>(&version_bin)
            .expect("failed to deserialize version info binary");
        // These values should always follow the env declared in Cargo.toml
        assert_eq!(
            CONTRACT_NAME, version_info.contract,
            "unexpected contract name value"
        );
        assert_eq!(
            CONTRACT_VERSION, version_info.version,
            "unexpected contract version value"
        );
    }
}
