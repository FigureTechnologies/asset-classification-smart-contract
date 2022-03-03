use super::constants::{
    ASSET_EVENT_TYPE_KEY, ASSET_SCOPE_ADDRESS_KEY, ASSET_TYPE_KEY, NEW_VALUE_KEY,
    VALIDATOR_ADDRESS_KEY,
};

pub enum EventType {
    OnboardAsset,
    ValidateAsset,
    AddAssetDefinition,
    UpdateAssetDefinition,
    ToggleAssetDefinition,
    AddAssetValidator,
    UpdateAssetValidator,
}
#[allow(clippy::from_over_into)]
impl Into<String> for EventType {
    fn into(self) -> String {
        match self {
            EventType::OnboardAsset => "onboard_asset",
            EventType::ValidateAsset => "validate_asset",
            EventType::AddAssetDefinition => "add_asset_definition",
            EventType::UpdateAssetDefinition => "update_asset_definition",
            EventType::ToggleAssetDefinition => "toggle_asset_definition",
            EventType::AddAssetValidator => "add_asset_validator",
            EventType::UpdateAssetValidator => "update_asset_validator",
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

    pub fn set_validator<T: Into<String>>(mut self, validator_address: T) -> Self {
        self.attributes
            .push((VALIDATOR_ADDRESS_KEY.into(), validator_address.into()));
        self
    }

    pub fn set_new_value<T: ToString>(mut self, new_value: T) -> Self {
        self.attributes
            .push((NEW_VALUE_KEY.into(), new_value.to_string()));
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

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::Response;

    use crate::{
        testutil::test_utilities::single_attribute_for_key,
        util::constants::{
            ASSET_EVENT_TYPE_KEY, ASSET_SCOPE_ADDRESS_KEY, ASSET_TYPE_KEY, NEW_VALUE_KEY,
            VALIDATOR_ADDRESS_KEY,
        },
    };

    use super::{EventAttributes, EventType};

    #[test]
    fn test_response_consumption() {
        let attributes = EventAttributes::new(EventType::OnboardAsset)
            .set_asset_type("asset type")
            .set_scope_address("scope address")
            .set_validator("validator address")
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
            "validator address",
            single_attribute_for_key(&response, VALIDATOR_ADDRESS_KEY),
            "the validator address attribute should be added correctly",
        );
        assert_eq!(
            "new value",
            single_attribute_for_key(&response, NEW_VALUE_KEY),
            "the new value attribute should be added correctly",
        );
    }
}