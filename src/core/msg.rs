use crate::core::state::AssetDefinition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub base_contract_name: String,
    pub asset_definitions: Vec<AssetDefinition>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    OnboardAsset {
        asset_uuid: String,
        asset_type: String,
        scope_address: String,
        validator_address: String,
    },
    ValidateAsset {
        asset_uuid: String,
        approve: bool,
    },
    AddAssetDefinition {
        asset_definition: AssetDefinition,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryAssetDefinition { asset_type: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
