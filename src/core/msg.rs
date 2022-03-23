use crate::core::asset::VerifierDetail;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::asset::{AssetDefinitionInput, AssetIdentifier, AssetQualifier};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    pub base_contract_name: String,
    pub bind_base_name: bool,
    pub asset_definitions: Vec<AssetDefinitionInput>,
    pub is_test: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    OnboardAsset {
        identifier: AssetIdentifier,
        asset_type: String,
        verifier_address: String,
        access_routes: Option<Vec<String>>,
    },
    VerifyAsset {
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
    AddAssetVerifier {
        asset_type: String,
        verifier: VerifierDetail,
    },
    UpdateAssetVerifier {
        asset_type: String,
        verifier: VerifierDetail,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryAssetDefinition { qualifier: AssetQualifier },
    QueryAssetScopeAttribute { identifier: AssetIdentifier },
    QueryState {},
    QueryVersion {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {
    ContractUpgrade {},
    // TODO: Remove this migration path once all instances of State are replaced with StateV2
    MigrateToStateV2 { is_test: Option<bool> },
}
