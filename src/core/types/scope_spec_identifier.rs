use crate::core::error::ContractError;
use crate::core::types::serialized_enum::SerializedEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::util::{
    aliases::AssetResult,
    scope_address_utils::{
        scope_spec_address_to_scope_spec_uuid, scope_spec_uuid_to_scope_spec_address,
    },
    traits::ResultExtensions,
};

const UUID_NAME: &str = "uuid";
const ADDRESS_NAME: &str = "address";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScopeSpecIdentifier {
    Uuid(String),
    Address(String),
}
impl ScopeSpecIdentifier {
    pub fn from_serialized_enum(e: &SerializedEnum) -> AssetResult<Self> {
        match e.r#type.as_str() {
            UUID_NAME => Self::uuid(&e.value).to_ok(),
            ADDRESS_NAME => Self::address(&e.value).to_ok(),
            _ => ContractError::UnexpectedSerializedEnum {
                received_type: e.r#type.clone(),
                explanation: format!(
                    "Invalid ScopeSpecIdentifier. Expected one of [{UUID_NAME}, {ADDRESS_NAME}]"
                ),
            }
            .to_err(),
        }
    }

    pub fn to_serialized_enum(&self) -> SerializedEnum {
        match self {
            Self::Uuid(uuid) => SerializedEnum::new(UUID_NAME, uuid),
            Self::Address(address) => SerializedEnum::new(ADDRESS_NAME, address),
        }
    }

    pub fn uuid<S: Into<String>>(scope_spec_uuid: S) -> Self {
        Self::Uuid(scope_spec_uuid.into())
    }

    pub fn address<S: Into<String>>(scope_spec_address: S) -> Self {
        Self::Address(scope_spec_address.into())
    }

    pub fn get_scope_spec_uuid(&self) -> AssetResult<String> {
        match self {
            Self::Uuid(scope_spec_uuid) => (*scope_spec_uuid).clone().to_ok(),
            Self::Address(scope_spec_address) => {
                scope_spec_address_to_scope_spec_uuid(scope_spec_address)
            }
        }
    }

    pub fn get_scope_spec_address(&self) -> AssetResult<String> {
        match self {
            Self::Uuid(scope_spec_uuid) => scope_spec_uuid_to_scope_spec_address(scope_spec_uuid),
            Self::Address(scope_spec_address) => (*scope_spec_address).clone().to_ok(),
        }
    }

    /// Takes the value provided and dervies both values from it, where necessary,
    /// ensuring that both scope_spec_uuid and scope_spec_address are available to the user
    pub fn to_identifiers(&self) -> AssetResult<ScopeSpecIdentifiers> {
        ScopeSpecIdentifiers::new(self.get_scope_spec_uuid()?, self.get_scope_spec_address()?)
            .to_ok()
    }
}

