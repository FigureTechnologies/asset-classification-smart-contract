use std::{convert::TryInto, str::FromStr};

use crate::{core::error::ContractError, util::aliases::AssetResult};
use bech32::{FromBase32, ToBase32, Variant};
use cosmwasm_std::Addr;
use result_extensions::ResultExtensions;
use uuid::Uuid;

// Standard scope key prefix from the Provenance libs
const KEY_SCOPE: u8 = 0x00;
// Standard bech32 encoding for mainnet addresses simply begins the with the string "pb"
const MAINNET_HRP: &str = "pb";
// Standard bech32 encoding for testnet addresses simply begins with the string "tp"
const TESTNET_HRP: &str = "tp";
// Standard bech32 encoding for scope addresses simply begins with the string "scope"
const SCOPE_HRP: &str = "scope";
// All valid hrps for use in the underlying functions
const VALID_HRPS: [&str; 3] = [MAINNET_HRP, TESTNET_HRP, SCOPE_HRP];

/// Converts a string containing an asset uuid into a scope address.
///
/// # Parameters
///
/// * `asset_uuid` A valid uuid v4 string.
pub fn asset_uuid_to_scope_address<S: Into<String>>(asset_uuid: S) -> AssetResult<String> {
    uuid_to_address(KEY_SCOPE, SCOPE_HRP, asset_uuid)
}

/// Takes a string representation of a scope address and converts it into an asset uuid string.
/// Note: This conversion can also be called scope_address_to_scope_uuid because asset uuid always
/// matches the scope uuid, as a convention.
///
/// # Parameters
///
/// * `scope_address` A valid bech32 address with an hrp of "scope".
pub fn scope_address_to_asset_uuid<S: Into<String>>(scope_address: S) -> AssetResult<String> {
    address_to_uuid(scope_address, SCOPE_HRP)
}

/// Validates that the address is valid by decoding to base 32, and then converts it to an Addr.
///
/// # Parameters
///
/// * `address` A valid bech32 address with any provenance blockchain hrp specified in this contract.
pub fn bech32_string_to_addr<S: Into<String>>(address: S) -> AssetResult<Addr> {
    let address_string = address.into();
    // First, try to decode the string as Bech32.  If this fails, then the input is invalid and should not be converted to an Addr
    let (hrp, _, _) = bech32::decode(&address_string)?;
    if !VALID_HRPS.contains(&hrp.as_str()) {
        ContractError::InvalidAddress {
            address: address_string,
            explanation: format!("invalid address prefix [{}]", hrp),
        }
        .to_err()
    } else {
        // Once the address has been validated as bech32, just funnel it into the Addr struct with an unchecked call
        Addr::unchecked(&address_string).to_ok()
    }
}

/// Takes a string representation of a UUID and converts it to a scope address by appending its
/// big-endian bytes to a byte slice that also contains a prefix key (as defined in the provenance source).
///
/// # Parameters
///
/// * `key_byte` The first byte of the result encoding, indicating the type of bech32 address to
/// generate.  This value, encoded into the bech32 value, should be a standard value accepted by
/// the Provenance Blockchain.
/// * `hrp` The human readable prefix of the bech32 address to generate.
/// * `uuid` A valid uuid v4 string.
fn uuid_to_address<S: Into<String>>(key_byte: u8, hrp: &str, uuid: S) -> AssetResult<String> {
    let mut buffer = vec![key_byte];
    buffer.append(&mut Uuid::from_str(&uuid.into())?.as_bytes().to_vec());
    bech32::encode(hrp, buffer.to_base32(), Variant::Bech32)?.to_ok()
}

