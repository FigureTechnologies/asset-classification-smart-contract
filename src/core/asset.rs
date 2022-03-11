use cosmwasm_std::{Addr, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::util::{
    aliases::{ContractResult, DepsC},
    functions::generate_asset_attribute_name,
    scope_address_utils::{
        asset_uuid_to_scope_address, bech32_string_to_addr, scope_address_to_asset_uuid,
        scope_spec_address_to_scope_spec_uuid, scope_spec_uuid_to_scope_spec_address,
    },
    traits::{OptionExtensions, ResultExtensions},
};

use super::{error::ContractError, state::config_read};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetDefinition {
    pub asset_type: String,
    pub scope_spec_address: String,
    pub validators: Vec<ValidatorDetail>,
    pub enabled: bool,
}
impl AssetDefinition {
    pub fn new<S1: Into<String>, S2: Into<String>>(
        asset_type: S1,
        scope_spec_address: S2,
        validators: Vec<ValidatorDetail>,
    ) -> Self {
        AssetDefinition {
            asset_type: asset_type.into(),
            scope_spec_address: scope_spec_address.into(),
            validators,
            enabled: true,
        }
    }

    /// Converts the asset_type value to lowercase and serializes it as bytes,
    /// then uplifts the value to a vector to allow it to be returned.
    pub fn storage_key(&self) -> Vec<u8> {
        self.asset_type.to_lowercase().as_bytes().to_vec()
    }

    pub fn attribute_name(&self, deps: &DepsC) -> ContractResult<String> {
        let state = config_read(deps.storage).load()?;
        generate_asset_attribute_name(&self.asset_type, state.base_contract_name).to_ok()
    }
}
impl From<AssetDefinitionInput> for AssetDefinition {
    fn from(input: AssetDefinitionInput) -> Self {
        Self {
            asset_type: input.asset_type,
            scope_spec_address: input.scope_spec_address,
            validators: input.validators,
            enabled: input.enabled.unwrap_or(true),
        }
    }
}
impl From<&AssetDefinitionInput> for AssetDefinition {
    fn from(input: &AssetDefinitionInput) -> Self {
        AssetDefinition {
            asset_type: input.asset_type.clone(),
            scope_spec_address: input.scope_spec_address.clone(),
            validators: input.validators.clone(),
            enabled: input.enabled.unwrap_or(true),
        }
    }
}

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
#[serde(rename_all = "snake_case")]
pub struct ValidatorDetail {
    pub address: String,
    pub onboarding_cost: Uint128,
    pub onboarding_denom: String,
    pub fee_percent: Decimal,
    pub fee_destinations: Vec<FeeDestination>,
}
impl ValidatorDetail {
    pub fn new<S1: Into<String>, S2: Into<String>>(
        address: S1,
        onboarding_cost: Uint128,
        onboarding_denom: S2,
        fee_percent: Decimal,
        fee_destinations: Vec<FeeDestination>,
    ) -> Self {
        ValidatorDetail {
            address: address.into(),
            onboarding_cost,
            onboarding_denom: onboarding_denom.into(),
            fee_percent,
            fee_destinations,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FeeDestination {
    pub address: String,
    pub fee_percent: Decimal,
}
impl FeeDestination {
    pub fn new<S: Into<String>>(address: S, fee_percent: Decimal) -> Self {
        FeeDestination {
            address: address.into(),
            fee_percent,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetOnboardingStatus {
    Pending,
    Denied,
    Approved,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetValidationResult {
    pub message: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetScopeAttribute {
    pub asset_uuid: String,
    pub scope_address: String,
    pub asset_type: String,
    pub requestor_address: Addr,
    pub validator_address: Addr,
    pub onboarding_status: AssetOnboardingStatus,
    pub latest_validator_detail: Option<ValidatorDetail>,
    pub latest_validation_result: Option<AssetValidationResult>,
    pub access_routes: Vec<String>,
}
impl AssetScopeAttribute {
    /// Constructs a new instance of AssetScopeAttribute from the input params
    /// Prefer initializing a scope attribute with this function!
    /// It ensures passed addresses are valid, as well as ensuring that the
    /// asset uuid and scope address match each other
    pub fn new<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        identifier: &AssetIdentifier,
        asset_type: S1,
        requestor_address: S2,
        validator_address: S3,
        onboarding_status: Option<AssetOnboardingStatus>,
        latest_validator_detail: ValidatorDetail,
    ) -> ContractResult<Self> {
        let identifiers = identifier.to_identifiers()?;
        let req_addr = bech32_string_to_addr(requestor_address)?;
        let val_addr = bech32_string_to_addr(validator_address)?;
        if val_addr != latest_validator_detail.address {
            return ContractError::std_err(format!("provided validator address [{}] did not match the validator detail's address [{}]", val_addr, latest_validator_detail.address).as_str()).to_err();
        }
        AssetScopeAttribute {
            asset_uuid: identifiers.asset_uuid,
            scope_address: identifiers.scope_address,
            asset_type: asset_type.into(),
            requestor_address: req_addr,
            validator_address: val_addr,
            onboarding_status: onboarding_status.unwrap_or(AssetOnboardingStatus::Pending),
            latest_validator_detail: latest_validator_detail.to_some(),
            latest_validation_result: None,
            access_routes: vec![],
        }
        .to_ok()
    }
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

    pub fn get_scope_spec_uuid(&self) -> ContractResult<String> {
        match self {
            Self::ScopeSpecUuid { scope_spec_uuid } => (*scope_spec_uuid).clone().to_ok(),
            Self::ScopeSpecAddress { scope_spec_address } => {
                scope_spec_address_to_scope_spec_uuid(scope_spec_address)
            }
        }
    }

    pub fn get_scope_spec_address(&self) -> ContractResult<String> {
        match self {
            Self::ScopeSpecUuid { scope_spec_uuid } => {
                scope_spec_uuid_to_scope_spec_address(scope_spec_uuid)
            }
            Self::ScopeSpecAddress { scope_spec_address } => (*scope_spec_address).clone().to_ok(),
        }
    }

    /// Takes the value provided and dervies both values from it, where necessary,
    /// ensuring that both scope_spec_uuid and scope_spec_address are available to the user
    pub fn to_identifiers(&self) -> ContractResult<ScopeSpecIdentifiers> {
        ScopeSpecIdentifiers::new(self.get_scope_spec_uuid()?, self.get_scope_spec_address()?)
            .to_ok()
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

#[cfg(test)]
mod tests {
    use super::{AssetIdentifier, ScopeSpecIdentifier};

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

    #[test]
    fn test_scope_spec_identifier_parse_for_scope_spec_uuid() {
        // The uuid was generated randomly and the scope spec address was derived via provenance's MetadataAddress util
        let scope_spec_uuid = "35398a4f-fd44-4737-ba01-f1c46ca62257";
        let expected_scope_spec_address = "scopespec1qs6nnzj0l4zywda6q8cugm9xyfts8xugze";
        let identifier = ScopeSpecIdentifier::uuid(scope_spec_uuid);
        let result_identifiers = identifier
            .to_identifiers()
            .expect("parsing identifiers should succeed");
        assert_eq!(
            scope_spec_uuid,
            result_identifiers.scope_spec_uuid.as_str(),
            "expected the scope spec uuid value to pass through successfully",
        );
        assert_eq!(
            expected_scope_spec_address,
            result_identifiers.scope_spec_address.as_str(),
            "expected the scope spec address to be derived correctly",
        );
    }

    #[test]
    fn test_scope_spec_identifier_parse_for_scope_spec_address() {
        // The uuid was generated randomly and the scope spec address was derived via provenance's MetadataAddress util
        let scope_spec_address = "scopespec1q3vehdq7dn25ar9y53llsaslcglqh43r38";
        let expected_scope_spec_uuid = "599bb41e-6cd5-4e8c-a4a4-7ff8761fc23e";
        let identifier = ScopeSpecIdentifier::address(scope_spec_address);
        let result_identifiers = identifier
            .to_identifiers()
            .expect("parsing identifiers should succeed");
        assert_eq!(
            scope_spec_address,
            result_identifiers.scope_spec_address.as_str(),
            "expected the scope spec address to pass through successfully",
        );
        assert_eq!(
            expected_scope_spec_uuid,
            result_identifiers.scope_spec_uuid.as_str(),
            "expected the scope spec uuid to be derived correctly",
        );
    }

    #[test]
    fn test_scope_spec_identifier_to_functions_from_scope_spec_uuid() {
        let initial_uuid = "a2d0ff08-1f01-4209-bdac-d8dea2487d7a";
        let expected_scope_spec_address = "scopespec1qj3dplcgruq5yzda4nvdagjg04aq93tuxs";
        let identifier = ScopeSpecIdentifier::uuid(initial_uuid);
        let scope_spec_uuid = identifier
            .get_scope_spec_uuid()
            .expect("the scope spec uuid should be directly accessible");
        let scope_spec_address = identifier
            .get_scope_spec_address()
            .expect("the scope spec address should be accessible by conversion");
        assert_eq!(
            initial_uuid, scope_spec_uuid,
            "the scope spec uuid output should be identical to the input",
        );
        assert_eq!(
            expected_scope_spec_address, scope_spec_address,
            "the scope spec address output should be as expected",
        );
    }

    #[test]
    fn test_scope_spec_identifier_to_functions_from_scope_spec_address() {
        let initial_address = "scopespec1q3ptevdt2x5yg5ycflqjsky8rz5q47e34p";
        let expected_scope_spec_uuid = "42bcb1ab-51a8-4450-984f-c128588718a8";
        let identifier = ScopeSpecIdentifier::address(initial_address);
        let scope_spec_address = identifier
            .get_scope_spec_address()
            .expect("the scope spec address should be directly accessible");
        let scope_spec_uuid = identifier
            .get_scope_spec_uuid()
            .expect("the scope spec uuid should be accessible by conversion");
        assert_eq!(
            initial_address, scope_spec_address,
            "the scope spec address output should be identical to the input",
        );
        assert_eq!(
            expected_scope_spec_uuid, scope_spec_uuid,
            "the scope spec uuid should be as expected",
        );
    }
}
