use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::types::serialized_enum::SerializedEnum;
use crate::{
    core::state::config_read_v2,
    util::{
        aliases::{AssetResult, DepsC},
        functions::generate_asset_attribute_name,
        traits::ResultExtensions,
    },
};

use super::verifier_detail::VerifierDetail;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetDefinition {
    pub asset_type: String,
    pub scope_spec_address: String,
    pub verifiers: Vec<VerifierDetail>,
    pub enabled: bool,
}
impl AssetDefinition {
    pub fn new<S1: Into<String>, S2: Into<String>>(
        asset_type: S1,
        scope_spec_address: S2,
        verifiers: Vec<VerifierDetail>,
    ) -> Self {
        AssetDefinition {
            asset_type: asset_type.into(),
            scope_spec_address: scope_spec_address.into(),
            verifiers,
            enabled: true,
        }
    }

    /// Converts the asset_type value to lowercase and serializes it as bytes,
    /// then uplifts the value to a vector to allow it to be returned.
    pub fn storage_key(&self) -> Vec<u8> {
        self.asset_type.to_lowercase().as_bytes().to_vec()
    }

    pub fn attribute_name(&self, deps: &DepsC) -> AssetResult<String> {
        let state = config_read_v2(deps.storage).load()?;
        generate_asset_attribute_name(&self.asset_type, state.base_contract_name).to_ok()
    }
}

/// Allows the user to optionally specify the enabled flag on an asset definition, versus forcing
/// it to be added manually on every request, when it will likely always be specified as `true`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetDefinitionInput {
    pub asset_type: String,
    pub scope_spec_identifier: SerializedEnum,
    pub verifiers: Vec<VerifierDetail>,
    pub enabled: Option<bool>,
    pub bind_name: Option<bool>,
}
impl AssetDefinitionInput {
    pub fn new<S1: Into<String>>(
        asset_type: S1,
        scope_spec_identifier: SerializedEnum,
        verifiers: Vec<VerifierDetail>,
        enabled: Option<bool>,
        bind_name: Option<bool>,
    ) -> AssetDefinitionInput {
        AssetDefinitionInput {
            asset_type: asset_type.into(),
            scope_spec_identifier,
            verifiers,
            enabled,
            bind_name,
        }
    }

    pub fn into_asset_definition(self) -> AssetResult<AssetDefinition> {
        AssetDefinition {
            asset_type: self.asset_type,
            scope_spec_address: self
                .scope_spec_identifier
                .to_scope_spec_identifier()?
                .get_scope_spec_address()?,
            verifiers: self.verifiers,
            enabled: self.enabled.unwrap_or(true),
        }
        .to_ok()
    }

    pub fn as_asset_definition(&self) -> AssetResult<AssetDefinition> {
        AssetDefinition::new(
            &self.asset_type,
            self.scope_spec_identifier
                .to_scope_spec_identifier()?
                .get_scope_spec_address()?,
            self.verifiers.clone(),
        )
        .to_ok()
    }
}