/// Takes a valid bech32 address with the acknowledged prefix and attempts to convert it to a uuid.
/// This should only be used for addresses that are derived from uuid sources, like a scope.
///
/// # Parameters
///
/// * `address` A valid Provenance Blockchain bech32 address.
/// * `expected_hrp` The hrp expected to be at the front of the provided address.  This ensures that
/// the correct operation is being performed and the code does not have any bugs.
fn address_to_uuid<S1: Into<String>, S2: Into<String>>(
    address: S1,
    expected_hrp: S2,
) -> AssetResult<String> {
    let target_address = address.into();
    let (hrp, base_32, _) = bech32::decode(&target_address)?;
    let expected_hrp_string = expected_hrp.into();
    // Run a human-readable-prefix match on the output of the decode to verify that the address passed into the
    // function is of the correct type, avoiding unnnecessary and confusing panics
    if hrp != expected_hrp_string {
        return ContractError::InvalidAddress {
            address: target_address,
             explanation: format!("expected the prefix [{}] to be included in the specified address, but the prefix was [{}]", expected_hrp_string, hrp)
        }
        .to_err();
    }
    let uuid_bytes: [u8; 16] = Vec::from_base32(&base_32)?
        .into_iter()
        // Lop off the first byte - it represents the key prefix byte and is not a portion of the uuid bytes
        .skip(1)
        .collect::<Vec<u8>>()
        .try_into()
        .map_err(|_| {
            ContractError::generic(format!(
                "Failed deserializing base32 data for address {}",
                &target_address,
            ))
        })?;
    // Important: this uses from_slice instead of from_bytes.  from_bytes is fully unchecked and trusts the
    // caller that they are using valid data that can convert to a uuid.  To avoid any weird panics when calling
    // to_string(), we just use the sliced data
    Uuid::from_slice(&uuid_bytes)?.to_string().to_ok()
}

#[cfg(test)]
mod tests {
    use crate::{
        core::error::ContractError, util::scope_address_utils::asset_uuid_to_scope_address,
    };

    use super::{bech32_string_to_addr, scope_address_to_asset_uuid};

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
            "a uuid error should occur when an invalid uuid is processed: similar to good uuid but invalid characters. Got error: {:?}",
            similar_but_bad,
        );
        let not_even_close = asset_uuid_to_scope_address("definitely not a uuid").unwrap_err();
        assert!(
            matches!(not_even_close, ContractError::UuidError(_)),
            "a uuid error should occur when an invalid uuid is processed: very malformatted uuid. Got error: {:?}",
            not_even_close,
        );
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
            "the function produced an incorrect uuid value",
        );
    }

    #[test]
    fn test_invalid_scope_address_to_asset_uuid_for_invalid_address() {
        let error = scope_address_to_asset_uuid("not a scope address").unwrap_err();
        assert!(
            matches!(error, ContractError::Bech32Error(_)),
            "a bech32 error should occur when attempting to parse an invalid scope address, but got error: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_scope_address_to_asset_uuid_for_wrong_address_type() {
        let error = scope_address_to_asset_uuid("scopespec1qj3s7dvsnlh3rmyy3pm5tszs2v7qegwr7j")
            .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidAddress { .. }),
            "an invalid address error should be returned when the wrong address type is provided, but got error: {:?}",
            error,
        );
    }

    #[test]
    fn test_valid_bech32_to_addr() {
        let bech_32_string = "tp15e6l9dv8s2rdshjfn34k8a2nju55tr4z42phrt";
        let addr = bech32_string_to_addr(bech_32_string)
            .expect("the resulting value should be converted to an Addr");
        assert_eq!(
            bech_32_string,
            addr.as_str(),
            "the resulting Addr value should reflect the input"
        );
    }

    #[test]
    fn test_invalid_bech32_to_addr_non_address_input() {
        let error = bech32_string_to_addr("not an address").unwrap_err();
        assert!(
            matches!(error, ContractError::Bech32Error(_)),
            "the underlying bech32 library should provide an error for an invalid address, but got error: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_bech32_to_addr_unsupported_hrp() {
        let bc_address = "bc1q35a3dc2e5lj237ns39q5pd7t8wxm2ah7rdvx5d";
        let error = bech32_string_to_addr(bc_address).unwrap_err();
        match error {
            ContractError::InvalidAddress {
                address,
                explanation,
            } => {
                assert_eq!(
                    bc_address,
                    address.as_str(),
                    "expected the address to be appended to the error",
                );
                assert_eq!(
                    "invalid address prefix [bc]", explanation,
                    "expected the explanation to include the invalid hrp",
                );
            }
            _ => panic!("unexpected error encountered: {:?}", error),
        }
    }
}
