use core::fmt;
use std::fmt::{Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetOnboardingStatus {
    Pending,
    Denied,
    Approved,
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
            }
        )
    }
}
