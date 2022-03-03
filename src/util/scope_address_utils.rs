use crate::{core::error::ContractError, util::aliases::ContractResult};
use bech32::{ToBase32, Variant};
use bytes::{BufMut, BytesMut};
use uuid::Uuid;

use super::traits::ResultExtensions;

/// Standard scope key prefix from the Provenance libs
const KEY_SCOPE: u8 = 0x00;
/// Standard bech32 encoding for scope addresses simply begin with the string "scope"
const SCOPE_HRP: &str = "scope";
const MOST_SIGNIFICANT_BITMASK: u128 = 0xFFFFFFFFFFFFFFFF0000000000000000;
const LEAST_SIGNIFICANT_BITMASK: u128 = 0x0000000000000000FFFFFFFFFFFFFFFF;

/// Takes a string representation of a UUID and converts it to a scope address by appending its
/// most and least significant bits a byte buffer that contains the scope key prefix.
pub fn asset_uuid_to_scope_address<S: Into<String>>(asset_uuid: S) -> ContractResult<String> {
    let (most_sig, least_sig) = get_uuid_bits(asset_uuid)?;
    let mut buf = BytesMut::new();
    // Append the scope prefix key
    buf.put_u8(KEY_SCOPE);
    // Order matters!  Append most significant bits first, least second
    buf.put_i64(most_sig);
    buf.put_i64(least_sig);
    Ok(bech32::encode(
        SCOPE_HRP,
        buf.to_vec().to_base32(),
        Variant::Bech32,
    )?)
}

/// Standard uuid most/least significant bits source, logically abstracted from the java tools for
/// locating these values.
fn get_uuid_bits<S: Into<String>>(uuid_source: S) -> ContractResult<(i64, i64)> {
    let uuid_value = Uuid::parse_str(&uuid_source.into())?.as_u128();
    let most_significant_bits: i64 = ((uuid_value & MOST_SIGNIFICANT_BITMASK) >> 64) as i64;
    let least_significant_bits: i64 = (uuid_value & LEAST_SIGNIFICANT_BITMASK) as i64;
    Ok((most_significant_bits, least_significant_bits))
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
        core::error::ContractError,
        util::scope_address_utils::{asset_uuid_to_scope_address, get_uuid_bits},
    };

    use super::get_validate_scope_address;

    #[test]
    fn test_conversion_result() {
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
    fn test_bit_extraction() {
        // Source uuid randomly generated via CLI tool
        let source_uuid = "fb932526-9a56-11ec-b5de-b3e1f55b0723";
        // Expected results taken from the battle-tested java util.UUID output for verification that
        // this functionality set produces the same result
        let expected_most_sig: i64 = -318870300884856340;
        let expected_least_sig: i64 = -5341634324949432541;
        let (most_sig, least_sig) = get_uuid_bits(source_uuid)
            .expect("expected the source uuid to properly derive most and least significant bits");
        assert_eq!(
            expected_most_sig, most_sig,
            "the most significant bits value should match expected output"
        );
        assert_eq!(
            expected_least_sig, least_sig,
            "the least significant bits value should match expected output"
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
}
