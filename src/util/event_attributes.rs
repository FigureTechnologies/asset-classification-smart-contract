use super::constants::{
    ASSET_EVENT_TYPE_KEY, ASSET_SCOPE_ADDRESS_KEY, ASSET_TYPE_KEY, NEW_VALUE_KEY, SCOPE_OWNER_KEY,
    VERIFIER_ADDRESS_KEY,
};
use crate::util::constants::ADDITIONAL_METADATA_KEY;
use std::collections::HashMap;

pub enum EventType {
    InstantiateContract,
    MigrateContract,
    OnboardAsset,
    VerifyAsset,
    AddAssetDefinition,
    UpdateAssetDefinition,
    ToggleAssetDefinition,
    AddAssetVerifier,
    UpdateAssetVerifier,
    UpdateAccessRoutes,
    BindContractAlias,
}
#[allow(clippy::from_over_into)]
impl Into<String> for EventType {
    fn into(self) -> String {
        match self {
            EventType::InstantiateContract => "instantiate_contract",
            EventType::MigrateContract => "migrate_contract",
            EventType::OnboardAsset => "onboard_asset",
            EventType::VerifyAsset => "verify_asset",
            EventType::AddAssetDefinition => "add_asset_definition",
            EventType::UpdateAssetDefinition => "update_asset_definition",
            EventType::ToggleAssetDefinition => "toggle_asset_definition",
            EventType::AddAssetVerifier => "add_asset_verifier",
            EventType::UpdateAssetVerifier => "update_asset_verifier",
            EventType::UpdateAccessRoutes => "update_access_routes",
            EventType::BindContractAlias => "bind_contract_alias",
        }
        .into()
    }
}
impl EventType {
    pub fn event_name(self) -> String {
        self.into()
    }
}

pub struct EventAttributes {
    attributes: Vec<(String, String)>,
}
impl EventAttributes {
    pub fn new(event_type: EventType) -> Self {
        EventAttributes {
            attributes: vec![(ASSET_EVENT_TYPE_KEY.into(), event_type.into())],
        }
    }

    pub fn for_asset_event<T1: Into<String>, T2: Into<String>>(
        event_type: EventType,
        asset_type: T1,
        scope_address: T2,
    ) -> Self {
        Self::new(event_type)
            .set_asset_type(asset_type)
            .set_scope_address(scope_address)
    }

    pub fn set_asset_type<T: Into<String>>(mut self, asset_type: T) -> Self {
        self.attributes
            .push((ASSET_TYPE_KEY.into(), asset_type.into()));
        self
    }

    pub fn set_scope_address<T: Into<String>>(mut self, scope_address: T) -> Self {
        self.attributes
            .push((ASSET_SCOPE_ADDRESS_KEY.into(), scope_address.into()));
        self
    }

    pub fn set_verifier<T: Into<String>>(mut self, verifier_address: T) -> Self {
        self.attributes
            .push((VERIFIER_ADDRESS_KEY.into(), verifier_address.into()));
        self
    }

    pub fn set_new_value<T: ToString>(mut self, new_value: T) -> Self {
        self.attributes
            .push((NEW_VALUE_KEY.into(), new_value.to_string()));
        self
    }

    pub fn set_scope_owner<T: ToString>(mut self, scope_owner: T) -> Self {
        self.attributes
            .push((SCOPE_OWNER_KEY.into(), scope_owner.to_string()));
        self
    }

    pub fn set_additional_metadata(
        mut self,
        additional_metadata: &EventAdditionalMetadata,
    ) -> Self {
        // Only append additional metadata if it actually has keys
        if additional_metadata.has_metadata() {
            self.attributes.push((
                ADDITIONAL_METADATA_KEY.into(),
                additional_metadata.get_meta_string(),
            ));
        }
        self
    }
}

impl IntoIterator for EventAttributes {
    type Item = (String, String);

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.attributes.into_iter()
    }
}

/// A helper collection that allows underlying processes to specify dynamic key values for processes
/// that don't necessarily need to specify a large amount of new event keys.
pub struct EventAdditionalMetadata {
    fields: HashMap<String, String>,
}
impl EventAdditionalMetadata {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    pub fn has_metadata(&self) -> bool {
        !self.fields.is_empty()
    }

    pub fn add_metadata<S1: Into<String>, S2: Into<String>>(&mut self, key: S1, value: S2) {
        self.fields.insert(key.into(), value.into());
    }

    pub fn get_meta_string(&self) -> String {
        let mut map_displays = self
            .fields
            .iter()
            .map(|(key, value)| format!("[{key}={value}]"))
            .collect::<Vec<_>>();
        // Keep the collection sorted to ensure that output is deterministic
        map_displays.sort();
        map_displays.join(", ")
    }
}
impl Default for EventAdditionalMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::Response;

    use crate::util::event_attributes::EventAdditionalMetadata;
    use crate::{
        testutil::test_utilities::single_attribute_for_key,
        util::constants::{
            ASSET_EVENT_TYPE_KEY, ASSET_SCOPE_ADDRESS_KEY, ASSET_TYPE_KEY, NEW_VALUE_KEY,
            VERIFIER_ADDRESS_KEY,
        },
    };

    use super::{EventAttributes, EventType};

    #[test]
    fn test_response_consumption() {
        let attributes = EventAttributes::new(EventType::OnboardAsset)
            .set_asset_type("asset type")
            .set_scope_address("scope address")
            .set_verifier("verifier address")
            .set_new_value("new value");
        let response: Response<String> = Response::new().add_attributes(attributes);
        assert_eq!(
            "onboard_asset",
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "the event type attribute should be added correctly",
        );
        assert_eq!(
            "asset type",
            single_attribute_for_key(&response, ASSET_TYPE_KEY),
            "the asset type attribute should be added correctly",
        );
        assert_eq!(
            "scope address",
            single_attribute_for_key(&response, ASSET_SCOPE_ADDRESS_KEY),
            "the scope address attribute should be added correctly",
        );
        assert_eq!(
            "verifier address",
            single_attribute_for_key(&response, VERIFIER_ADDRESS_KEY),
            "the verifier address attribute should be added correctly",
        );
        assert_eq!(
            "new value",
            single_attribute_for_key(&response, NEW_VALUE_KEY),
            "the new value attribute should be added correctly",
        );
    }

    #[test]
    fn test_additional_metadata_string_output() {
        let mut metadata = EventAdditionalMetadata::new();
        assert_eq!(
            "",
            metadata.get_meta_string(),
            "expected no output to be derived when no metadata has been added",
        );
        metadata.add_metadata("b", "b_value");
        assert_eq!(
            "[b=b_value]",
            metadata.get_meta_string(),
            "expected the key/value addition to display properly",
        );
        metadata.add_metadata("a", "a_value");
        assert_eq!(
            "[a=a_value], [b=b_value]",
            metadata.get_meta_string(),
            "expected the second key/value addition to also display alongside the first, alphabetically sorted",
        );
        metadata.add_metadata("c", "c_value");
        assert_eq!(
            "[a=a_value], [b=b_value], [c=c_value]",
            metadata.get_meta_string(),
            "expected the third key/value addition to also display alongside the first two, alphabetically sorted",
        );
    }
}
