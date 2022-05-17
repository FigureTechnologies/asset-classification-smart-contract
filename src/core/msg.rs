use crate::core::types::serialized_enum::SerializedEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::types::{
    access_route::AccessRoute, asset_definition::AssetDefinitionInput,
    verifier_detail::VerifierDetail,
};

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
        /// Expects an AssetIdentifier-compatible SerializedEnum
        identifier: SerializedEnum,
        asset_type: String,
        verifier_address: String,
        access_routes: Option<Vec<AccessRoute>>,
    },
    VerifyAsset {
        /// Expects an AssetIdentifier-compatible SerializedEnum
        identifier: SerializedEnum,
        success: bool,
        message: Option<String>,
        access_routes: Option<Vec<AccessRoute>>,
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
    UpdateAccessRoutes {
        /// Expects an AssetIdentifier-compatible SerializedEnum
        identifier: SerializedEnum,
        owner_address: String,
        access_routes: Vec<AccessRoute>,
    },
    BindContractAlias {
        alias_name: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryAssetDefinition {
        /// Expects an AssetQualifier-compatible SerializedEnum
        qualifier: SerializedEnum,
    },
    QueryAssetDefinitions {},
    QueryAssetScopeAttribute {
        /// Expects an AssetIdentifier-compatible SerializedEnum
        identifier: SerializedEnum,
    },
    QueryState {},
    QueryVersion {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {
    ContractUpgrade {},
}
