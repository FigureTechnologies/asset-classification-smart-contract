use super::constants::{
    ASSET_EVENT_TYPE_KEY, ASSET_SCOPE_ADDRESS_KEY, ASSET_TYPE_KEY, NEW_VALUE_KEY, SCOPE_OWNER_KEY,
    VERIFIER_ADDRESS_KEY,
};
use crate::util::constants::ADDITIONAL_METADATA_KEY;
use std::collections::HashMap;

/// An enum that contains all different event types that can occur throughout the [contract's](crate::contract)
/// routes.
pub enum EventType {
    /// Occurs when the contract is [instantiated](crate::contract::instantiate) with [instantiate](crate::instantiate::init_contract).
    InstantiateContract,
    /// Occurs when the contract is [migrated](crate::contract::migrate) with [migrate](crate::contract::migrate).
    MigrateContract,
    /// Occurs when the contract is [executed](crate::contract::execute) to [onboard an asset](crate::execute::onboard_asset).
    OnboardAsset,
    /// Occurs when the contract is [executed](crate::contract::execute) to [verify an asset](crate::execute::verify_asset).
    VerifyAsset,
    /// Occurs when the contract is [executed](crate::contract::execute) to [finalize asset classification](crate::execute::finalize_classification).
    FinalizeClassification,
    /// Occurs when the contract is [executed](crate::contract::execute) to [add an asset definition](crate::execute::add_asset_definition).
    AddAssetDefinition,
    /// Occurs when the contract is [executed](crate::contract::execute) to [update an asset definition](crate::execute::update_asset_definition).
    UpdateAssetDefinition,
    /// Occurs when the contract is [executed](crate::contract::execute) to [toggle an asset definition](crate::execute::toggle_asset_definition).
    ToggleAssetDefinition,
    /// Occurs when the contract is [executed](crate::contract::execute) to [add an asset verifier detail](crate::execute::add_asset_verifier).
    AddAssetVerifier,
    /// Occurs when the contract is [executed](crate::contract::execute) to [update an asset verifier detail](crate::execute::update_asset_verifier).
    UpdateAssetVerifier,
    /// Occurs when the contract is [executed](crate::contract::execute) to [update access routes](crate::execute::update_access_routes).
    UpdateAccessRoutes,
    /// Occurs when the contract is [executed](crate::contract::execute) to [delete an asset definition](crate::execute::delete_asset_definition).
    DeleteAssetDefinition,
}
#[allow(clippy::from_over_into)]
impl Into<String> for EventType {
    fn into(self) -> String {
        match self {
            EventType::InstantiateContract => "instantiate_contract",
            EventType::MigrateContract => "migrate_contract",
            EventType::OnboardAsset => "onboard_asset",
            EventType::VerifyAsset => "verify_asset",
            EventType::FinalizeClassification => "finalize_classification",
            EventType::AddAssetDefinition => "add_asset_definition",
            EventType::UpdateAssetDefinition => "update_asset_definition",
            EventType::ToggleAssetDefinition => "toggle_asset_definition",
            EventType::AddAssetVerifier => "add_asset_verifier",
            EventType::UpdateAssetVerifier => "update_asset_verifier",
            EventType::UpdateAccessRoutes => "update_access_routes",
            EventType::DeleteAssetDefinition => "delete_asset_definition",
        }
        .into()
    }
}
impl EventType {
    /// Utilizes the implementation of Into<String> to automatically derive the event name.  This
    /// allows an invocation without an explicit type declaration.
    pub fn event_name(self) -> String {
        self.into()
    }
}

/// A helper struct to emit attributes for a [Response](cosmwasm_std::Response).
pub struct EventAttributes {
    /// All generated attributes as tuples, which can easily be used to add into a [Response](cosmwasm_std::Response).
    attributes: Vec<(String, String)>,
}
impl EventAttributes {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `event_type` All events should denote their type for external consumers of Provenance
    /// Blockchain Event Stream, so this value is required for any new instance and appends the
    /// name of the event with the key of [ASSET_EVENT_TYPE_KEY](super::constants::ASSET_EVENT_TYPE_KEY).
    pub fn new(event_type: EventType) -> Self {
        EventAttributes {
            attributes: vec![(ASSET_EVENT_TYPE_KEY.into(), event_type.into())],
        }
    }

    /// Certain events like [onboard_asset](crate::execute::onboard_asset::onboard_asset) require a
    /// standard set of event types.  This is a constructor for the struct that includes those
    /// values to facilitate the process of generating all events.
    ///
    /// # Parameters
    ///
    /// * `event_type` All events should denote their type for external consumers of Provenance
    /// Blockchain Event Stream, so this value is required for any new instance and appends the
    /// name of the event with the key of [ASSET_EVENT_TYPE_KEY](super::constants::ASSET_EVENT_TYPE_KEY).
    /// * `asset_type` A unique key for an [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
    /// that uses the key [ASSET_TYPE_KEY](super::constants::ASSET_TYPE_KEY).
    /// * `scope_address` A unique key for an [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
    /// that uses the key [ASSET_SCOPE_ADDRESS_KEY](super::constants::ASSET_SCOPE_ADDRESS_KEY).
    pub fn for_asset_event<T1: Into<String>, T2: Into<String>>(
        event_type: EventType,
        asset_type: T1,
        scope_address: T2,
    ) -> Self {
        Self::new(event_type)
            .set_asset_type(asset_type)
            .set_scope_address(scope_address)
    }

