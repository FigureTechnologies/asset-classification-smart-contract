use crate::core::types::asset_identifier::AssetIdentifier;
use crate::util::aliases::AssetResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// There is a bug in cosmwasm 1.0.0's interaction with serde-json-wasm that causes floating point
/// operations to be added into the compiled wasm, so the previous solution of using things like
/// AssetIdentifier directly and specifying them with a tag and content param in their serde
/// annotation is impossible as of 1.0.0.  This solution will allow existing requests to remain
/// identical, but not generate floating point errors.  It makes the schema less useful, but it's a
/// hack to fix a bug, so...
///
/// It's also worth noting that this solution can only create enum switches that have Strings as
/// their values.  Anything different will not work for this solution and will require further
/// adaptation and hackery.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SerializedEnum {
    /// Specifies the type of enum to deserialize into. Maps into one of the values specified in
    /// the impl for this struct.
    pub r#type: String,
    /// Specifies the string value to be used for the type.
    pub value: String,
}
impl SerializedEnum {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `enum_type` Specifies the type of enum to deserialize into. Maps into one of the values specified in
    /// the impl for this struct.
    /// * `value` Specifies the string value to be used for the type.
    pub fn new<S1: Into<String>, S2: Into<String>>(enum_type: S1, value: S2) -> Self {
        Self {
            r#type: enum_type.into(),
            value: value.into(),
        }
    }

    /// Converts this value to an instance of an asset identifier if it has a compatible type.
    pub fn to_asset_identifier(&self) -> AssetResult<AssetIdentifier> {
        AssetIdentifier::from_serialized_enum(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::error::ContractError;
    use crate::core::types::asset_identifier::AssetIdentifier;
    use crate::core::types::serialized_enum::SerializedEnum;
    use uuid::Uuid;

    #[test]
    fn test_to_asset_identifier_uuid_success() {
        let uuid = Uuid::new_v4().to_string();
        let identifier = SerializedEnum::new("asset_uuid", &uuid)
            .to_asset_identifier()
            .expect("expected the conversion to succeed to asset identifier");
        match identifier {
            AssetIdentifier::AssetUuid(asset_uuid) => {
                assert_eq!(uuid, asset_uuid, "expected the proper uuid to be derived",);
            }
            _ => panic!("unexpected identifier derived: {:?}", identifier),
        };
    }

    #[test]
    fn test_to_asset_identifier_address_success() {
        let identifier = SerializedEnum::new("scope_address", "my-address")
            .to_asset_identifier()
            .expect("expected the conversion to succeed to asset identifier");
        match identifier {
            AssetIdentifier::ScopeAddress(scope_address) => {
                assert_eq!(
                    "my-address", scope_address,
                    "expected the proper address to be derived",
                );
            }
            _ => panic!("unexpected identifier derived: {:?}", identifier),
        };
    }

    #[test]
    fn test_to_asset_identifier_failure() {
        let err = SerializedEnum::new("incorrect_variant", "some-value")
            .to_asset_identifier()
            .expect_err("expected an incorrect variant to produce an error");
        match err {
            ContractError::UnexpectedSerializedEnum {
                received_type,
                explanation,
            } => {
                assert_eq!(
                    "incorrect_variant", received_type,
                    "expected the unexpected type to be provided in the error message",
                );
                assert_eq!(
                    format!("Invalid AssetIdentifier. Expected one of [asset_uuid, scope_address]"),
                    explanation,
                    "expected the explanation to list the type of the enum and the expected values",
                );
            }
            _ => panic!("unexpected error encountered: {:?}", err),
        };
    }
}
