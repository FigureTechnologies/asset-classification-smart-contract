use crate::core::types::fee_destination::FeeDestinationV2;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::entity_detail::EntityDetail;

/// Defines the fees and addresses for a single verifier account for an [AssetDefinitionV3](super::asset_definition::AssetDefinitionV3).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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
            vec![FeeDestinationV2::new("fee-address", 55)],
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
                FeeDestinationV2::new("fee-address-1", 10),
                FeeDestinationV2::new("fee-address-2", 20),
                FeeDestinationV2::new("fee-address-3", 30),
                FeeDestinationV2::new("fee-address-4", 40),
                FeeDestinationV2::new("fee-address-5", 50),
                FeeDestinationV2::new("fee-address-6", 60),
            ],
            None,
        );
        assert_eq!(
            210, verifier.get_fee_total(),
            "expected the fee total to be the sum of all fee destinations' fee amounts (10 + 20 + 30 + 40 + 50 + 60 = 210)",
        );
    }
}
