use crate::core::types::onboarding_cost::OnboardingCost;
use crate::util::traits::OptionExtensions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The root subsequent classifications node for a [VerifierDetailV2](super::verifier_detail::VerifierDetailV2).
/// Contains the default subsequent costs for onboarding an asset with this verifier after already
/// classifying it as a different type with the same verifier.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SubsequentClassificationDetail {
    /// The onboarding cost to use when classifying an asset using the associated verifier after
    /// having already classified it as a different type with the same verifier.  If not set, the
    /// default verifier costs are used.
    pub cost: Option<OnboardingCost>,
    /// Specifies the asset types that an asset can already be classified as when using this verifier.
    /// If not set, any asset type may be used.  This value will be rejected if it is supplied as
    /// an empty vector.
    pub allowed_asset_types: Option<Vec<String>>,
}
impl SubsequentClassificationDetail {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `cost` The onboarding cost to use when classifying an asset using the associated verifier
    /// after having already classified it as a different type with the same verifier.  If not set,
    /// the default verifier costs are used.
    /// * `allowed_asset_types` Specifies the asset types that an asset can already be classified as
    /// when using this verifier.  If not set, any asset type may be used.  This value will be
    /// rejected if it is supplied as an empty vector.
    pub fn new<S: Into<String> + Clone>(
        cost: Option<OnboardingCost>,
        allowed_asset_types: &[S],
    ) -> Self {
        let allowed_asset_types = if !allowed_asset_types.is_empty() {
            allowed_asset_types
                .iter()
                .cloned()
                .map(|s| s.into())
                .collect::<Vec<String>>()
                .to_some()
        } else {
            None
        };
        Self {
            cost,
            allowed_asset_types,
        }
    }
}
