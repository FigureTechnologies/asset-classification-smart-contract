use crate::core::types::onboarding_cost::OnboardingCost;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// TODO: Doc comments to link other relevant structs.
/// The root subsequent classifications node for a verifier detail.  Contains the default subsequent
/// costs for onboarding an asset with this verifier after already classifying it as a different
/// type with the same verifier.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SubsequentClassificationDetail {
    /// The default onboarding costs to use when classifying a subsequent asset type on a scope.  If
    /// no asset type specification is provided for a subsequent type, this value will be used.  If
    /// this value is not present in that circumstance and no specific asset type target is supplied,
    /// the default onboarding costs in the root of the verifier detail will be used.
    pub default_cost: Option<OnboardingCost>,
    /// Specific asset type onboarding costs to use when onboarding a subsequent asset to a verifier.
    /// This value is preferred over the default cost values when both are present.
    pub asset_type_specifications: Vec<SubsequentClassificationSpecification>,
}
impl SubsequentClassificationDetail {
    pub fn new(
        default_cost: Option<OnboardingCost>,
        asset_type_specifications: &[SubsequentClassificationSpecification],
    ) -> Self {
        Self {
            default_cost,
            asset_type_specifications: asset_type_specifications.iter().cloned().collect(),
        }
    }
}

/// TODO: Doc comments to link other relevant structs.
/// Costs specified for onboarding an asset as a subsequent type to the contract with the same
/// verifier.  These values are preferred over default costs when provided.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SubsequentClassificationSpecification {
    /// The type of asset.  Must be unique when compared against other SubsequentClassificationSpecifications
    /// on a SubsequentClassificationDetail.
    pub asset_type: String,
    /// The cost to onboard this specific asset type in subsequent classification scenarios.
    pub cost: OnboardingCost,
}
impl SubsequentClassificationSpecification {
    pub fn new<S: Into<String>>(asset_type: S, cost: OnboardingCost) -> Self {
        Self {
            asset_type: asset_type.into(),
            cost,
        }
    }
}
