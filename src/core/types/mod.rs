//! Contains all structs used to drive core functionality throughout the contract.

/// Defines a collection of [AccessRoute](self::access_route::AccessRoute) for a specific address.
pub mod access_definition;
/// Defines a method of obtaining underlying asset data for a scope.
pub mod access_route;
/// Defines a specific asset type associated with the contract.  Allows its specified type to be onboarded and verified.
pub mod asset_definition;
pub mod asset_identifier;
pub mod asset_onboarding_status;
pub mod asset_qualifier;
pub mod asset_scope_attribute;
pub mod asset_verification_result;
pub mod entity_detail;
pub mod fee_destination;
pub mod scope_spec_identifier;
pub mod serialized_enum;
pub mod verifier_detail;
