use crate::core::types::fee_destination::FeeDestinationV2;
use crate::core::types::onboarding_cost::OnboardingCost;
use crate::core::types::subsequent_classification_detail::SubsequentClassificationDetail;
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
    /// Defines the cost to use in place of the root [onboarding_cost](self::VerifierDetailV2::onboarding_cost) and
    /// [fee_destinations](self::VerifierDetailV2::fee_destinations) when retrying classification for a failed
    /// verification.  If not present, the original values used for the first verification will be
    /// used.
    pub retry_cost: Option<OnboardingCost>,
    /// An optional set of fields that define behaviors when classification is being run for an
    /// asset that is already classified as a different type.
    pub subsequent_classification_detail: Option<SubsequentClassificationDetail>,
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
    /// * `retry_cost` Defines the cost to use in place of the root [onboarding_cost](self::VerifierDetailV2::onboarding_cost)
    /// and [fee_destinations](self::VerifierDetailV2::fee_destinations) when retrying classification for a failed
    /// verification.  If not present, the original values used for the first verification will be
    /// used.
    /// * `subsequent_classification_detail` An optional set of fields that define behaviors when
    /// classification is being run for an asset that is already classified as a different type.
    pub fn new<S1: Into<String>, S2: Into<String>>(
        address: S1,
        onboarding_cost: Uint128,
        onboarding_denom: S2,
        fee_destinations: Vec<FeeDestinationV2>,
        entity_detail: Option<EntityDetail>,
        retry_cost: Option<OnboardingCost>,
        subsequent_classification_detail: Option<SubsequentClassificationDetail>,
    ) -> Self {
        VerifierDetailV2 {
            address: address.into(),
            onboarding_cost,
            onboarding_denom: onboarding_denom.into(),
            fee_destinations,
            entity_detail,
            retry_cost,
            subsequent_classification_detail,
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

    /// Packs the root-level onboarding_cost and fee_destinations fields into an OnboardingCost
    /// struct.
    pub fn get_default_cost(&self) -> OnboardingCost {
        OnboardingCost::new(self.onboarding_cost.u128(), &self.fee_destinations)
    }

    /// Determines the values to use for retrying classification on an asset that has been rejected
    /// by a verifier.  Will try to use the root [retry_cost](self::VerifierDetailV2::retry_cost) values, if present.
    /// If missing, the default costs will be used.
    pub fn get_retry_cost(&self) -> OnboardingCost {
        if let Some(ref retry_cost) = self.retry_cost {
            retry_cost.to_owned()
        } else {
            self.get_default_cost()
        }
    }

    /// Determines the values to use for classifying an asset that has been previously classified
    /// as a different asset type.  If the [subsequent_classification_detail](self::VerifierDetailV2::subsequent_classification_detail)
    /// field is populated, its [cost](super::subsequent_classification_detail::SubsequentClassificationDetail::cost) field will be
    /// used to determine costs paid during onboarding.  If it is not populated, the default costs
    /// will be used.
    pub fn get_subsequent_classification_cost(&self) -> OnboardingCost {
        if let Some(ref subsequent_detail) = self.subsequent_classification_detail {
            if let Some(ref cost) = subsequent_detail.cost {
                return cost.to_owned();
            }
        }
        self.get_default_cost()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::types::fee_destination::FeeDestinationV2;
    use crate::core::types::onboarding_cost::OnboardingCost;
    use crate::core::types::subsequent_classification_detail::SubsequentClassificationDetail;
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::util::constants::NHASH;
    use crate::util::traits::OptionExtensions;
    use cosmwasm_std::Uint128;

    #[test]
    fn test_no_fee_destinations_fee_total() {
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            NHASH,
            vec![],
            None,
            None,
            None,
        );
        assert_eq!(
            0,
            verifier.get_default_cost().get_fee_total(),
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
            None,
            None,
        );
        assert_eq!(
            55, verifier.get_default_cost().get_fee_total(),
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
            None,
            None,
        );
        assert_eq!(
            210, verifier.get_default_cost().get_fee_total(),
            "expected the fee total to be the sum of all fee destinations' fee amounts (10 + 20 + 30 + 40 + 50 + 60 = 210)",
        );
    }

    #[test]
    fn test_get_default_cost_uses_the_root_fields_as_source() {
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            NHASH,
            vec![FeeDestinationV2::new("fee", 1)],
            None,
            None,
            None,
        );
        let onboarding_cost = verifier.get_default_cost();
        assert_eq!(
            verifier.onboarding_cost, onboarding_cost.cost,
            "the cost of the onboarding cost should equate to the value specified in the root of the verifier",
        );
        assert_eq!(
            verifier.fee_destinations,
            onboarding_cost.fee_destinations,
            "the fee destinations of the onboarding cost should equate to the value specified in the root of the verifier",
        );
    }

    #[test]
    fn test_get_retry_cost_uses_default_when_not_specified() {
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            NHASH,
            vec![FeeDestinationV2::new("fee", 1)],
            None,
            None,
            None,
        );
        assert_eq!(
            verifier.get_default_cost(),
            verifier.get_retry_cost(),
            "the retry cost should equate to the default cost when the retry node is not specified",
        );
    }

    #[test]
    fn test_get_retry_cost_uses_provided_values_when_specified() {
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            NHASH,
            vec![FeeDestinationV2::new("fee", 1)],
            None,
            OnboardingCost::new(150, &[FeeDestinationV2::new("fee-2", 5)]).to_some(),
            None,
        );
        let root_retry_cost = verifier
            .retry_cost
            .clone()
            .expect("retry cost should be set on the root of the verifier");
        assert_eq!(
            root_retry_cost,
            verifier.get_retry_cost(),
            "the root retry cost should be returned when it is set on the verifier",
        );
    }

    #[test]
    fn test_get_subsequent_classification_cost_uses_default_when_none_specified() {
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            NHASH,
            vec![FeeDestinationV2::new("fee", 1)],
            None,
            None,
            None,
        );
        assert_eq!(
            verifier.get_default_cost(),
            verifier.get_subsequent_classification_cost(),
            "the default costs should be used for subsequent classification cost when no subsequent detail is provided",
        );
    }

    #[test]
    fn test_get_subsequent_classification_cost_uses_default_when_subsequent_node_is_empty() {
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            NHASH,
            vec![FeeDestinationV2::new("fee", 1)],
            None,
            None,
            SubsequentClassificationDetail::new::<String>(None, &[]).to_some(),
        );
        assert_eq!(
            verifier.get_default_cost(),
            verifier.get_subsequent_classification_cost(),
            "the default costs should be used for subsequent classification cost when the subsequent detail is blank",
        );
    }

    #[test]
    fn test_get_subsequent_classification_cost_uses_inner_default_when_provided() {
        let expected_onboarding_cost =
            OnboardingCost::new(100, &[FeeDestinationV2::new("default_cost_fee", 7)]);
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            NHASH,
            vec![FeeDestinationV2::new("fee", 1)],
            None,
            None,
            SubsequentClassificationDetail::new::<String>(
                expected_onboarding_cost.clone().to_some(),
                &[],
            )
            .to_some(),
        );
        assert_eq!(
            expected_onboarding_cost,
            verifier.get_subsequent_classification_cost(),
            "the default subsequent classification cost should be used when no asset type targets match",
        );
    }
}
