use crate::core::types::entity_detail::EntityDetail;
use crate::util::traits::OptionExtensions;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Defines an external account designated as a recipient of funds during the verification process.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FeeDestinationV2 {
    /// The Provenance Blockchain bech32 address belonging to the account.
    pub address: String,
    /// The amount to be distributed to this account from the designated total [onboarding_cost](super::verifier_detail::VerifierDetailV2::onboarding_cost) of the
    /// containing [VerifierDetailV2](super::verifier_detail::VerifierDetailV2).  This number should
    /// always sum with the other fee destinations to be less than or at most equal to the total
    /// onboarding cost.
    pub fee_amount: Uint128,
    /// An optional set of fields that define the fee destination, including its name and home URL location.
    pub entity_detail: Option<EntityDetail>,
}
impl FeeDestinationV2 {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `address` The Provenance Blockchain bech32 address belonging to the account.
    /// * `fee_amount` The amount to be distributed to this account from the designated total [onboarding_cost](super::verifier_detail::VerifierDetailV2::onboarding_cost)
    /// of the containing [VerifierDetailV2](super::verifier_detail::VerifierDetailV2).
    pub fn new<S: Into<String>>(address: S, fee_amount: u128) -> Self {
        Self {
            address: address.into(),
            fee_amount: Uint128::new(fee_amount),
            entity_detail: None,
        }
    }

    /// Constructs a new instance of this struct with an entity detail.
    ///
    /// # Parameters
    ///
    /// * `address` The Provenance Blockchain bech32 address belonging to the account.
    /// * `fee_amount` The amount to be distributed to this account from the designated total [onboarding_cost](super::verifier_detail::VerifierDetailV2::onboarding_cost)
    /// of the containing [VerifierDetailV2](super::verifier_detail::VerifierDetailV2).
    /// * `entity_detail` An optional set of fields that define the fee destination, including its
    /// name and home URL location.
    pub fn new_with_detail<S: Into<String>>(
        address: S,
        fee_amount: Uint128,
        entity_detail: EntityDetail,
    ) -> Self {
        Self {
            address: address.into(),
            fee_amount,
            entity_detail: entity_detail.to_some(),
        }
    }
}
