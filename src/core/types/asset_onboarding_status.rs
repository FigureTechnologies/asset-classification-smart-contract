use core::fmt;
use std::fmt::{Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An enum that denotes the various states that an [AssetScopeAttribute](super::asset_scope_attribute::AssetScopeAttribute) can have.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetOnboardingStatus {
    /// Indicates that the asset has been onboarded but has yet to be verified.
    Pending,
    /// Indicates that the asset has been verified and is determined to be unfit to be classified as
    /// its designated asset type.
    Denied,
    /// Indicates that the asset has been verified and has been successfully classified as its
    /// designated asset type.
    Approved,
    /// Indicates that the onboarding account has chosen to not trust the verifier, and the
    /// verification process has been completed.  At this point, the onboarding account must now
    /// execute the finalize_classification route to pay all required fees and complete classification.
    AwaitingFinalization,
}
impl Display for AssetOnboardingStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Pending => "pending",
                Self::Denied => "denied",
                Self::Approved => "approved",
                Self::AwaitingFinalization => "awaiting_finalization",
            }
        )
    }
}
