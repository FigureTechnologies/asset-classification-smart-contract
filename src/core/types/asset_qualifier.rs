use crate::core::error::ContractError;
use crate::core::types::serialized_enum::SerializedEnum;
use crate::util::aliases::AssetResult;
use crate::util::traits::ResultExtensions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const ASSET_TYPE_NAME: &str = "asset_type";
const SCOPE_SPEC_ADDRESS_NAME: &str = "scope_spec_address";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum AssetQualifier {
    AssetType(String),
    ScopeSpecAddress(String),
}
impl AssetQualifier {
    pub fn from_serialized_enum(e: &SerializedEnum) -> AssetResult<Self> {
        match e.enum_type.as_str() {
            ASSET_TYPE_NAME => Self::asset_type(&e.value).to_ok(),
            SCOPE_SPEC_ADDRESS_NAME => Self::scope_spec_address(&e.value).to_ok(),
            _ => ContractError::UnexpectedSerializedEnum {
                received_type: e.enum_type.clone(),
                explanation: format!(
                    "Invalid AssetQualifier. Expected one of [{ASSET_TYPE_NAME}, {SCOPE_SPEC_ADDRESS_NAME}]"
                ),
            }
            .to_err(),
        }
    }

    pub fn to_serialized_enum(&self) -> SerializedEnum {
        match self {
            Self::AssetType(asset_type) => SerializedEnum::new(ASSET_TYPE_NAME, asset_type),
            Self::ScopeSpecAddress(scope_spec_address) => {
                SerializedEnum::new(SCOPE_SPEC_ADDRESS_NAME, scope_spec_address)
            }
        }
    }

    pub fn asset_type<S: Into<String>>(asset_type: S) -> Self {
        Self::AssetType(asset_type.into())
    }

    pub fn scope_spec_address<S: Into<String>>(scope_spec_address: S) -> Self {
        Self::ScopeSpecAddress(scope_spec_address.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::error::ContractError;
    use crate::core::types::asset_qualifier::{
        AssetQualifier, ASSET_TYPE_NAME, SCOPE_SPEC_ADDRESS_NAME,
    };
    use crate::core::types::serialized_enum::SerializedEnum;

    #[test]
    fn test_from_serialized_enum_asset_type() {
        let ser_enum = SerializedEnum::new(ASSET_TYPE_NAME, "heloc");
        let qualifier = AssetQualifier::from_serialized_enum(&ser_enum)
            .expect("expected serialized enum to qualifier to succeed");
        match qualifier {
            AssetQualifier::AssetType(asset_type) => {
                assert_eq!(
                    "heloc", asset_type,
                    "expected the asset type to be properly derived",
                );
            }
            _ => panic!("unexpected qualifier derived from type: {:?}", qualifier),
        };
    }

    #[test]
    fn test_from_serialized_enum_scope_spec_address() {
        let ser_enum = SerializedEnum::new(SCOPE_SPEC_ADDRESS_NAME, "my-address");
        let qualifier = AssetQualifier::from_serialized_enum(&ser_enum)
            .expect("expected serialized enum to address to succeed");
        match qualifier {
            AssetQualifier::ScopeSpecAddress(address) => {
                assert_eq!(
                    "my-address", address,
                    "expected the scope spec address to be properly derived",
                );
            }
            _ => panic!("unexpected qualifier derived from type: {:?}", qualifier),
        };
    }

    #[test]
    fn test_from_serialized_enum_wrong_type_error() {
        let ser_enum = SerializedEnum::new("bad_type", "some_value");
        let err = AssetQualifier::from_serialized_enum(&ser_enum)
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
                    format!(
                        "Invalid AssetQualifier. Expected one of [{ASSET_TYPE_NAME}, {SCOPE_SPEC_ADDRESS_NAME}]"
                    ),
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
    fn test_to_serialized_enum_asset_type() {
        let asset_type = AssetQualifier::asset_type("heloc");
        let ser_enum = asset_type.to_serialized_enum();
        assert_eq!(
            ASSET_TYPE_NAME, ser_enum.enum_type,
            "expected the proper enum type to be derived",
        );
        assert_eq!(
            "heloc", ser_enum.value,
            "expected the proper value to be derived",
        );
    }

    #[test]
    fn test_to_serialized_enum_scope_spec_address() {
        let scope_spec_address = AssetQualifier::scope_spec_address("my-address");
        let ser_enum = scope_spec_address.to_serialized_enum();
        assert_eq!(
            SCOPE_SPEC_ADDRESS_NAME, ser_enum.enum_type,
            "expected the proper enum type to be derived",
        );
        assert_eq!(
            "my-address", ser_enum.value,
            "expected the proper value to be derived",
        );
    }
}
