use crate::core::types::entity_detail::EntityDetail;
use crate::util::traits::OptionExtensions;
use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Mul;

// TODO: Delete after upgrading all contract instances to FeeDestinationV2
/// Defines an external account designated as a recipient of funds during the verification process.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FeeDestination {
    /// The Provenance Blockchain bech32 address belonging to the account.
    pub address: String,
    /// The amount to be distributed to this account from the designated total [fee_percent](super::verifier_detail::VerifierDetail::fee_percent) of the
    /// containing [VerifierDetail](super::verifier_detail::VerifierDetail).  This number should
    /// always be between 0 and 1, and indicate a percentage.  Ex: 0.5 indicates 50%.
    /// For instance, if the fee total is 100nhash and the verifier detail's fee percent is .5 (50%)
    /// and the destination's fee percent is 1 (100%), then that fee destination account would
    /// receive 50nhash during the transaction, which is 100% of the 50% designated to fee accounts.
    pub fee_percent: Decimal,
}
impl FeeDestination {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `address` The Provenance Blockchain bech32 address belonging to the account.
    /// * `fee_percent` The amount to be distributed to this account from the designated total [fee_percent](super::verifier_detail::VerifierDetail::fee_percent)
    /// of the containing [VerifierDetail](super::verifier_detail::VerifierDetail).
    pub fn new<S: Into<String>>(address: S, fee_percent: Decimal) -> Self {
        FeeDestination {
            address: address.into(),
            fee_percent,
        }
    }

    pub fn to_v2(self, total_fee_cost: u128) -> FeeDestinationV2 {
        FeeDestinationV2 {
            address: self.address,
            fee_amount: Uint128::new(total_fee_cost).mul(self.fee_percent),
            // New field that can't have ever been set, so just start it out as unset
            entity_detail: None,
        }
    }
}

/// Defines an external account designated as a recipient of funds during the verification process.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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
    pub fn new<S: Into<String>>(address: S, fee_amount: Uint128) -> Self {
        Self {
            address: address.into(),
            fee_amount,
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
