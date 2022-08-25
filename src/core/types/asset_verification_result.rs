use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A simple wrapper for the result of a verification for a scope.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetVerificationResult {
    /// A free-form message describing the result of the verification process.
    pub message: String,
    /// If true, the asset is deemed as successfully classified.  On false, an issue arose with the
    /// verifier and/or underlying asset data that caused the scope to not be classified.
    pub success: bool,
}
