use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::util::{
    aliases::AssetResult, scope_address_utils::bech32_string_to_addr, traits::ResultExtensions,
};

use super::access_route::AccessRoute;

/// Allows access definitions to be differentiated based on their overarching type, versus having to differentiate them based on known addresses.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessDefinitionType {
    /// Indicates that the access definition was created by the requestor that onboarded the scope
    Requestor,
    /// Indicates that the access definition was created by the verifier for a scope
    Verifier,
}

/// Defines a collection of [AccessRoute](super::access_route::AccessRoute) for a specific address.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AccessDefinition {
    /// The bech32 address of the account that created the underlying [AccessRoutes](super::access_route::AccessRoute).
    pub owner_address: String,
    /// A collection of [AccessRoute](super::access_route::AccessRoute) structs that define methods of
    /// obtaining the underlying data for a scope.
    pub access_routes: Vec<AccessRoute>,
    /// Defines the source that created this definition.
    pub definition_type: AccessDefinitionType,
}

impl AccessDefinition {
    /// Constructs a new instance of this struct, ensuring that the provided `owner_address` is a
    /// valid Provenance Blockchain bech32 address.
    pub fn new_checked<S1: Into<String>>(
        owner_address: S1,
        access_routes: Vec<AccessRoute>,
        definition_type: AccessDefinitionType,
    ) -> AssetResult<Self> {
        Self {
            owner_address: bech32_string_to_addr(owner_address)?.into_string(),
            access_routes,
            definition_type,
        }
        .to_ok()
    }
}
