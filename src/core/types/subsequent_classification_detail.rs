use crate::core::types::fee_destination::FeeDestinationV2;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SubsequentClassificationDetail {
    pub onboarding_cost: Uint128,
    pub fee_destinations: Vec<FeeDestinationV2>,
    pub allowed_asset_types: Vec<String>,
}
impl SubsequentClassificationDetail {
    pub fn new<S: Into<String> + Clone>(
        onboarding_cost: u128,
        fee_destinations: &[FeeDestinationV2],
        allowed_asset_types: &[S],
    ) -> Self {
        Self {
            onboarding_cost: Uint128::new(onboarding_cost),
            fee_destinations: fee_destinations.iter().cloned().collect(),
            allowed_asset_types: allowed_asset_types
                .iter()
                .cloned()
                .map(|s| s.into())
                .collect(),
        }
    }
}