    /// Appends an asset type value to an existing [EventAttributes](self::EventAttributes) and
    /// returns the same instance to create a functional chain for further attribute addition.
    ///
    /// # Parameters
    ///
    /// * `asset_type` A unique key for an [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
    /// that uses the key [ASSET_TYPE_KEY](super::constants::ASSET_TYPE_KEY).
    pub fn set_asset_type<T: Into<String>>(mut self, asset_type: T) -> Self {
        self.attributes
            .push((ASSET_TYPE_KEY.into(), asset_type.into()));
        self
    }

    /// Appends a scope address bech32 value to an existing [EventAttributes](self::EventAttributes) and
    /// returns the same instance to create a functional chain for further attribute addition.
    ///
    /// # Parameters
    ///
    /// * `scope_address` A unique key for an [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
    /// that uses the key [ASSET_SCOPE_ADDRESS_KEY](super::constants::ASSET_SCOPE_ADDRESS_KEY).
    pub fn set_scope_address<T: Into<String>>(mut self, scope_address: T) -> Self {
        self.attributes
            .push((ASSET_SCOPE_ADDRESS_KEY.into(), scope_address.into()));
        self
    }

    /// Appends a verifier address bech32 value to an existing [EventAttributes](self::EventAttributes) and
    /// returns the same instance to create a functional chain for further attribute addition.
    ///
    /// # Parameters
    ///
    /// * `verifier_address` The [address](crate::core::types::verifier_detail::VerifierDetailV2::address)
    /// for a [VerifierDetailV2](crate::core::types::verifier_detail::VerifierDetailV2) that uses the
    /// key [VERIFIER_ADDRESS_KEY](super::constants::VERIFIER_ADDRESS_KEY).
    pub fn set_verifier<T: Into<String>>(mut self, verifier_address: T) -> Self {
        self.attributes
            .push((VERIFIER_ADDRESS_KEY.into(), verifier_address.into()));
        self
    }

    /// Appends a dynamic value to an existing [EventAttributes](self::EventAttributes) and
    /// returns the same instance to create a functional chain for further attribute addition.
    ///
    /// # Parameters
    ///
    /// * `new_value` Any dynamic value that pertains to the current execution process, using the
    /// key [NEW_VALUE_KEY](super::constants::NEW_VALUE_KEY).
    pub fn set_new_value<T: ToString>(mut self, new_value: T) -> Self {
        self.attributes
            .push((NEW_VALUE_KEY.into(), new_value.to_string()));
        self
    }

    /// Appends a scope owner bech32 value to an existing [EventAttributes](self::EventAttributes) and
    /// returns the same instance to create a functional chain for further attribute addition.
    ///
    /// # Parameters
    ///
    /// * `scope_owner` A bech32 address that owns a Provenance Metadata Scope referred to by the
    /// current execution process, appended with the key [SCOPE_OWNER_KEY](super::constants::SCOPE_OWNER_KEY).
    pub fn set_scope_owner<T: ToString>(mut self, scope_owner: T) -> Self {
        self.attributes
            .push((SCOPE_OWNER_KEY.into(), scope_owner.to_string()));
        self
    }

    /// Appends a dynamic set of additional metadata to an existing [EventAttributes](self::EventAttributes)
    /// and returns the same instance to create a functional chain for further attribute addition.
    /// Note: If the metadata provided is empty, this key will be skipped to prevent strange value
    /// displays to external consumers.
    ///
    /// # Parameters
    ///
    /// * `additional_metadata` An instance of additional metadata to be displayed to any external
    /// consumers.  Uses the key of [ADDITIONAL_METADATA_KEY](super::constants::ADDITIONAL_METADATA_KEY).
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
/// that don't necessarily need to specify a large amount of new event keys.  Emitted values are
/// aggregated and sorted deterministically, and then displayed using the format:
/// \[a=1, b=2, c=3, etc\]
pub struct EventAdditionalMetadata {
    /// An internal collection of all added metadata.
    fields: HashMap<String, String>,
}
impl EventAdditionalMetadata {
    /// Constructs a new instance of this struct with an empty fields set.
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Returns `true` only if metadata fields have been added with the [add_metadata](self::EventAdditionalMetadata::add_metadata)
    /// function.
    pub fn has_metadata(&self) -> bool {
        !self.fields.is_empty()
    }

    /// Appends a new key and value pair to the internal fields value.
    ///
    /// # Parameters
    ///
    /// * `key` The string key that will be displayed before the = sign in the display.
    /// * `value` The string value that will be displayed after the = sign in the display.
    pub fn add_metadata<S1: Into<String>, S2: Into<String>>(&mut self, key: S1, value: S2) {
        self.fields.insert(key.into(), value.into());
    }

    /// Aggregates and deterministically sorts the internal values, resulting in a display string
    /// for adding as an event attribute in the format: \[a=1, b=2, c=3, etc\]
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
