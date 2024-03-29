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
/// An asset scope attribute contains all relevant information for asset classification, and is serialized directly
/// as json into a Provenance Blockchain Attribute Module attribute on a Provenance Blockchain Metadata Scope.
pub mod asset_scope_attribute;
/// A simple wrapper for the result of a verification for a scope.
pub mod asset_verification_result;
/// Various fields describing an entity, which could be an organization, account, etc.
pub mod entity_detail;
/// Defines an external account designated as a recipient of funds during the verification process.
pub mod fee_destination;
/// Defines a stored set of values for charging fees to the onboarding account during the asset
/// classification process.
pub mod fee_payment_detail;
/// A node that defines how much onboarding should cost and any specific fees that should be paid.
pub mod onboarding_cost;
/// A simple struct that allows a type and value to be translated to some of the optional enums in the contract:
/// [AssetIdentifier](self::asset_identifier::AssetIdentifier)
pub mod serialized_enum;
/// Defines fees and values that can be used when classification is being done on an asset for a
/// new type beyond the first.
pub mod subsequent_classification_detail;
/// Defines the fees and addresses for a single verifier account for an [AssetDefinitionV3](self::asset_definition::AssetDefinitionV3).
pub mod verifier_detail;
