use std::{convert::TryInto, str::FromStr};

use crate::{core::error::ContractError, util::aliases::ContractResult};
use bech32::{FromBase32, ToBase32, Variant};
use uuid::Uuid;

use super::traits::ResultExtensions;

/// Standard scope key prefix from the Provenance libs
const KEY_SCOPE: u8 = 0x00;
/// Standard bech32 encoding for scope addresses simply begin with the string "scope"
const SCOPE_HRP: &str = "scope";
/// Takes a string representation of a UUID and converts it to a scope address by appending its
/// most and least significant bits a byte buffer that contains the scope key prefix.
pub fn asset_uuid_to_scope_address<S: Into<String>>(asset_uuid: S) -> ContractResult<String> {
    let mut buffer: Vec<u8> = vec![KEY_SCOPE];
    buffer.append(
        &mut Uuid::from_str(&asset_uuid.into())?
            .as_u128()
            .to_be_bytes()
            .to_vec(),
    );
    bech32::encode(SCOPE_HRP, buffer.to_vec().to_base32(), Variant::Bech32)?.to_ok()
}

/// Takes a string representation of a scope address and converts it into an asset uuid string.
pub fn scope_address_to_asset_uuid<S: Into<String>>(scope_address: S) -> ContractResult<String> {
    let target_address = scope_address.into();
    let (_, base_32, _) = bech32::decode(&target_address)?;
    let uuid_bytes: [u8; 16] = Vec::from_base32(&base_32)?
        .into_iter()
        // Lop off the first byte - it represents the scope key prefix and is not a portion of the uuid bytes
        .skip(1)
        .collect::<Vec<u8>>()
        .try_into()
        .map_err(|_| {
            ContractError::std_err(format!(
                "Failed deserializing base32 data for scope address {}",
                &target_address,
            ))
        })?;
    // Important: this uses from_slice instead of from_bytes.  from_bytes is fully unchecked and trusts the
    // caller that they are using valid data that can convert to a uuid.  To avoid any weird panics when calling
    // to_string(), we just use the sliced data
    Uuid::from_slice(&uuid_bytes)?.to_string().to_ok()
}

