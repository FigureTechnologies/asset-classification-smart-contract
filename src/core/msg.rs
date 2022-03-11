use crate::{
    core::asset::{AssetDefinition, ValidatorDetail},
    util::{
        aliases::ContractResult,
        scope_address_utils::{
            asset_uuid_to_scope_address, scope_address_to_asset_uuid,
            scope_spec_address_to_scope_spec_uuid, scope_spec_uuid_to_scope_spec_address,
        },
        traits::{OptionExtensions, ResultExtensions},
    },
};
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
            def.enabled.to_some(),
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
pub enum AssetIdentifier {
    AssetUuid { asset_uuid: String },
    ScopeAddress { scope_address: String },
}
impl AssetIdentifier {
    pub fn asset_uuid<S: Into<String>>(asset_uuid: S) -> Self {
        Self::AssetUuid {
            asset_uuid: asset_uuid.into(),
        }
    }

    pub fn scope_address<S: Into<String>>(scope_address: S) -> Self {
        Self::ScopeAddress {
            scope_address: scope_address.into(),
        }
    }

    pub fn get_asset_uuid(&self) -> ContractResult<String> {
        match self {
            Self::AssetUuid { asset_uuid } => (*asset_uuid).clone().to_ok(),
            Self::ScopeAddress { scope_address } => scope_address_to_asset_uuid(scope_address),
        }
    }

    pub fn get_scope_address(&self) -> ContractResult<String> {
        match self {
            Self::AssetUuid { asset_uuid } => asset_uuid_to_scope_address(asset_uuid),
            Self::ScopeAddress { scope_address } => (*scope_address).clone().to_ok(),
        }
    }

    /// Takes the value provided and derives both values from it, where necessary,
    /// ensuring that both asset_uuid and scope_address are available to the user
    pub fn to_identifiers(&self) -> ContractResult<AssetIdentifiers> {
        AssetIdentifiers::new(self.get_asset_uuid()?, self.get_scope_address()?).to_ok()
    }
}

