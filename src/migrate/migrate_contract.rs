use cosmwasm_std::{Response, Storage};
use semver::Version;

use crate::{
    core::error::ContractError,
    util::{
        aliases::{AssetResult, DepsMutC, EntryPointResponse},
        event_attributes::{EventAttributes, EventType},
        traits::ResultExtensions,
    },
};

use super::version_info::{
    get_version_info, migrate_version_info, CONTRACT_NAME, CONTRACT_VERSION,
};

pub fn migrate_contract(deps: DepsMutC) -> EntryPointResponse {
    // Ensure the migration is not attempting to revert to an old version or something crazier
    check_valid_migration_versioning(deps.storage)?;
    // Store the new version info
    let new_version_info = migrate_version_info(deps.storage)?;
    Response::new()
        .add_attributes(
            EventAttributes::new(EventType::MigrateContract)
                .set_new_value(&new_version_info.version),
        )
        .to_ok()
}

/// Verifies that the migration is going to a proper version and the contract name of the new wasm matches
fn check_valid_migration_versioning(storage: &mut dyn Storage) -> AssetResult<()> {
    let stored_version_info = get_version_info(storage)?;
    // If the contract name has changed or another contract attempts to overwrite this one, this
    // check will reject the change
    if CONTRACT_NAME != stored_version_info.contract {
        return ContractError::InvalidContractName {
            current_contract: stored_version_info.contract,
            migration_contract: CONTRACT_NAME.to_string(),
        }
        .to_err();
    }
    let contract_version = CONTRACT_VERSION.parse::<Version>()?;
    // If the stored version in the contract is greater than the derived version from the package,
    // then this migration is effectively a downgrade and should not be committed
    if stored_version_info.parse_sem_ver()? > contract_version {
        return ContractError::InvalidContractVersion {
            current_version: stored_version_info.version,
            migration_version: CONTRACT_VERSION.to_string(),
        }
        .to_err();
    }
    Ok(())
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use provwasm_mocks::mock_dependencies;

    use crate::{
        migrate::version_info::{set_version_info, VersionInfoV1},
        testutil::test_utilities::single_attribute_for_key,
        util::constants::{ASSET_EVENT_TYPE_KEY, NEW_VALUE_KEY},
    };

    use super::*;

    #[test]
    fn test_successful_migration() {
        let mut deps = mock_dependencies(&[]);
        set_version_info(
            deps.as_mut().storage,
            &VersionInfoV1 {
                contract: CONTRACT_NAME.to_string(),
                version: "0.0.0".to_string(),
            },
        )
        .expect("setting the initial version info should not fail");
        let response = migrate_contract(deps.as_mut()).expect(
            "a migration should be successful when the contract is migrating to a new version",
        );
        assert!(
            response.messages.is_empty(),
            "a migration should not produce messages, and they would be ignored"
        );
        assert_eq!(
            2,
            response.attributes.len(),
            "the migration should produce the correct number of attributes",
        );
        assert_eq!(
            EventType::MigrateContract.event_name().as_str(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "the proper event type attribute should be emitted",
        );
        assert_eq!(
            CONTRACT_VERSION,
            single_attribute_for_key(&response, NEW_VALUE_KEY),
            "the new value key should equate to the current contract version",
        );
    }

    #[test]
    fn test_failed_migration_for_incorrect_name() {
        let mut deps = mock_dependencies(&[]);
        set_version_info(
            deps.as_mut().storage,
            &VersionInfoV1 {
                contract: "Wrong name".to_string(),
                version: CONTRACT_VERSION.to_string(),
            },
        )
        .unwrap();
        let error = migrate_contract(deps.as_mut()).unwrap_err();
        match error {
            ContractError::InvalidContractName {
                current_contract,
                migration_contract,
            } => {
                assert_eq!(
                    "Wrong name",
                    current_contract.as_str(),
                    "the current contract name should equate to the value stored in contract storage",
                );
                assert_eq!(
                    CONTRACT_NAME,
                    migration_contract.as_str(),
                    "the migration contract should be the env contract name",
                );
            }
            _ => panic!("unexpected error encountered: {:?}", error),
        };
    }

    #[test]
    fn test_failed_migration_for_too_low_version() {
        let mut deps = mock_dependencies(&[]);
        set_version_info(
            deps.as_mut().storage,
            &VersionInfoV1 {
                contract: CONTRACT_NAME.to_string(),
                version: "99.9.9".to_string(),
            },
        )
        .unwrap();
        let error = migrate_contract(deps.as_mut()).unwrap_err();
        match error {
            ContractError::InvalidContractVersion {
                current_version,
                migration_version,
            } => {
                assert_eq!(
                    "99.9.9",
                    current_version.as_str(),
                    "the current contract version should equate to the value stored in contract storage",
                );
                assert_eq!(
                    CONTRACT_VERSION,
                    migration_version.as_str(),
                    "the migration contract version should equate to the env value",
                );
            }
            _ => panic!("unexpected error encountered: {:?}", error),
        };
    }
}
