use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Various fields describing an entity, which could be an organization, account, etc.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EntityDetail {
    /// A short name describing the entity.
    pub name: Option<String>,
    /// A short description of the entity's purpose.
    pub description: Option<String>,
    /// A web link that can send observers to the organization that the entity belongs to.
    pub home_url: Option<String>,
    /// A web link that can send observers to the source code of the entity for increased transparency.
    pub source_url: Option<String>,
}
impl EntityDetail {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `name` A short name describing the entity.
    /// * `description` A short description of the entity's purpose.
    /// * `home_url` A web link that can send observers to the organization that the entity belongs to.
    /// * `source_url` A web link that can send observers to the source code of the entity for increased transparency.
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
