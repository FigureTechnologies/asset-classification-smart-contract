use cosmwasm_std::{Addr, Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::util::{aliases::ContractResult, functions::validate_address, traits::ResultExtensions};

use super::{error::ContractError, msg::AssetDefinitionInput};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetDefinition {
    pub asset_type: String,
    pub validators: Vec<ValidatorDetail>,
    pub enabled: bool,
}
impl AssetDefinition {
    pub fn new<S: Into<String>>(asset_type: S, validators: Vec<ValidatorDetail>) -> Self {
        AssetDefinition {
            asset_type: asset_type.into(),
            validators,
            enabled: true,
        }
    }
}
impl From<AssetDefinitionInput> for AssetDefinition {
    fn from(input: AssetDefinitionInput) -> Self {
        Self {
            asset_type: input.asset_type,
            validators: input.validators,
            enabled: input.enabled.unwrap_or(true),
        }
    }
}
impl From<&AssetDefinitionInput> for AssetDefinition {
    fn from(input: &AssetDefinitionInput) -> Self {
        AssetDefinition {
            asset_type: input.asset_type.clone(),
            validators: input.validators.clone(),
            enabled: input.enabled.unwrap_or(true),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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
pub enum AssetOnboardingStatus {
    Pending,
    Denied,
    Approved,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetValidationResult {
    pub message: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetScopeAttribute {
    pub asset_type: String,
    pub requestor_address: Addr,
    pub validator_address: Addr,
    pub onboarding_status: AssetOnboardingStatus,
    pub latest_validator_detail: Option<ValidatorDetail>,
    pub latest_validation_result: Option<AssetValidationResult>,
    pub access_routes: Vec<String>,
}
impl AssetScopeAttribute {
    pub fn new_unchecked<S1: Into<String>, A1: Into<Addr>, A2: Into<Addr>>(
        asset_type: S1,
        requestor_address: A1,
        validator_address: A2,
        onboarding_status: Option<AssetOnboardingStatus>,
        latest_validator_detail: ValidatorDetail,
    ) -> Self {
        AssetScopeAttribute {
            asset_type: asset_type.into(),
            requestor_address: requestor_address.into(),
            validator_address: validator_address.into(),
            onboarding_status: onboarding_status.unwrap_or(AssetOnboardingStatus::Pending),
            latest_validator_detail: Some(latest_validator_detail),
            latest_validation_result: None,
            access_routes: vec![],
        }
    }

    pub fn new<S1: Into<String>, A1: Into<Addr>, A2: Into<Addr>>(
        asset_type: S1,
        requestor_address: A1,
        validator_address: A2,
        onboarding_status: Option<AssetOnboardingStatus>,
        latest_validator_detail: ValidatorDetail,
    ) -> ContractResult<Self> {
        let req_addr = validate_address(requestor_address)?;
        let val_addr = validate_address(validator_address)?;
        if val_addr.to_string() != latest_validator_detail.address {
            return ContractError::std_err(format!("provided validator address [{}] did not match the validator detail's address [{}]", val_addr, latest_validator_detail.address).as_str()).to_err();
        }
        AssetScopeAttribute::new_unchecked(
            asset_type,
            req_addr,
            val_addr,
            onboarding_status,
            latest_validator_detail,
        )
        .to_ok()
    }
}
impl ResultExtensions for AssetScopeAttribute {}
