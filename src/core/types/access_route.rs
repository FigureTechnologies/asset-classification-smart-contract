use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::util::traits::OptionExtensions;

// Extra derivations here are for sorting and determining if duplicate access routes are being added during updates
#[derive(
    Serialize, Deserialize, Clone, PartialEq, JsonSchema, Eq, Hash, Debug, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
/// Defines a method of obtaining underlying asset data for a scope
pub struct AccessRoute {
    /// A path to a resource that can provide underlying asset data for a scope.  Can be anything:
    /// http path, grpc, etc
    pub route: String,
    /// An optional name parameter, allowing the creator of the route to give it a definition or
    /// to label it for external resources to identify it
    pub name: Option<String>,
}
impl AccessRoute {
    pub fn new<S1: Into<String>, S2: Into<String>>(route: S1, name: Option<S2>) -> Self {
        Self {
            route: route.into(),
            name: name.map(|s| s.into()),
        }
    }

    pub fn route_only<S: Into<String>>(route: S) -> Self {
        Self {
            route: route.into(),
            name: None,
        }
    }

    pub fn route_and_name<S1: Into<String>, S2: Into<String>>(route: S1, name: S2) -> Self {
        Self {
            route: route.into(),
            name: name.into().to_some(),
        }
    }

    pub fn trim_values(self) -> Self {
        Self {
            route: self.route.trim().to_string(),
            name: self.name.map(|s| s.trim().to_string()),
        }
    }
}
