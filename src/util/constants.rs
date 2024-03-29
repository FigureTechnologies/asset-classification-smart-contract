// Execution output attributes.  All should be prefixed with "asset_" to make them easy to
// discern when observed in the event stream

//////////////////////////////////////////
// Asset registration output attributes //
//////////////////////////////////////////

/// Value = Asset UUID (String).
pub const ASSET_REGISTERED_KEY: &str = "asset_registered";
/// Value = Scope ID Tied to the Asset (String).
pub const SCOPE_ID_KEY: &str = "asset_related_scope_id";
/// Value = The scope owner that sent the onboarding message.
pub const SCOPE_OWNER_KEY: &str = "asset_scope_owner_address";

//////////////////////////////
// Shared output attributes //
//////////////////////////////

/// Value = Event Type correlating to EvenType enum into String values (String).
pub const ASSET_EVENT_TYPE_KEY: &str = "asset_event_type";
/// Value = Asset UUID (String).
pub const ASSET_SCOPE_ADDRESS_KEY: &str = "asset_scope_address";
/// Value = Asset Type (String).
pub const ASSET_TYPE_KEY: &str = "asset_type";
/// Value = The address of the verifier associated with the asset (String).
pub const VERIFIER_ADDRESS_KEY: &str = "asset_verifier_address";
/// Value = The new [onboarding status](crate::core::types::asset_onboarding_status::AssetOnboardingStatus) of an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute) after performing an execute function.
pub const NEW_ASSET_ONBOARDING_STATUS_KEY: &str = "asset_onboarding_status";
/// Value = Any new value being changed that can be coerced to a string target. Dynamic to be used on various routes (String).
pub const NEW_VALUE_KEY: &str = "asset_new_value";
/// Value = EventAdditionalMetadata meta string.
pub const ADDITIONAL_METADATA_KEY: &str = "asset_additional_metadata";

//////////////////////
// Global Constants //
//////////////////////
/// A constant declaration to ensure the word "nhash" does not have typos when used throughout the
/// contract's source.
pub const NHASH: &str = "nhash";
/// All denominations of coin that are valid for a verifier detail to include in its [onboarding_denom](crate::core::types::verifier_detail::VerifierDetailV2::onboarding_denom)
/// field.
pub const VALID_VERIFIER_DENOMS: [&str; 1] = [NHASH];
