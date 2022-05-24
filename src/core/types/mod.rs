//! Contains all structs used to drive core functionality throughout the contract.

/// Defines a collection of [AccessRoute](self::access_route::AccessRoute) for a specific address.
pub mod access_definition;
/// Defines a method of obtaining underlying asset data for a scope.
pub mod access_route;
/// Defines a specific asset type associated with the contract.  Allows its specified type to be onboarded and verified.
pub mod asset_definition;
/// An enum containing interchangeable values that can be used to define an asset (uuid or address).
pub mod asset_identifier;
/// An enum that denotes the various states that an [AssetScopeAttribute](self::asset_scope_attribute::AssetScopeAttribute) can have.
pub mod asset_onboarding_status;
/// An enum containing different identifiers that can be used to fetch an [AssetDefinitionV2](self::asset_definition::AssetDefinitionV2).
pub mod asset_qualifier;
/// An asset scope attribute contains all relevant information for asset classification, and is serialized directly
/// as json into a Provenance Blockchain Attribute Module attribute on a Provenance Blockchain Metadata Scope.
pub mod asset_scope_attribute;
/// A simple wrapper for the result of a verification for a scope.
pub mod asset_verification_result;
/// Various fields describing an entity, which could be an organization, account, etc.
pub mod entity_detail;
/// Defines an external account designated as a recipient of funds during the verification process.
pub mod fee_destination;
/// An enum containing interchangeable values that can be used to define a Provenance Blockchain Metadata Scope Specification.
pub mod scope_spec_identifier;
/// A simple struct that allows a type and value to be translated to some of the optional enums in the contract:
/// [AssetIdentifier](self::asset_identifier::AssetIdentifier), [AssetQualifier](self::asset_qualifier::AssetQualifier), and [ScopeSpecIdentifier](self::scope_spec_identifier::ScopeSpecIdentifier).
pub mod serialized_enum;
/// Defines the fees and addresses for a single verifier account for an [AssetDefinitionV2](self::asset_definition::AssetDefinitionV2).
pub mod verifier_detail;
