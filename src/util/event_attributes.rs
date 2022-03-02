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
    pub fn new<T: Into<String>>(event_type: EventType, asset_type: T, asset_uuid: T) -> Self {
        EventAttributes {
            attributes: vec![
                (ASSET_EVENT_TYPE_KEY.into(), event_type.into()),
                (ASSET_TYPE_KEY.into(), asset_type.into()),
                (ASSET_UUID_KEY.into(), asset_uuid.into()),
            ],
        }
    }

    pub fn set_validator(mut self, validator_address: String) -> Self {
        self.attributes
            .push((VALIDATOR_ADDRESS_KEY.into(), validator_address));
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
