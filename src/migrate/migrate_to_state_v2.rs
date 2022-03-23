use crate::{
    core::state::{config_read, config_v2, StateV2},
    util::aliases::{DepsMutC, EntryPointResponse},
};

use super::migrate_contract::migrate_contract;

// TODO: Remove this entire file and mod.rs entry once all instances of State are replaced with StateV2
pub fn migrate_to_state_v2(deps: DepsMutC, is_test: Option<bool>) -> EntryPointResponse {
    // Place all existing values from state_v1 into state_v2, and then funnel into the standard migration code
    let state_v1 = config_read(deps.storage).load()?;
    let state_v2 = StateV2 {
        base_contract_name: state_v1.base_contract_name,
        admin: state_v1.admin,
        is_test: is_test.unwrap_or(false),
    };
    config_v2(deps.storage).save(&state_v2)?;
    migrate_contract(deps)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::state::{config, config_read_v2, State},
        migrate::version_info::migrate_version_info,
    };

    use super::migrate_to_state_v2;

    #[test]
    fn test_migration_is_test_some_true() {
        test_migration_to_state_v2(Some(true));
    }

    #[test]
    fn test_migration_is_test_some_false() {
        test_migration_to_state_v2(Some(false));
    }

    #[test]
    fn test_migration_is_test_none() {
        test_migration_to_state_v2(None);
    }

    fn test_migration_to_state_v2(is_test_input_value: Option<bool>) {
        let mut deps = mock_dependencies(&[]);
        // The migration won't run if version info doesn't yet exist.  It assumes the contract has been instantiated,
        // which, in this test, it isn't.  Comma splices forever!
        migrate_version_info(deps.as_mut().storage)
            .expect("expected version info to be setup correctly");
        let state_v1 = State {
            base_contract_name: "justassetthings.pb".to_string(),
            admin: Addr::unchecked("fakeadmin"),
        };
        config(deps.as_mut().storage)
            .save(&state_v1)
            .expect("save original State struct should succeed");
        migrate_to_state_v2(deps.as_mut(), is_test_input_value)
            .expect("migration should complete successfully");
        let state_v2 = config_read_v2(deps.as_ref().storage)
            .load()
            .expect("StateV2 should be saved after the migration completes");
        assert_eq!(
            state_v1.base_contract_name, state_v2.base_contract_name,
            "base contract name should be ported successfully",
        );
        assert_eq!(
            state_v1.admin, state_v2.admin,
            "admin address should be ported successfully",
        );
        let (expected_value, message) = if let Some(is_test) = is_test_input_value {
            (
                is_test,
                format!(
                    "expected is_test to be [{}] when the bool was provided explicitly",
                    is_test
                ),
            )
        } else {
            (
                false,
                "expected is_test to be [false] when no value was provided".to_string(),
            )
        };
        assert_eq!(expected_value, state_v2.is_test, "{}", message);
    }
}
