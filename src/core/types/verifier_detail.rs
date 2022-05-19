use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{entity_detail::EntityDetail, fee_destination::FeeDestination};

/// Defines the fees and addresses for a single verifier account for an [AssetDefinition](super::asset_definition::AssetDefinition).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct VerifierDetail {
    /// The Provenance Blockchain bech32 address of the verifier account.
    pub address: String,
    /// The total amount charged to use the onboarding process this this verifier.
    pub onboarding_cost: Uint128,
    /// The coin denomination used for this onboarding process.
    pub onboarding_denom: String,
    /// The percent amount taken from the total [onboarding_cost](self::VerifierDetail::onboarding_cost)
    /// to send to the underlying [FeeDestinations](super::fee_destination::FeeDestination). This should
    /// be a number from 0 to 1, representing a percentage (ex: 0.35 = 35%).
    pub fee_percent: Decimal,
    /// Each account that should receive the amount designated in the [fee_percent](self::VerifierDetail::fee_percent).
    /// All of these destinations' individual [fee_percent](super::fee_destination::FeeDestination::fee_percent) properties
    /// should sum to 1.  Less amounts will cause this verifier detail to be considered invalid and rejected
    /// in requests that include it.
    pub fee_destinations: Vec<FeeDestination>,
    /// An optional set of fields that define the verifier, including its name and home URL location.
    pub entity_detail: Option<EntityDetail>,
}
impl VerifierDetail {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `address` The Provenance Blockchain bech32 address of the verifier account.
    /// * `onboarding_cost` The total amount charged to use the onboarding process this this verifier.
    /// * `onboarding_denom` The coin denomination used for this onboarding process.
    /// * `fee_percent` The percent amount taken from the total [onboarding_cost](self::VerifierDetail::onboarding_cost)
    /// to send to the underlying [FeeDestinations](super::fee_destination::FeeDestination).
    /// `fee_destinations` Each account that should receive the amount designated in the [fee_percent](self::VerifierDetail::fee_percent).
    /// `entity_detail` An optional set of fields that define the verifier, including its name and home URL location.
    pub fn new<S1: Into<String>, S2: Into<String>>(
        address: S1,
        onboarding_cost: Uint128,
        onboarding_denom: S2,
        fee_percent: Decimal,
        fee_destinations: Vec<FeeDestination>,
        entity_detail: Option<EntityDetail>,
    ) -> Self {
        VerifierDetail {
            address: address.into(),
            onboarding_cost,
            onboarding_denom: onboarding_denom.into(),
            fee_percent,
            fee_destinations,
            entity_detail,
        }
    }
}
