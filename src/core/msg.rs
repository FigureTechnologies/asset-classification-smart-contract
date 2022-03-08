use crate::core::asset::{AssetDefinition, ValidatorDetail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Allows the user to optionally specify the enabled flag on an asset definition, versus forcing
/// it to be added manually on every request, when it will likely always be specified as `true`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetDefinitionInput {
    pub asset_type: String,
    pub scope_spec_address: String,
    pub validators: Vec<ValidatorDetail>,
    pub enabled: Option<bool>,
}
impl AssetDefinitionInput {
    pub fn new<S1: Into<String>, S2: Into<String>>(
        asset_type: S1,
        scope_spec_address: S2,
        validators: Vec<ValidatorDetail>,
        enabled: Option<bool>,
    ) -> AssetDefinitionInput {
        AssetDefinitionInput {
            asset_type: asset_type.into(),
            scope_spec_address: scope_spec_address.into(),
            validators,
            enabled,
        }
    }
}
impl From<AssetDefinition> for AssetDefinitionInput {
    fn from(def: AssetDefinition) -> Self {
        Self::new(
            def.asset_type,
            def.scope_spec_address,
            def.validators,
            Some(def.enabled),
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub base_contract_name: String,
    pub asset_definitions: Vec<AssetDefinitionInput>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    OnboardAsset {
        asset_uuid: Option<String>,
        asset_type: String,
        scope_address: Option<String>,
        validator_address: String,
    },
    ValidateAsset {
        asset_uuid: String,
        approve: bool,
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
    QueryAssetDefinition { asset_type: String },
    QueryState {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
