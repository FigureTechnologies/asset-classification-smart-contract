use cosmwasm_std::{Addr, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::util::{
    aliases::{ContractResult, DepsC},
    functions::generate_asset_attribute_name,
    scope_address_utils::bech32_string_to_addr,
    traits::{OptionExtensions, ResultExtensions},
};

use super::{
    error::ContractError,
    msg::{AssetDefinitionInput, AssetIdentifier},
    state::config_read,
};

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
        access_routes: Vec<String>,
    ) -> ContractResult<Self> {
        let identifiers = identifier.parse_identifiers()?;
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
            access_routes,
        }
        .to_ok()
    }
}