pub struct ScopeSpecIdentifiers {
    pub scope_spec_uuid: String,
    pub scope_spec_address: String,
}
impl ScopeSpecIdentifiers {
    pub fn new<S1, S2>(scope_spec_uuid: S1, scope_spec_address: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            scope_spec_uuid: scope_spec_uuid.into(),
            scope_spec_address: scope_spec_address.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::error::ContractError;
    use crate::core::types::scope_spec_identifier::{ScopeSpecIdentifier, ADDRESS_NAME, UUID_NAME};
    use crate::core::types::serialized_enum::SerializedEnum;
    use uuid::Uuid;

    #[test]
    fn test_scope_spec_identifier_parse_for_scope_spec_uuid() {
        // The uuid was generated randomly and the scope spec address was derived via provenance's MetadataAddress util
        let scope_spec_uuid = "35398a4f-fd44-4737-ba01-f1c46ca62257";
        let expected_scope_spec_address = "scopespec1qs6nnzj0l4zywda6q8cugm9xyfts8xugze";
        let identifier = ScopeSpecIdentifier::uuid(scope_spec_uuid);
        let result_identifiers = identifier
            .to_identifiers()
            .expect("parsing identifiers should succeed");
        assert_eq!(
            scope_spec_uuid,
            result_identifiers.scope_spec_uuid.as_str(),
            "expected the scope spec uuid value to pass through successfully",
        );
        assert_eq!(
            expected_scope_spec_address,
            result_identifiers.scope_spec_address.as_str(),
            "expected the scope spec address to be derived correctly",
        );
    }

    #[test]
    fn test_scope_spec_identifier_parse_for_scope_spec_address() {
        // The uuid was generated randomly and the scope spec address was derived via provenance's MetadataAddress util
        let scope_spec_address = "scopespec1q3vehdq7dn25ar9y53llsaslcglqh43r38";
        let expected_scope_spec_uuid = "599bb41e-6cd5-4e8c-a4a4-7ff8761fc23e";
        let identifier = ScopeSpecIdentifier::address(scope_spec_address);
        let result_identifiers = identifier
            .to_identifiers()
            .expect("parsing identifiers should succeed");
        assert_eq!(
            scope_spec_address,
            result_identifiers.scope_spec_address.as_str(),
            "expected the scope spec address to pass through successfully",
        );
        assert_eq!(
            expected_scope_spec_uuid,
            result_identifiers.scope_spec_uuid.as_str(),
            "expected the scope spec uuid to be derived correctly",
        );
    }

    #[test]
    fn test_scope_spec_identifier_to_functions_from_scope_spec_uuid() {
        let initial_uuid = "a2d0ff08-1f01-4209-bdac-d8dea2487d7a";
        let expected_scope_spec_address = "scopespec1qj3dplcgruq5yzda4nvdagjg04aq93tuxs";
        let identifier = ScopeSpecIdentifier::uuid(initial_uuid);
        let scope_spec_uuid = identifier
            .get_scope_spec_uuid()
            .expect("the scope spec uuid should be directly accessible");
        let scope_spec_address = identifier
            .get_scope_spec_address()
            .expect("the scope spec address should be accessible by conversion");
        assert_eq!(
            initial_uuid, scope_spec_uuid,
            "the scope spec uuid output should be identical to the input",
        );
        assert_eq!(
            expected_scope_spec_address, scope_spec_address,
            "the scope spec address output should be as expected",
        );
    }

    #[test]
    fn test_scope_spec_identifier_to_functions_from_scope_spec_address() {
        let initial_address = "scopespec1q3ptevdt2x5yg5ycflqjsky8rz5q47e34p";
        let expected_scope_spec_uuid = "42bcb1ab-51a8-4450-984f-c128588718a8";
        let identifier = ScopeSpecIdentifier::address(initial_address);
        let scope_spec_address = identifier
            .get_scope_spec_address()
            .expect("the scope spec address should be directly accessible");
        let scope_spec_uuid = identifier
            .get_scope_spec_uuid()
            .expect("the scope spec uuid should be accessible by conversion");
        assert_eq!(
            initial_address, scope_spec_address,
            "the scope spec address output should be identical to the input",
        );
        assert_eq!(
            expected_scope_spec_uuid, scope_spec_uuid,
            "the scope spec uuid should be as expected",
        );
    }

    #[test]
    fn test_from_serialized_enum_to_uuid() {
        let uuid = Uuid::new_v4().to_string();
        let ser_enum = SerializedEnum::new(UUID_NAME, &uuid);
        let identifier = ScopeSpecIdentifier::from_serialized_enum(&ser_enum)
            .expect("expected serialized enum to identifier to succeed");
        match identifier {
            ScopeSpecIdentifier::Uuid(spec_uuid) => {
                assert_eq!(
                    uuid, spec_uuid,
                    "expected the spec uuid to be properly derived",
                );
            }
            _ => panic!("unexpected identifier derived from type {:?}", identifier),
        };
    }

    #[test]
    fn test_from_serialized_enum_to_address() {
        let ser_enum = SerializedEnum::new(ADDRESS_NAME, "my-address");
        let identifier = ScopeSpecIdentifier::from_serialized_enum(&ser_enum)
            .expect("expected serialized enum to identifier to succeed");
        match identifier {
            ScopeSpecIdentifier::Address(address) => {
                assert_eq!(
                    "my-address", address,
                    "expected the address to be properly derived",
                );
            }
            _ => panic!("unexpected identifier derived from type {:?}", identifier),
        };
    }

    #[test]
    fn test_from_serialized_enum_wrong_type_error() {
        let ser_enum = SerializedEnum::new("bad_type", "some_value");
        let err = ScopeSpecIdentifier::from_serialized_enum(&ser_enum)
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
                    format!("Invalid ScopeSpecIdentifier. Expected one of [{UUID_NAME}, {ADDRESS_NAME}]"),
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
    fn test_to_serialized_enum_uuid() {
        let uuid = Uuid::new_v4().to_string();
        let spec_uuid = ScopeSpecIdentifier::uuid(&uuid);
        let ser_enum = spec_uuid.to_serialized_enum();
        assert_eq!(
            UUID_NAME, ser_enum.r#type,
            "expected the proper enum type to be derived",
        );
        assert_eq!(
            uuid, ser_enum.value,
            "expected the proper value to be derived",
        );
    }

    #[test]
    fn test_to_serialized_enum_address() {
        let address = ScopeSpecIdentifier::address("my-address");
        let ser_enum = address.to_serialized_enum();
        assert_eq!(
            ADDRESS_NAME, ser_enum.r#type,
            "expected the proper enum type to be derived",
        );
        assert_eq!(
            "my-address", ser_enum.value,
            "expected the proper value to be derived",
        );
    }
}
