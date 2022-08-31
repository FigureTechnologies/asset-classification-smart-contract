use crate::core::error::ContractError;
use crate::core::types::serialized_enum::SerializedEnum;
use crate::util::aliases::AssetResult;
use crate::util::traits::ResultExtensions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// TODO: REMOVE THIS QUALIFIER CLASS/FILE ENTIRELY?

const ASSET_TYPE_NAME: &str = "asset_type";

/// An enum containing different identifiers that can be used to fetch an [AssetDefinitionV2](super::asset_definition::AssetDefinitionV2).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetQualifier {
    /// The unique name for an asset type.  Ex: heloc, payable, etc.
    AssetType(String),
}
impl AssetQualifier {
    /// Converts a [SerializedEnum](super::serialized_enum::SerializedEnum) instance to one of the
    /// variants of this enum, if possible.  On a failure, a [ContractError::UnexpectedSerializedEnum](crate::core::error::ContractError::UnexpectedSerializedEnum)
    /// error will be produced, indicating the unexpected value.
    ///
    /// # Parameters
    ///
    /// * `e` The serialized enum instance for which to attempt conversion.
    pub fn from_serialized_enum(e: &SerializedEnum) -> AssetResult<Self> {
        match e.r#type.as_str() {
            ASSET_TYPE_NAME => Self::asset_type(&e.value).to_ok(),
            _ => ContractError::UnexpectedSerializedEnum {
                received_type: e.r#type.clone(),
                explanation: format!("Invalid AssetQualifier. Expected [{ASSET_TYPE_NAME}]"),
            }
            .to_err(),
        }
    }

    /// Converts the specific variant of this enum to a [SerializedEnum](super::serialized_enum::SerializedEnum).
    pub fn to_serialized_enum(&self) -> SerializedEnum {
        match self {
            Self::AssetType(asset_type) => SerializedEnum::new(ASSET_TYPE_NAME, asset_type),
        }
    }

    /// Creates a new instance of this enum as the [AssetType](self::AssetQualifier::AssetType) variant.
    ///
    /// # Parameters
    ///
    /// * `asset_type` The unique name for an asset type.
    pub fn asset_type<S: Into<String>>(asset_type: S) -> Self {
        Self::AssetType(asset_type.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::error::ContractError;
    use crate::core::types::asset_qualifier::{AssetQualifier, ASSET_TYPE_NAME};
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
                    format!("Invalid AssetQualifier. Expected [{ASSET_TYPE_NAME}]"),
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
            ASSET_TYPE_NAME, ser_enum.r#type,
            "expected the proper enum type to be derived",
        );
        assert_eq!(
            "heloc", ser_enum.value,
            "expected the proper value to be derived",
        );
    }
}
