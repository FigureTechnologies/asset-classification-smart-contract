//! Contains the functionality used in the [contract file](crate::contract) to perform a contract migration.

/// The main entrypoint function for running a code migration.  Referred to in the [contract file](crate::contract).
pub mod migrate_contract;
/// Module for structs and helper functions containing the version information stored for contract
/// migrations.
pub mod version_info;
