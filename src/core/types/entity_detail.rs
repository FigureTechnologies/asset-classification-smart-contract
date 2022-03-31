use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EntityDetail {
    /// A short name describing the entity
    pub name: Option<String>,
    /// A short description of the entity's purpose
    pub description: Option<String>,
    /// A web link that can send observers to a location that the verifier belongs to
    pub web_address: Option<String>,
}
impl EntityDetail {
    pub fn new<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        name: S1,
        description: S2,
        web_address: S3,
    ) -> Self {
        Self {
            name: Some(name.into()),
            description: Some(description.into()),
            web_address: Some(web_address.into()),
        }
    }
}
