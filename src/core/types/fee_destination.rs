use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
}
