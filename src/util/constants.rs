// Execution output attributes.  All should be prefixed with "asset_" to make them easy to
// discern when observed in the event stream

////////////////////////////////////////////
// Asset registration output attributes //
////////////////////////////////////////////

/// Value = Asset UUID (String)
pub const ASSET_REGISTERED_KEY: &str = "asset_registered";
/// Value = Scope ID Tied to the Asset (String)
pub const SCOPE_ID_KEY: &str = "asset_related_scope_id";

//////////////////////////////
// Shared output attributes //
//////////////////////////////

pub const ASSET_EVENT_TYPE_KEY: &str = "asset_event_type";
/// Value = Asset UUID (String)
pub const ASSET_SCOPE_ADDRESS_KEY: &str = "asset_scope_address";
/// Value = Asset Type (String)
pub const ASSET_TYPE_KEY: &str = "asset_type";
/// Value = The address of the validator associated with the asset (String)
pub const VALIDATOR_ADDRESS_KEY: &str = "asset_validator_address";
