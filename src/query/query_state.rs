use cosmwasm_std::{to_binary, Binary};

use crate::{
    core::state::config_read,
    util::{
        aliases::{ContractResult, DepsC},
        traits::ResultExtensions,
    },
};

pub fn query_state(deps: &DepsC) -> ContractResult<Binary> {
    let state = config_read(deps.storage).load()?;
    to_binary(&state)?.to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::from_binary;
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::state::State,
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
            from_binary::<State>(&state_binary).expect("state should deserialize correctly");
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
    }
}
