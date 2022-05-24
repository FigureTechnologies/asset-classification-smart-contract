use crate::core::types::fee_destination::FeeDestinationV2;
use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Mul;

use super::{entity_detail::EntityDetail, fee_destination::FeeDestination};

// TODO: Delete after upgrading all contract instances to VerifierDetailV2
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

    pub fn to_v2(self) -> VerifierDetailV2 {
        let total_fee_cost = self.onboarding_cost.mul(self.fee_percent);
        VerifierDetailV2 {
            address: self.address,
            onboarding_cost: self.onboarding_cost,
            onboarding_denom: self.onboarding_denom,
            fee_destinations: self
                .fee_destinations
                .into_iter()
                .map(|dest| dest.to_v2(total_fee_cost.u128()))
                .collect(),
            entity_detail: self.entity_detail,
        }
    }
}

/// Defines the fees and addresses for a single verifier account for an [AssetDefinitionV2](super::asset_definition::AssetDefinitionV2).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct VerifierDetailV2 {
    /// The Provenance Blockchain bech32 address of the verifier account.
    pub address: String,
    /// The total amount charged to use the onboarding process this this verifier.
    pub onboarding_cost: Uint128,
    /// The coin denomination used for this onboarding process.
    pub onboarding_denom: String,
    /// Each account that should receive fees when onboarding a new scope to the contract.
    /// All of these destinations' individual [fee_amount](super::fee_destination::FeeDestinationV2::fee_amount) properties
    /// should sum to an amount less than or equal to the [onboarding_cost](super::verifier_detail::VerifierDetailV2::onboarding_cost).
    /// Amounts not precisely equal in sum will cause this verifier detail to be considered invalid
    /// and rejected in requests that include it.
    pub fee_destinations: Vec<FeeDestinationV2>,
    /// An optional set of fields that define the verifier, including its name and home URL location.
    pub entity_detail: Option<EntityDetail>,
}
impl VerifierDetailV2 {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `address` The Provenance Blockchain bech32 address of the verifier account.
    /// * `onboarding_cost` The total amount charged to use the onboarding process this this verifier.
    /// * `onboarding_denom` The coin denomination used for this onboarding process.
    /// * `fee_destinations` Each account that should receive some (or all) of the amount specified in [onboarding_cost](self::VerifierDetailV2::onboarding_cost).
    /// * `entity_detail` An optional set of fields that define the verifier, including its name and home URL location.
    pub fn new<S1: Into<String>, S2: Into<String>>(
        address: S1,
        onboarding_cost: Uint128,
        onboarding_denom: S2,
        fee_destinations: Vec<FeeDestinationV2>,
        entity_detail: Option<EntityDetail>,
    ) -> Self {
        VerifierDetailV2 {
            address: address.into(),
            onboarding_cost,
            onboarding_denom: onboarding_denom.into(),
            fee_destinations,
            entity_detail,
        }
    }

    /// Calculates a sum of all held [fee_destinations](self::VerifierDetailV2::fee_destinations)
    /// respective [fee_amount](super::fee_destination::FeeDestinationV2::fee_amount) fields.
    ///
    pub fn get_fee_total(&self) -> u128 {
        self.fee_destinations
            .iter()
            .map(|d| d.fee_amount.u128())
            .sum::<u128>()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::types::fee_destination::FeeDestinationV2;
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::util::constants::NHASH;
    use cosmwasm_std::Uint128;

    #[test]
    fn test_no_fee_destinations_fee_total() {
        let verifier = VerifierDetailV2::new("address", Uint128::new(100), NHASH, vec![], None);
        assert_eq!(
            0,
            verifier.get_fee_total(),
            "expected the fee total to be zero when no fee definitions are listed",
        );
    }

    #[test]
    fn test_one_fee_destination_fee_total() {
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            NHASH,
            vec![FeeDestinationV2::new("fee-address", Uint128::new(55))],
            None,
        );
        assert_eq!(
            55, verifier.get_fee_total(),
            "expected the fee total to directly reflect the amount listed in the single fee destination",
        );
    }

    #[test]
    fn test_many_fee_destinations_fee_total() {
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(1000),
            NHASH,
            vec![
                FeeDestinationV2::new("fee-address-1", Uint128::new(10)),
                FeeDestinationV2::new("fee-address-2", Uint128::new(20)),
                FeeDestinationV2::new("fee-address-3", Uint128::new(30)),
                FeeDestinationV2::new("fee-address-4", Uint128::new(40)),
                FeeDestinationV2::new("fee-address-5", Uint128::new(50)),
                FeeDestinationV2::new("fee-address-6", Uint128::new(60)),
            ],
            None,
        );
        assert_eq!(
            210, verifier.get_fee_total(),
            "expected the fee total to be the sum of all fee destinations' fee amounts (10 + 20 + 30 + 40 + 50 + 60 = 210)",
        );
    }
}
