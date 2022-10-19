use crate::core::types::fee_destination::FeeDestinationV2;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Defines costs used to onboard an asset to the contract for classification.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct OnboardingCost {
    /// The amount of coin to be paid when an asset is sent to the [onboard_asset execute function](crate::execute::onboard_asset::onboard_asset).
    pub cost: Uint128,
    /// Any specific fee destinations that should be sent to sources other than the selected [verifier](super::verifier_detail::VerifierDetailV2).
    pub fee_destinations: Vec<FeeDestinationV2>,
}
impl OnboardingCost {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `cost` The amount of coin to be paid when an asset is sent to the [onboard_asset execute function](crate::execute::onboard_asset::onboard_asset).
    /// * `fee_destinations` Any specific fee destinations that should be sent to sources other than
    /// the selected [verifier](super::verifier_detail::VerifierDetailV2).
    pub fn new(cost: u128, fee_destinations: &[FeeDestinationV2]) -> Self {
        Self {
            cost: Uint128::new(cost),
            fee_destinations: fee_destinations.iter().cloned().collect(),
        }
    }

    /// Sums all the fee amounts held within the individual fee destinations in this struct.
    pub fn get_fee_total(&self) -> u128 {
        self.fee_destinations
            .iter()
            .map(|d| d.fee_amount.u128())
            .sum::<u128>()
    }
}
