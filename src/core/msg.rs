use crate::core::asset::ValidatorDetail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::asset::{AssetDefinitionInput, AssetIdentifier, AssetQualifier};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    pub base_contract_name: String,
    pub asset_definitions: Vec<AssetDefinitionInput>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    OnboardAsset {
        identifier: AssetIdentifier,
        asset_type: String,
        validator_address: String,
        access_routes: Option<Vec<String>>,
    },
    ValidateAsset {
        identifier: AssetIdentifier,
        success: bool,
        message: Option<String>,
        access_routes: Option<Vec<String>>,
    },
    AddAssetDefinition {
        asset_definition: AssetDefinitionInput,
    },
    UpdateAssetDefinition {
        asset_definition: AssetDefinitionInput,
    },
    ToggleAssetDefinition {
        asset_type: String,
        expected_result: bool,
    },
    AddAssetValidator {
        asset_type: String,
        validator: ValidatorDetail,
    },
    UpdateAssetValidator {
        asset_type: String,
        validator: ValidatorDetail,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryAssetDefinition { qualifier: AssetQualifier },
    QueryAssetScopeAttribute { identifier: AssetIdentifier },
    QueryState {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