/// A simple named collection of both the asset uuid and scope address
pub struct AssetIdentifiers {
    pub asset_uuid: String,
    pub scope_address: String,
}
impl AssetIdentifiers {
    pub fn new<S1: Into<String>, S2: Into<String>>(asset_uuid: S1, scope_address: S2) -> Self {
        Self {
            asset_uuid: asset_uuid.into(),
            scope_address: scope_address.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetQualifier {
    AssetType { asset_type: String },
    ScopeSpecAddress { scope_spec_address: String },
}
impl AssetQualifier {
    pub fn asset_type<S: Into<String>>(asset_type: S) -> Self {
        Self::AssetType {
            asset_type: asset_type.into(),
        }
    }

    pub fn scope_spec_address<S: Into<String>>(scope_spec_address: S) -> Self {
        Self::ScopeSpecAddress {
            scope_spec_address: scope_spec_address.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScopeSpecIdentifier {
    ScopeSpecUuid { scope_spec_uuid: String },
    ScopeSpecAddress { scope_spec_address: String },
}
impl ScopeSpecIdentifier {
    pub fn uuid<S: Into<String>>(scope_spec_uuid: S) -> Self {
        Self::ScopeSpecUuid {
            scope_spec_uuid: scope_spec_uuid.into(),
        }
    }

    pub fn address<S: Into<String>>(scope_spec_address: S) -> Self {
        Self::ScopeSpecAddress {
            scope_spec_address: scope_spec_address.into(),
        }
    }

    /// Takes the value provided and dervies both values from it, where necessary,
    /// ensuring that both scope_spec_uuid and scope_spec_address are available to the user
    pub fn to_identifiers(&self) -> ContractResult<ScopeSpecIdentifiers> {
        match self {
            Self::ScopeSpecUuid { scope_spec_uuid } => ScopeSpecIdentifiers::new(
                scope_spec_uuid,
                scope_spec_uuid_to_scope_spec_address(scope_spec_uuid)?,
            )
            .to_ok(),
            Self::ScopeSpecAddress { scope_spec_address } => ScopeSpecIdentifiers::new(
                scope_spec_address_to_scope_spec_uuid(scope_spec_address)?,
                scope_spec_address,
            )
            .to_ok(),
        }
    }
}

pub struct ScopeSpecIdentifiers {
    pub scope_spec_uuid: String,
    pub scope_spec_address: String,
}
impl ScopeSpecIdentifiers {
    pub fn new<S1, S2>(scope_spec_uuid: S1, scope_spec_address: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            scope_spec_uuid: scope_spec_uuid.into(),
            scope_spec_address: scope_spec_address.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    OnboardAsset {
        identifier: AssetIdentifier,
        asset_type: String,
        validator_address: String,
    },
    ValidateAsset {
        identifier: AssetIdentifier,
        success: bool,
        message: Option<String>,
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

#[cfg(test)]
mod tests {
    use crate::core::msg::AssetIdentifier;

    #[test]
    fn test_asset_identifier_parse_for_asset_uuid() {
        // The uuid was generated randomly and the scope address was derived via provenance's MetadataAddress util
        let asset_uuid = "0c39efb6-9fef-11ec-ab21-6bf5c9fb3f83";
        let expected_scope_address = "scope1qqxrnmaknlh3rm9ty94ltj0m87psnapt5l";
        let identifier = AssetIdentifier::asset_uuid(asset_uuid);
        let result_identifiers = identifier
            .to_identifiers()
            .expect("parsing idenitifiers should succeed");
        assert_eq!(
            asset_uuid,
            result_identifiers.asset_uuid.as_str(),
            "expected the asset uuid value to pass through successfully",
        );
        assert_eq!(
            expected_scope_address,
            result_identifiers.scope_address.as_str(),
            "expected the scope address to be derived correctly",
        );
    }

    #[test]
    fn test_asset_identifier_parse_for_scope_address() {
        // The uuid was generated randomly and the scope address was derived via provenance's MetadataAddress util
        let scope_address = "scope1qz3s7dvsnlh3rmyy3pm5tszs2v7qhwhde8";
        let expected_asset_uuid = "a30f3590-9fef-11ec-8488-7745c050533c";
        let identifier = AssetIdentifier::scope_address(scope_address);
        let result_identifiers = identifier
            .to_identifiers()
            .expect("parsing identifiers should succeed");
        assert_eq!(
            scope_address,
            result_identifiers.scope_address.as_str(),
            "expected the scope address to pass through successfully",
        );
        assert_eq!(
            expected_asset_uuid,
            result_identifiers.asset_uuid.as_str(),
            "expected the asset uuid to be derived correctly",
        );
    }

    #[test]
    fn test_asset_identifier_to_functions_from_asset_uuid() {
        let initial_uuid = "5134f836-a15c-11ec-abb6-a733aad66af8";
        let expected_scope_address = "scope1qpgnf7pk59wprm9tk6nn82kkdtuq2wlq5p";
        let identifier = AssetIdentifier::asset_uuid(initial_uuid);
        let asset_uuid = identifier
            .get_asset_uuid()
            .expect("the asset uuid should be directly accessible");
        let scope_address = identifier
            .get_scope_address()
            .expect("the scope address should be accessible by conversion");
        assert_eq!(
            initial_uuid, asset_uuid,
            "the asset uuid output should be identical to the input"
        );
        assert_eq!(
            expected_scope_address, scope_address,
            "the scope address output should be as expected"
        );
    }

    #[test]
    fn test_asset_identifier_to_functions_from_scope_address() {
        let initial_address = "scope1qzdyhglu59w3rm9n0z0h3mn657yqrgjcwl";
        let expected_asset_uuid = "9a4ba3fc-a15d-11ec-b378-9f78ee7aa788";
        let identifier = AssetIdentifier::scope_address(initial_address);
        let scope_address = identifier
            .get_scope_address()
            .expect("the scope address should be directly accessible");
        let asset_uuid = identifier
            .get_asset_uuid()
            .expect("the asset uuid should be accessible by conversion");
        assert_eq!(
            initial_address, scope_address,
            "the scope address output should be identical to the input"
        );
        assert_eq!(
            expected_asset_uuid, asset_uuid,
            "the asset uuid output should be as expected"
        );
    }
}
