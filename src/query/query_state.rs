use cosmwasm_std::{to_binary, Binary};
use result_extensions::ResultExtensions;

use crate::{
    core::state::config_read_v2,
    util::aliases::{AssetResult, DepsC},
};

/// A query that directly returns the contract's stored [StateV2](crate::core::state::StateV2) value.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
pub fn query_state(deps: &DepsC) -> AssetResult<Binary> {
    let state = config_read_v2(deps.storage).load()?;
    to_binary(&state)?.to_ok()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::from_binary;
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::state::StateV2,
        testutil::{
            test_constants::{DEFAULT_ADMIN_ADDRESS, DEFAULT_CONTRACT_BASE_NAME},
            test_utilities::{test_instantiate_success, InstArgs},
        },
    };

    use super::*;

    #[test]
    fn test_successful_query_state() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let state_binary = query_state(&deps.as_ref()).expect("state query should return properly");
        let state =
            from_binary::<StateV2>(&state_binary).expect("state should deserialize correctly");
        assert_eq!(
            DEFAULT_CONTRACT_BASE_NAME,
            state.base_contract_name.as_str(),
            "the base contract name in the state should be the default value after default instantiation",
        );
        assert_eq!(
            DEFAULT_ADMIN_ADDRESS,
            state.admin.as_str(),
            "the default info name should be tagged as the admin address after default instantiation",
        );
        assert!(!state.is_test, "the default is_test value should be false");
    }
}
