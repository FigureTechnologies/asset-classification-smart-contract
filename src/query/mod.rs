//! Contains the functionality used in the [contract file](crate::contract) to perform a contract query.

/// A query that fetches a target [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
/// from the contract's internal storage.
pub mod query_asset_definition;
/// A query that fetches all [AssetDefinitionV2s](crate::core::types::asset_definition::AssetDefinitionV2)
/// from the contract's internal storage.
pub mod query_asset_definitions;
/// A query that attempts to find all [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)s
/// on a Provenance Blockchain Metadata Scope that was added by this contract.
pub mod query_asset_scope_attribute;
/// A query that attempts to find an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
/// for a specific asset type on a Provenance Blockchain Metadata Scope that was added by this contract.
pub mod query_asset_scope_attribute_by_asset_type;
/// A query that attempts to find a [FeePaymentDetail](crate::core::types::fee_payment_detail::FeePaymentDetail)
/// stored for an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
/// that has not yet finished its asset verification step.
pub mod query_fee_payments;
/// A query that directly returns the contract's stored [StateV2](crate::core::state::StateV2) value.
pub mod query_state;
/// A query that directly returns the contract's stored [VersionInfoV1](crate::migrate::version_info::VersionInfoV1)
/// value.
pub mod query_version;
