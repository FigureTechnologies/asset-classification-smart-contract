use crate::core::error::ContractError;
use crate::core::types::serialized_enum::SerializedEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::util::{
    aliases::AssetResult,
    scope_address_utils::{asset_uuid_to_scope_address, scope_address_to_asset_uuid},
    traits::ResultExtensions,
};

const ASSET_UUID_NAME: &str = "asset_uuid";
const SCOPE_ADDRESS_NAME: &str = "scope_address";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetIdentifier {
    AssetUuid(String),
    ScopeAddress(String),
}
impl AssetIdentifier {
    pub fn from_serialized_enum(e: &SerializedEnum) -> AssetResult<Self> {
        match e.enum_type.as_str() {
            ASSET_UUID_NAME => Self::asset_uuid(&e.value).to_ok(),
            SCOPE_ADDRESS_NAME => Self::scope_address(&e.value).to_ok(),
            _ => ContractError::UnexpectedSerializedEnum {
                received_type: e.enum_type.clone(),
                explanation: format!("Invalid AssetIdentifier. Expected one of [{ASSET_UUID_NAME}, {SCOPE_ADDRESS_NAME}]"),
            }
            .to_err(),
        }
    }

    pub fn to_serialized_enum(&self) -> SerializedEnum {
        match self {
            Self::AssetUuid(uuid) => SerializedEnum::new(ASSET_UUID_NAME, uuid),
            Self::ScopeAddress(address) => SerializedEnum::new(SCOPE_ADDRESS_NAME, address),
        }
    }

    pub fn asset_uuid<S: Into<String>>(asset_uuid: S) -> Self {
        Self::AssetUuid(asset_uuid.into())
    }

    pub fn scope_address<S: Into<String>>(scope_address: S) -> Self {
        Self::ScopeAddress(scope_address.into())
    }

    pub fn get_asset_uuid(&self) -> AssetResult<String> {
        match self {
            Self::AssetUuid(asset_uuid) => (*asset_uuid).clone().to_ok(),
            Self::ScopeAddress(scope_address) => scope_address_to_asset_uuid(scope_address),
        }
    }

    pub fn get_scope_address(&self) -> AssetResult<String> {
        match self {
            Self::AssetUuid(asset_uuid) => asset_uuid_to_scope_address(asset_uuid),
            Self::ScopeAddress(scope_address) => (*scope_address).clone().to_ok(),
        }
    }

    /// Takes the value provided and derives both values from it, where necessary,
    /// ensuring that both asset_uuid and scope_address are available to the user
    pub fn to_identifiers(&self) -> AssetResult<AssetIdentifiers> {
        AssetIdentifiers::new(self.get_asset_uuid()?, self.get_scope_address()?).to_ok()
    }
}

