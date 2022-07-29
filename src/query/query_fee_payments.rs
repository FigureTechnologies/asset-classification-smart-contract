use crate::core::state::may_load_fee_payment_detail;
use crate::core::types::asset_identifier::AssetIdentifier;
use crate::util::aliases::{AssetResult, DepsC};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{to_binary, Binary};

/// A query that fetches a target [FeePaymentDetail](crate::core::types::fee_payment_detail::FeePaymentDetail)
/// from the contract's internal storage and serializes it to a [Binary](cosmwasm_std::Binary)
/// struct.  When none is found, a None Option variant is serialized instead, effectively representing
/// a null json payload.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `qualifier` An enum containing identifier information that can be used to look up a stored
/// [FeePaymentDetail](crate::core::types::fee_payment_detail::FeePaymentDetail) by a derived
/// Provenance Blockchain Metadata Scope bech32 address.
pub fn query_fee_payments(deps: &DepsC, identifier: AssetIdentifier) -> AssetResult<Binary> {
    to_binary(&may_load_fee_payment_detail(
        deps.storage,
        identifier.get_scope_address()?,
    ))?
    .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::core::state::insert_fee_payment_detail;
    use crate::core::types::asset_identifier::AssetIdentifier;
    use crate::core::types::fee_payment_detail::FeePaymentDetail;
    use crate::query::query_fee_payments::query_fee_payments;
    use crate::testutil::test_constants::{DEFAULT_ASSET_UUID, DEFAULT_SCOPE_ADDRESS};
    use crate::testutil::test_utilities::get_duped_fee_payment_detail;
    use cosmwasm_std::from_binary;
    use provwasm_mocks::mock_dependencies;
    use uuid::Uuid;

    #[test]
    fn test_successful_query() {
        let mut deps = mock_dependencies(&[]);
        let payment_detail = get_duped_fee_payment_detail(DEFAULT_SCOPE_ADDRESS);
        insert_fee_payment_detail(deps.as_mut().storage, &payment_detail)
            .expect("expected payment detail to be inserted successfully");
        let result_binary = query_fee_payments(
            &deps.as_ref(),
            AssetIdentifier::asset_uuid(DEFAULT_ASSET_UUID),
        )
        .expect("expected binary result from asset uuid to be successful");
        let result_detail = from_binary::<Option<FeePaymentDetail>>(&result_binary)
            .expect("expected binary deserialization for asset uuid target to be successful")
            .expect("expected the result to be a Some variant");
        assert_eq!(
            payment_detail, result_detail,
            "expected the result to equate to the stored value for asset uuid target",
        );
        let result_binary = query_fee_payments(
            &deps.as_ref(),
            AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
        )
        .expect("expected binary result from scope address to be successful");
        let result_detail = from_binary::<Option<FeePaymentDetail>>(&result_binary)
            .expect("expected binary deserialization for scope address target to be successful")
            .expect("expected the result to be a Some variant");
        assert_eq!(
            payment_detail, result_detail,
            "expected the result to equate to the stored value for asset uuid target",
        );
    }

    #[test]
    fn test_missing_resource_query() {
        let deps = mock_dependencies(&[]);
        let result_binary = query_fee_payments(
            &deps.as_ref(),
            AssetIdentifier::asset_uuid(Uuid::new_v4().to_string()),
        )
        .expect("result should successfully produce a binary even when the value is missing");
        let result_detail = from_binary::<Option<FeePaymentDetail>>(&result_binary).expect(
            "the result should successfully deserialize to an Option for asset uuid variant",
        );
        assert!(
            result_detail.is_none(),
            "the resulting Option should be a None variant for asset uuid variant because the detail did not exist",
        );
        let result_binary = query_fee_payments(
            &deps.as_ref(),
            AssetIdentifier::scope_address("scope1qqse8umjp7pprmd390dnsj7s4yrse73q0x"),
        )
        .expect("result should successfully produce a binary even when the value is missing");
        let result_detail = from_binary::<Option<FeePaymentDetail>>(&result_binary).expect(
            "the result should successfully deserialize to an Option for scope address variant",
        );
        assert!(
            result_detail.is_none(),
            "the resulting Option should be a None variant for scope address variant because the detail did not exist",
        );
    }
}
