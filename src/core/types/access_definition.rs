use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::util::{
    aliases::AssetResult, scope_address_utils::bech32_string_to_addr, traits::ResultExtensions,
};

use super::access_route::AccessRoute;

/// Allows access definitions to be differentiated based on their overarching type, versus having to differentiate them based on known addresses
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessDefinitionType {
    /// Indicates that the access definition was created by the requestor that onboarded the scope
    Requestor,
    /// Indicates that the access definition was created by the verifier for a scope
    Verifier,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AccessDefinition {
    pub owner_address: String,
    pub access_routes: Vec<AccessRoute>,
    pub definition_type: AccessDefinitionType,
}

impl AccessDefinition {
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