/// A simple named collection of both the asset uuid and scope address
pub struct AssetIdentifiers {
    pub asset_uuid: String,
    pub scope_address: String,
}
impl AssetIdentifiers {
    pub fn new<S1: Into<String>, S2: Into<String>>(asset_uuid: S1, scope_address: S2) -> Self {
        Self {
            asset_uuid: asset_uuid.into(),
            scope_address: scope_address.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::error::ContractError;
    use crate::core::types::asset_identifier::{
        AssetIdentifier, ASSET_UUID_NAME, SCOPE_ADDRESS_NAME,
    };
    use crate::core::types::serialized_enum::SerializedEnum;
    use uuid::Uuid;

    #[test]
    fn test_asset_identifier_parse_for_asset_uuid() {
        // The uuid was generated randomly and the scope address was derived via provenance's MetadataAddress util
        let asset_uuid = "0c39efb6-9fef-11ec-ab21-6bf5c9fb3f83";
        let expected_scope_address = "scope1qqxrnmaknlh3rm9ty94ltj0m87psnapt5l";
        let identifier = AssetIdentifier::asset_uuid(asset_uuid);
        let result_identifiers = identifier
            .to_identifiers()
            .expect("parsing idenitifiers should succeed");
        assert_eq!(
            asset_uuid,
            result_identifiers.asset_uuid.as_str(),
            "expected the asset uuid value to pass through successfully",
        );
        assert_eq!(
            expected_scope_address,
            result_identifiers.scope_address.as_str(),
            "expected the scope address to be derived correctly",
        );
    }

    #[test]
    fn test_asset_identifier_parse_for_scope_address() {
        // The uuid was generated randomly and the scope address was derived via provenance's MetadataAddress util
        let scope_address = "scope1qz3s7dvsnlh3rmyy3pm5tszs2v7qhwhde8";
        let expected_asset_uuid = "a30f3590-9fef-11ec-8488-7745c050533c";
        let identifier = AssetIdentifier::scope_address(scope_address);
        let result_identifiers = identifier
            .to_identifiers()
            .expect("parsing identifiers should succeed");
        assert_eq!(
            scope_address,
            result_identifiers.scope_address.as_str(),
            "expected the scope address to pass through successfully",
        );
        assert_eq!(
            expected_asset_uuid,
            result_identifiers.asset_uuid.as_str(),
            "expected the asset uuid to be derived correctly",
        );
    }

    #[test]
    fn test_asset_identifier_to_functions_from_asset_uuid() {
        let initial_uuid = "5134f836-a15c-11ec-abb6-a733aad66af8";
        let expected_scope_address = "scope1qpgnf7pk59wprm9tk6nn82kkdtuq2wlq5p";
        let identifier = AssetIdentifier::asset_uuid(initial_uuid);
        let asset_uuid = identifier
            .get_asset_uuid()
            .expect("the asset uuid should be directly accessible");
        let scope_address = identifier
            .get_scope_address()
            .expect("the scope address should be accessible by conversion");
        assert_eq!(
            initial_uuid, asset_uuid,
            "the asset uuid output should be identical to the input"
        );
        assert_eq!(
            expected_scope_address, scope_address,
            "the scope address output should be as expected"
        );
    }

    #[test]
    fn test_asset_identifier_to_functions_from_scope_address() {
        let initial_address = "scope1qzdyhglu59w3rm9n0z0h3mn657yqrgjcwl";
        let expected_asset_uuid = "9a4ba3fc-a15d-11ec-b378-9f78ee7aa788";
        let identifier = AssetIdentifier::scope_address(initial_address);
        let scope_address = identifier
            .get_scope_address()
            .expect("the scope address should be directly accessible");
        let asset_uuid = identifier
            .get_asset_uuid()
            .expect("the asset uuid should be accessible by conversion");
        assert_eq!(
            initial_address, scope_address,
            "the scope address output should be identical to the input"
        );
        assert_eq!(
            expected_asset_uuid, asset_uuid,
            "the asset uuid output should be as expected"
        );
    }

    #[test]
    fn test_from_serialized_enum_asset_uuid() {
        let uuid = Uuid::new_v4().to_string();
        let ser_enum = SerializedEnum::new(ASSET_UUID_NAME, &uuid);
        let identifier = AssetIdentifier::from_serialized_enum(&ser_enum)
            .expect("expected serialized enum to identifier to succeed");
        match identifier {
            AssetIdentifier::AssetUuid(asset_uuid) => {
                assert_eq!(
                    uuid, asset_uuid,
                    "expected the asset uuid to be properly derived",
                );
            }
            _ => panic!("unexpected identifier derived from type {:?}", identifier),
        };
    }

    #[test]
    fn test_from_serialized_enum_to_scope_address() {
        let ser_enum = SerializedEnum::new(SCOPE_ADDRESS_NAME, "my-address");
        let identifier = AssetIdentifier::from_serialized_enum(&ser_enum)
            .expect("expected serialized enum to identifier to succeed");
        match identifier {
            AssetIdentifier::ScopeAddress(scope_address) => {
                assert_eq!(
                    "my-address", scope_address,
                    "expected the scope address to be properly derived",
                );
            }
            _ => panic!("unexpected identifier derived from type {:?}", identifier),
        };
    }

    #[test]
    fn test_from_serialized_enum_wrong_type_error() {
        let ser_enum = SerializedEnum::new("bad_type", "some_value");
        let err = AssetIdentifier::from_serialized_enum(&ser_enum)
            .expect_err("expected an incompatible type to cause an error");
        match err {
            ContractError::UnexpectedSerializedEnum {
                received_type,
                explanation,
            } => {
                assert_eq!(
                    "bad_type", received_type,
                    "expected the unexpected type to be provided in the error message",
                );
                assert_eq!(
                    format!("Invalid AssetIdentifier. Expected one of [{ASSET_UUID_NAME}, {SCOPE_ADDRESS_NAME}]"),
                    explanation,
                    "expected the explanation to list the type of the enum and the expected values",
                );
            }
            _ => panic!(
                "unexpected error encountered on bad type provided: {:?}",
                err
            ),
        };
    }

    #[test]
    fn test_to_serialized_enum_asset_uuid() {
        let uuid = Uuid::new_v4().to_string();
        let asset_uuid = AssetIdentifier::asset_uuid(&uuid);
        let ser_enum = asset_uuid.to_serialized_enum();
        assert_eq!(
            ASSET_UUID_NAME, ser_enum.enum_type,
            "expected the proper enum type to be derived",
        );
        assert_eq!(
            uuid, ser_enum.value,
            "expected the proper value to be derived",
        );
    }

    #[test]
    fn test_to_serialized_enum_scope_address() {
        let scope_address = AssetIdentifier::scope_address("my-address");
        let ser_enum = scope_address.to_serialized_enum();
        assert_eq!(
            SCOPE_ADDRESS_NAME, ser_enum.enum_type,
            "expected the proper enum type to be derived",
        );
        assert_eq!(
            "my-address", ser_enum.value,
            "expected the proper value to be derived",
        );
    }
}
