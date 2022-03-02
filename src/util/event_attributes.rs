use super::constants::{
    ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY, ASSET_UUID_KEY, VALIDATOR_ADDRESS_KEY,
};

pub enum EventType {
    OnboardAsset,
}
impl Into<String> for EventType {
    fn into(self) -> String {
        match self {
            EventType::OnboardAsset => "onboard_asset",
        }
        .into()
    }
}

pub struct EventAttributes {
    attributes: Vec<(String, String)>,
}
impl EventAttributes {
    pub fn new<T1: Into<String>, T2: Into<String>>(
        event_type: EventType,
        asset_type: T1,
        asset_uuid: T2,
    ) -> Self {
        EventAttributes {
            attributes: vec![
                (ASSET_EVENT_TYPE_KEY.into(), event_type.into()),
                (ASSET_TYPE_KEY.into(), asset_type.into()),
                (ASSET_UUID_KEY.into(), asset_uuid.into()),
            ],
        }
    }

    pub fn set_validator<T: Into<String>>(mut self, validator_address: T) -> Self {
        self.attributes
            .push((VALIDATOR_ADDRESS_KEY.into(), validator_address.into()));
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
