//! Contains the functionality used in the [contract file](crate::contract) to perform a contract migration.

/// The main entrypoint function for running a code migration.  Referred to in the [contract file](crate::contract).
pub mod migrate_contract;
// TODO: Remove after removing AssetDefinitionV1
pub mod migrate_to_asset_definition_v2;
/// Module for structs and helper functions containing the version information stored for contract
/// migrations.
pub mod version_info;
