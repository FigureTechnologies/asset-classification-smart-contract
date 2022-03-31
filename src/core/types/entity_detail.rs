use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EntityDetail {
    /// A short name describing the entity
    pub name: Option<String>,
    /// A short description of the entity's purpose
    pub description: Option<String>,
    /// A web link that can send observers to the organization that the verifier belongs to
    pub home_url: Option<String>,
    // A web link that can send observers to the source code of the verifier, for increased transparency
    pub source_url: Option<String>,
}
impl EntityDetail {
    pub fn new<S1: Into<String>, S2: Into<String>, S3: Into<String>, S4: Into<String>>(
        name: S1,
        description: S2,
        home_url: S3,
        source_url: S4,
    ) -> Self {
        Self {
            name: Some(name.into()),
            description: Some(description.into()),
            home_url: Some(home_url.into()),
            source_url: Some(source_url.into()),
        }
    }
}
