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

/// Defines a specific asset type associated with the contract.  Allows its specified type to be
/// onboarded and verified.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetDefinition {
    /// The name of the asset associated with the definition.  This value must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    pub asset_type: String,
    /// A link to a scope specification that defines this asset type.  Must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    pub scope_spec_address: String,
    /// Individual verifier definitions.  Each value must have a unique `address` property or
    /// requests to add will be rejected.
    pub verifiers: Vec<VerifierDetail>,
    /// Indicates whether or not the asset definition is enabled for use in the contract.  If disabled,
    /// requests to onboard assets of this type will be rejected.
    pub enabled: bool,
}
impl AssetDefinition {
    /// Constructs a new instance of AssetDefinition, setting enabled to `true` by default.
    ///
    /// # Parameters
    ///
    /// *
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
