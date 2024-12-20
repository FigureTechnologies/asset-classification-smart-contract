use crate::{core::error::ContractError, util::aliases::AssetResult};
use cosmwasm_std::Storage;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};

// When cargo is building this project, it automatically adds these env vars for the code to infer.
// See Cargo.toml's name and version fields in the [package] section for the values.
// NOTE: The program verifies that migrated versions match the contract name and have a greater
// version than that which was previous stored. Ensure to update the version field on each release
// before migrating, because it's important to be able to differentiate versions as they're applied.
/// Automatically derived from the Cargo.toml's name property.
pub const CONTRACT_NAME: &str = env!("CARGO_CRATE_NAME");
/// Automatically derived from the Cargo.toml's version property.
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const VERSION_INFO_NAMESPACE: &str = "version_info";
const VERSION_INFO: Item<VersionInfoV1> = Item::new(VERSION_INFO_NAMESPACE);

/// Holds both the contract's unique name and version.
/// Used to ensure that migrations have the correct targets and are not downgrades.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct VersionInfoV1 {
    /// The name of the contract, set to the value of [CONTRACT_NAME](self::CONTRACT_NAME) during instantiation and
    /// following migrations.
    pub contract: String,
    /// The version of the contract, set to the value of [CONTRACT_VERSION](self::CONTRACT_VERSION) during
    /// instantiation and following migrations.
    pub version: String,
}
impl VersionInfoV1 {
    pub fn parse_sem_ver(&self) -> Result<Version, semver::Error> {
        self.version.parse()
    }
}

/// Sets the contract's version definition directly to the specified [VersionInfoV1](self::VersionInfoV1) struct.
///
/// # Parameters
///
/// * `storage` A mutable instance of the contract's internal storage.
/// * `version_info` A struct defining the name and version of the current contract instance.
pub fn set_version_info(
    storage: &mut dyn Storage,
    version_info: &VersionInfoV1,
) -> AssetResult<()> {
    VERSION_INFO
        .save(storage, version_info)
        .map_err(ContractError::Std)
}

/// Fetches, if possible, the current version information for the contract.
///
/// # Parameters
///
/// * `storage` A read-only instance of the contract's internal storage.
pub fn get_version_info(storage: &dyn Storage) -> AssetResult<VersionInfoV1> {
    VERSION_INFO.load(storage).map_err(ContractError::Std)
}

/// Sets the version info for the given contract to the derived values from the Cargo.toml file.
///
/// # Parameters
///
/// * `storage` A mutable instance of the contract's internal storage.
pub fn migrate_version_info(storage: &mut dyn Storage) -> AssetResult<VersionInfoV1> {
    let version_info = VersionInfoV1 {
        contract: CONTRACT_NAME.to_string(),
        version: CONTRACT_VERSION.to_string(),
    };
    set_version_info(storage, &version_info)?;
    Ok(version_info)
}

#[cfg(test)]
mod tests {
    use provwasm_mocks::mock_provenance_dependencies;

    use crate::migrate::version_info::{
        get_version_info, migrate_version_info, set_version_info, VersionInfoV1, CONTRACT_NAME,
        CONTRACT_VERSION,
    };

    #[test]
    fn test_set_and_get_version_info() {
        let mut deps = mock_provenance_dependencies();
        let result = set_version_info(
            &mut deps.storage,
            &VersionInfoV1 {
                contract: "CONTRACT".into(),
                version: "4.2.0".into(),
            },
        );
        assert!(result.is_ok(), "storage should succeed");
        let info = get_version_info(&deps.storage).unwrap();
        assert_eq!(
            "CONTRACT",
            info.contract.as_str(),
            "the proper contract name should be returned"
        );
        assert_eq!(
            "4.2.0",
            info.version.as_str(),
            "the proper contract version should be returned"
        );
    }

    #[test]
    fn test_migrate_version_info() {
        let mut deps = mock_provenance_dependencies();
        let migrate_result = migrate_version_info(&mut deps.storage).unwrap();
        assert_eq!(
            CONTRACT_NAME,
            migrate_result.contract.as_str(),
            "expected the returned version info to have the declared contract name",
        );
        assert_eq!(
            CONTRACT_VERSION,
            migrate_result.version.as_str(),
            "expected the returned version info to have the declared contract version",
        );
        let stored_info = get_version_info(&deps.storage).unwrap();
        assert_eq!(
            migrate_result.contract.as_str(),
            stored_info.contract.as_str(),
            "expected the stored value for contract name to be the same as the value returned from the migrate function",
        );
        assert_eq!(
            migrate_result.version.as_str(),
            stored_info.version.as_str(),
            "expected the stored value for version number to be the same as the value returned from the migration function",
        );
    }
}
