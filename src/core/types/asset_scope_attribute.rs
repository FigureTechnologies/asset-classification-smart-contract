use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    core::{error::ContractError, types::access_definition::AccessDefinitionType},
    util::{
        aliases::AssetResult,
        scope_address_utils::bech32_string_to_addr,
        traits::{OptionExtensions, ResultExtensions},
    },
};

use super::{
    access_definition::AccessDefinition, asset_identifier::AssetIdentifier,
    asset_onboarding_status::AssetOnboardingStatus,
    asset_verification_result::AssetVerificationResult, verifier_detail::VerifierDetail,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetScopeAttribute {
    pub asset_uuid: String,
    pub scope_address: String,
    pub asset_type: String,
    pub requestor_address: Addr,
    pub verifier_address: Addr,
    pub onboarding_status: AssetOnboardingStatus,
    pub latest_verifier_detail: Option<VerifierDetail>,
    pub latest_verification_result: Option<AssetVerificationResult>,
    pub access_definitions: Vec<AccessDefinition>,
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
        verifier_address: S3,
        onboarding_status: Option<AssetOnboardingStatus>,
        latest_verifier_detail: VerifierDetail,
        access_routes: Vec<String>,
    ) -> AssetResult<Self> {
        let identifiers = identifier.to_identifiers()?;
        let req_addr = bech32_string_to_addr(requestor_address)?;
        let ver_addr = bech32_string_to_addr(verifier_address)?;
        if ver_addr != latest_verifier_detail.address {
            return ContractError::generic(format!("provided verifier address [{}] did not match the verifier detail's address [{}]", ver_addr, latest_verifier_detail.address).as_str()).to_err();
        }
        // Remove all access routes that are empty strings to prevent bad data from beign provided
        let filtered_access_routes = access_routes
            .into_iter()
            .map(|r| r.trim().to_owned())
            .filter(|r| !r.is_empty())
            .collect::<Vec<String>>();
        // If access routes were provided as an empty array, or the array only contains empty strings, don't create an access definition for the requestor
        let access_definitions = if filtered_access_routes.is_empty() {
            vec![]
        } else {
            vec![AccessDefinition::new_checked(
                &req_addr,
                filtered_access_routes,
                AccessDefinitionType::Requestor,
            )?]
        };
        AssetScopeAttribute {
            asset_uuid: identifiers.asset_uuid,
            scope_address: identifiers.scope_address,
            asset_type: asset_type.into(),
            requestor_address: req_addr,
            verifier_address: ver_addr,
            onboarding_status: onboarding_status.unwrap_or(AssetOnboardingStatus::Pending),
            latest_verifier_detail: latest_verifier_detail.to_some(),
            latest_verification_result: None,
            access_definitions,
        }
        .to_ok()
    }
}
