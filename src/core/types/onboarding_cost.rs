use crate::core::types::fee_destination::FeeDestinationV2;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// TODO: Doc comments to link other relevant structs.
/// Defines costs used to onboard an asset to the contract for classification.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct OnboardingCost {
    /// The amount of coin to be paid when an asset is sent to the onboard_asset execute function.
    pub cost: Uint128,
    /// Any specific fee destinations that should be sent to sources other than the selected verifier.
    pub fee_destinations: Vec<FeeDestinationV2>,
}
impl OnboardingCost {
    pub fn new(cost: u128, fee_destinations: &[FeeDestinationV2]) -> Self {
        Self {
            cost: Uint128::new(cost),
            fee_destinations: fee_destinations.iter().cloned().collect(),
        }
    }
}
