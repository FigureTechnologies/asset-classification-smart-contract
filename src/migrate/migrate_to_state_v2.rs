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
