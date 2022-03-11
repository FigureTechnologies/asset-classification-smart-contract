use crate::{
    core::asset::{AssetDefinition, ValidatorDetail},
    util::{
        aliases::ContractResult,
        scope_address_utils::{asset_uuid_to_scope_address, scope_address_to_asset_uuid},
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
        AssetIdentifier::AssetUuid {
            asset_uuid: asset_uuid.into(),
        }
    }

    pub fn scope_address<S: Into<String>>(scope_address: S) -> Self {
        AssetIdentifier::ScopeAddress {
            scope_address: scope_address.into(),
        }
    }

    /// Takes the value provided and derives both values from it, where necessary,
    /// ensuring that both asset_uuid and scope_address are available to the user
    pub fn parse_identifiers(&self) -> ContractResult<AssetIdentifiers> {
        match self {
            AssetIdentifier::AssetUuid { asset_uuid } => {
                AssetIdentifiers::new(asset_uuid, asset_uuid_to_scope_address(asset_uuid)?).to_ok()
            }
            AssetIdentifier::ScopeAddress { scope_address } => {
                AssetIdentifiers::new(scope_address_to_asset_uuid(scope_address)?, scope_address)
                    .to_ok()
            }
        }
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
        AssetQualifier::AssetType {
            asset_type: asset_type.into(),
        }
    }

    pub fn scope_spec_address<S: Into<String>>(scope_spec_address: S) -> Self {
        AssetQualifier::ScopeSpecAddress {
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
            .parse_identifiers()
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
            .parse_identifiers()
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
}
