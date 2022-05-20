//! Contains the functionality used in the [contract file](crate::contract) to perform a contract query.

/// A query that fetches a target [AssetDefinition](crate::core::types::asset_definition::AssetDefinition)
/// from the contract's internal storage.
pub mod query_asset_definition;
/// A query that fetches all [AssetDefinitions](crate::core::types::asset_definition::AssetDefinition)
/// from the contract's internal storage.
pub mod query_asset_definitions;
/// A query that attempts to find an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
/// on a Provenance Blockchain Metadata Scope that was added by this contract.
pub mod query_asset_scope_attribute;
/// A query that directly returns the contract's stored [StateV2](crate::core::state::StateV2) value.
pub mod query_state;
/// A query that directly returns the contract's stored [VersionInfoV1](crate::migrate::version_info::VersionInfoV1)
/// value.
pub mod query_version;