pub fn get_validate_scope_address<S1: Into<String> + Clone, S2: Into<String> + Clone>(
    asset_uuid: Option<S1>,
    scope_address: Option<S2>,
) -> ContractResult<String> {
    if let (Some(uuid), Some(address)) = (asset_uuid.clone(), scope_address.clone()) {
        let parsed_address = asset_uuid_to_scope_address(uuid.clone())?;
        if parsed_address != address.clone().into() {
            return ContractError::AssetIdentifierMismatch {
                asset_uuid: uuid.into(),
                scope_address: address.into(),
            }
            .to_err();
        }
    }

    if let Some(addr) = scope_address {
        Ok(addr.into())
    } else if let Some(uuid) = asset_uuid {
        asset_uuid_to_scope_address(uuid)
    } else {
        ContractError::AssetIdentifierNotSupplied.to_err()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        core::error::ContractError, util::scope_address_utils::asset_uuid_to_scope_address,
    };

    use super::{get_validate_scope_address, scope_address_to_asset_uuid};

    #[test]
    fn test_successful_asset_uuid_to_scope_address() {
        // Source uuid randomly generated via CLI tool
        let source_uuid = "a5e5a828-9a48-11ec-8193-1731fd63d6a6";
        // Expected result taken from MetadataAddress Provenance tool for verification that this
        // functionality set produces the same result
        let expected_bech32 = "scope1qzj7t2pgnfyprmypjvtnrltr66nqd4c3cq";
        let result = asset_uuid_to_scope_address(source_uuid)
            .expect("conversion should execute without failure");
        assert_eq!(
            expected_bech32,
            result.as_str(),
            "the resulting scope address should match"
        );
    }

    #[test]
    fn test_invalid_asset_uuid_to_scope_address() {
        // Close to a UUID but has invalid characters
        let similar_but_bad =
            asset_uuid_to_scope_address("zzzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz").unwrap_err();
        assert!(
            matches!(similar_but_bad, ContractError::UuidError(_)),
            "a uuid error should occur when an invalid uuid is processed: similar to good uuid but invalid characters",
        );
        let not_even_close = asset_uuid_to_scope_address("definitely not a uuid").unwrap_err();
        assert!(
            matches!(not_even_close, ContractError::UuidError(_)),
            "a uuid error should occur when an invalid uuid is processed: very malformatted uuid",
        );
    }

    #[test]
    fn test_get_validate_scope_address_mismatch_error() {
        let asset_uuid_mismatched = "cde62981-526e-4fde-9cb4-312f040dc283";
        let scope_address_mismatched = "scope1qzj7t2pgnfyprmypjvtnrltr66nqd4c3cq"; // doesn't correspond to asset_uuid

        let err =
            get_validate_scope_address(Some(asset_uuid_mismatched), Some(scope_address_mismatched))
                .unwrap_err();

        match err {
            ContractError::AssetIdentifierMismatch {
                asset_uuid,
                scope_address,
            } => {
                assert_eq!(
                    asset_uuid_mismatched, asset_uuid,
                    "Asset identifier mismatch error should contain the provided asset_uuid"
                );
                assert_eq!(
                    scope_address_mismatched, scope_address,
                    "Asset identifier mismatch error should contain the provided scope_address"
                );
            }
            _ => panic!("Unexpected error for asset identifier mismatch"),
        }
    }

    #[test]
    fn test_get_validate_scope_address_none_provided_error() {
        let err = get_validate_scope_address::<&str, &str>(None, None).unwrap_err();

        match err {
            ContractError::AssetIdentifierNotSupplied => {}
            _ => panic!("Unexpected error for asset identifier mismatch"),
        }
    }

    #[test]
    fn test_get_validate_scope_address_asset_uuid_returns_address() {
        // Source uuid randomly generated via CLI tool
        let source_uuid = "a5e5a828-9a48-11ec-8193-1731fd63d6a6";
        // Expected result taken from MetadataAddress Provenance tool for verification that this
        // functionality set produces the same result
        let expected_bech32 = "scope1qzj7t2pgnfyprmypjvtnrltr66nqd4c3cq";

        let result = get_validate_scope_address::<&str, &str>(Some(source_uuid), None).unwrap();

        assert_eq!(
            expected_bech32, result,
            "The resulting scope address should match when only a uuid was provided"
        )
    }

    #[test]
    fn test_get_validate_scope_address_scope_address_returns_provided_address() {
        let expected_bech32 = "scope1qzj7t2pgnfyprmypjvtnrltr66nqd4c3cq";

        let result = get_validate_scope_address::<&str, &str>(None, Some(expected_bech32)).unwrap();

        assert_eq!(
            expected_bech32, result,
            "The resulting scope address should match when only a uuid was provided"
        )
    }

    #[test]
    fn test_successful_scope_address_to_asset_uuid() {
        // These values were generated using the MetadataAddress kotlin helper to verify their authenticity
        // from random input
        let scope_address = "scope1qzwk9mygnlv3rm96d0mn6lynsdyqwn6nra";
        let expected_uuid = "9d62ec88-9fd9-11ec-ba6b-f73d7c938348";
        let result_uuid = scope_address_to_asset_uuid(scope_address)
            .expect("expected the conversion to occur without error");
        assert_eq!(
            expected_uuid, result_uuid,
            "the function produced the incorrect uuid value"
        );
    }

    #[test]
    fn test_invalid_scope_address_to_asset_uuid() {
        let error = scope_address_to_asset_uuid("not a scope address").unwrap_err();
        assert!(
            matches!(error, ContractError::Bech32Error(_)),
            "a bech32 error should occur when attempting to parse an invalid scope address",
        );
    }
}
