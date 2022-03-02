use crate::util::aliases::ContractResult;
use bech32::{ToBase32, Variant};
use bytes::{BufMut, BytesMut};
use uuid::Uuid;

/// Standard scope key prefix from the Provenance libs
const KEY_SCOPE: u8 = 0x00;
/// Standard bech32 encoding for scope addresses simply begin with the string "scope"
const SCOPE_HRP: &str = "scope";
const MOST_SIGNIFICANT_BITMASK: u128 = 0xFFFFFFFFFFFFFFFF0000000000000000u128;
const LEAST_SIGNIFICANT_BITMASK: u128 = 0x0000000000000000FFFFFFFFFFFFFFFFu128;

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

#[cfg(test)]
mod tests {
    use crate::util::scope_address_utils::{asset_uuid_to_scope_address, get_uuid_bits};

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
}
