use cosmwasm_std::Deps;
use result_extensions::ResultExtensions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::error::ContractError;
use crate::core::state::{StateV2, STATE_V2};
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::util::{aliases::AssetResult, functions::generate_asset_attribute_name};

/// Defines a specific asset type associated with the contract.  Allows its specified type to be
/// onboarded and verified.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetDefinitionV3 {
    /// The unique name of the asset associated with the definition.
    pub asset_type: String,
    /// A pretty human-readable name for this asset type (vs a typically snake_case asset_type name)
    pub display_name: Option<String>,
    /// Individual verifier definitions.  There can be many verifiers for a single asset type.
    pub verifiers: Vec<VerifierDetailV2>,
    /// Indicates whether or not the asset definition is enabled for use in the contract.  If disabled,
    /// requests to onboard assets of this type will be rejected.
    pub enabled: bool,
}
impl AssetDefinitionV3 {
    /// Constructs a new instance of AssetDefinitionV3, setting enabled to `true` by default.
    ///
    /// # Parameters
    ///
    /// * `asset_type` The unique name of the asset associated with the definition.
    /// * `verifiers` Individual verifier definitions.
    pub fn new<S1: Into<String>, S2: Into<String>>(
        asset_type: S1,
        display_name: Option<S2>,
        verifiers: Vec<VerifierDetailV2>,
    ) -> Self {
        AssetDefinitionV3 {
            asset_type: asset_type.into(),
            display_name: display_name.map(|n| n.into()),
            verifiers,
            enabled: true,
        }
    }

    /// Converts the asset_type value to lowercase and serializes it as bytes,
    /// then uplifts the value to a vector to allow it to be returned.
    pub fn storage_key(&self) -> String {
        self.asset_type.to_lowercase()
    }

    /// Helper functionality to retrieve the base contract name from state and use it to create the
    /// Provenance Blockchain Attribute Module name for this asset type.
    ///
    /// # Parameters
    ///
    /// * `deps` A read-only instance of the cosmwasm-provided Deps value.
    pub fn attribute_name(&self, deps: &Deps) -> AssetResult<String> {
        let state = STATE_V2.load(deps.storage)?;
        self.attribute_name_state(&state).to_ok()
    }

    /// Helper functionality to use the base contract name from already fetched state and use it to create the
    /// Provenance Blockchain Attribute Module name for this asset type.
    ///
    /// # Parameters
    ///
    /// * `deps` A read-only instance of the cosmwasm-provided Deps value.
    pub fn attribute_name_state(&self, state: &StateV2) -> String {
        generate_asset_attribute_name(&self.asset_type, &state.base_contract_name)
    }

    /// Helper functionality to retrieve a verifier detail from the self-contained vector of
    /// verifiers by matching on the given address.
    ///
    /// # Parameters
    ///
    /// * `verifier_address` The bech32 address of the verifier to locate within the verifiers
    /// vector.
    pub fn get_verifier_detail<S: Into<String>>(
        &self,
        verifier_address: S,
    ) -> Result<VerifierDetailV2, ContractError> {
        let verifier_address = verifier_address.into();
        match self
            .verifiers
            .iter()
            .find(|verifier| verifier.address == verifier_address)
        {
            Some(verifier) => verifier.to_owned().to_ok(),
            None => ContractError::UnsupportedVerifier {
                asset_type: self.asset_type.to_owned(),
                verifier_address,
            }
            .to_err(),
        }
    }
}

/// Allows the user to optionally specify the enabled flag on an asset definition, versus forcing
/// it to be added manually on every request, when it will likely always be specified as `true`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetDefinitionInputV3 {
    /// The name of the asset associated with the definition.  This value must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    pub asset_type: String,
    /// A pretty human-readable name for this asset type (vs a typically snake_case asset_type name)
    pub display_name: Option<String>,
    /// Individual verifier definitions.  There can be many verifiers for a single asset type.  Each
    /// value must have a unique `address` property or requests to add will be rejected.
    pub verifiers: Vec<VerifierDetailV2>,
    /// Indicates whether or not the asset definition is enabled for use in the contract.  If disabled,
    /// requests to onboard assets of this type will be rejected.
    pub enabled: Option<bool>,
    /// Whether or not to bind a Provenance Blockchain Name Module name to this contract when this
    /// struct is used to add a new asset type to the contract.  If this value is omitted OR set to
    /// true in a request that adds an asset definition, the name derived by combining the
    /// [base_contract_name](crate::core::state::StateV2::base_contract_name) and the `asset_type`
    /// will be bound to the contract.  For example, if the base name is "pb" and the asset type is
    /// "myasset," the resulting bound name would be "myasset.pb".
    pub bind_name: Option<bool>,
}
impl AssetDefinitionInputV3 {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `asset_type` The name of the asset associated with the definition.  This value must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    /// * `verifiers` Individual verifier definitions.  There can be many verifiers for a single asset type.  Each
    /// value must have a unique `address` property or requests to add will be rejected.
    /// * `enabled` Indicates whether or not the asset definition is enabled for use in the contract.  If disabled,
    /// requests to onboard assets of this type will be rejected.
    /// * `bind_name` Whether or not to bind a Provenance Blockchain Name Module name to this contract when this
    /// struct is used to add a new asset type to the contract.
    pub fn new<S1: Into<String>, S2: Into<String>>(
        asset_type: S1,
        display_name: Option<S2>,
        verifiers: Vec<VerifierDetailV2>,
        enabled: Option<bool>,
        bind_name: Option<bool>,
    ) -> Self {
        Self {
            asset_type: asset_type.into(),
            display_name: display_name.map(|n| n.into()),
            verifiers,
            enabled,
            bind_name,
        }
    }

    /// Moves this struct into an instance of [AssetDefinitionV3](self::AssetDefinitionV3)
    pub fn into_asset_definition(self) -> AssetDefinitionV3 {
        AssetDefinitionV3 {
            asset_type: self.asset_type,
            display_name: self.display_name,
            verifiers: self.verifiers,
            enabled: self.enabled.unwrap_or(true),
        }
    }

    /// Clones the values contained within this struct into an instance of [AssetDefinitionV3](self::AssetDefinitionV3).
    /// This process is more expensive than moving the struct with [into_asset_definition](self::AssetDefinitionInputV3::into_asset_definition).
    pub fn as_asset_definition(&self) -> AssetDefinitionV3 {
        AssetDefinitionV3 {
            asset_type: self.asset_type.clone(),
            display_name: self.display_name.clone(),
            verifiers: self.verifiers.clone(),
            enabled: self.enabled.unwrap_or(true),
        }
    }
}
