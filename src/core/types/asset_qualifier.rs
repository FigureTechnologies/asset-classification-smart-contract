use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum AssetQualifier {
    AssetType(String),
    ScopeSpecAddress(String),
}
impl AssetQualifier {
    pub fn asset_type<S: Into<String>>(asset_type: S) -> Self {
        Self::AssetType(asset_type.into())
    }

    pub fn scope_spec_address<S: Into<String>>(scope_spec_address: S) -> Self {
        Self::ScopeSpecAddress(scope_spec_address.into())
    }
}
