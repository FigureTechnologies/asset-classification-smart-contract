use cosmwasm_std::{to_binary, Addr, Binary};
use provwasm_std::ProvenanceQuerier;
use result_extensions::ResultExtensions;

use crate::{
    core::{
        error::ContractError,
        state::load_asset_definition_by_type_v3,
        types::{asset_identifier::AssetIdentifier, asset_scope_attribute::AssetScopeAttribute},
    },
    util::{
        aliases::{AssetResult, DepsC},
        scope_address_utils::asset_uuid_to_scope_address,
    },
};

/// Fetches an AssetScopeAttribute by either the asset uuid or the scope address for a particular asset type.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `identifier` Helps derive a unique key that can locate an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
/// * `asset_type` The asset type to query for
pub fn query_asset_scope_attribute_by_asset_type<S: Into<String>>(
    deps: &DepsC,
    identifier: AssetIdentifier,
    asset_type: S,
) -> AssetResult<Binary> {
    let scope_attribute = match identifier {
        AssetIdentifier::AssetUuid(asset_uuid) => {
            may_query_scope_attribute_by_asset_uuid_and_asset_type(deps, asset_uuid, asset_type)
        }
        AssetIdentifier::ScopeAddress(scope_address) => {
            may_query_scope_attribute_by_scope_address_and_asset_type(
                deps,
                scope_address,
                asset_type,
            )
        }
    }?;
    to_binary(&scope_attribute)?.to_ok()
}

/// Fetches an AssetScopeAttribute by the asset uuid value directly for a particular asset type.  Useful for internal contract
/// functionality.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `asset_uuid` Directly links to the [asset_uuid](crate::core::types::asset_scope_attribute::AssetScopeAttribute::asset_uuid)
/// value on an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
/// * `asset_type` The asset type to query for
pub fn query_scope_attribute_by_asset_uuid_and_asset_type<S1: Into<String>, S2: Into<String>>(
    deps: &DepsC,
    asset_uuid: S1,
    asset_type: S2,
) -> AssetResult<AssetScopeAttribute> {
    query_scope_attribute_by_scope_address_and_asset_type(
        deps,
        asset_uuid_to_scope_address(asset_uuid)?,
        asset_type,
    )
}

/// Fetches an AssetScopeAttribute by the scope address value directly for a particular asset type.  The most efficient version
/// of these functions, but still has to do quite a few lookups.  This functionality should only be used
/// on a once-per-transaction basis, if possible.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `scope_address` Directly links to the [scope_address](crate::core::types::asset_scope_attribute::AssetScopeAttribute::scope_address)
/// value on an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
/// * `asset_type` The asset type to query for
pub fn query_scope_attribute_by_scope_address_and_asset_type<S1: Into<String>, S2: Into<String>>(
    deps: &DepsC,
    scope_address: S1,
    asset_type: S2,
) -> AssetResult<AssetScopeAttribute> {
    let scope_address = scope_address.into();
    let asset_type = asset_type.into();
    let scope_attribute = may_query_scope_attribute_by_scope_address_and_asset_type(
        deps,
        scope_address.clone(),
        &asset_type,
    )?;
    // This is a normal scenario, which just means the scope didn't have an attribute.  This can happen if a scope was
    // never registered by using onboard_asset.
    if let Some(attr) = scope_attribute {
        attr.to_ok()
    } else {
        ContractError::NotFound {
            explanation: format!(
                "scope at address [{}] did not include an asset scope attribute for asset type [{}]",
                scope_address,
                asset_type
            ),
        }
        .to_err()
    }
}

/// Fetches an AssetScopeAttribute by the scope address value, derived from the asset uuid, and asset type.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `asset_uuid` Directly links to the [asset_uuid](crate::core::types::asset_scope_attribute::AssetScopeAttribute::asset_uuid)
/// value on an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
/// * `asset_type` The asset type to query for
pub fn may_query_scope_attribute_by_asset_uuid_and_asset_type<
    S1: Into<String>,
    S2: Into<String>,
>(
    deps: &DepsC,
    asset_uuid: S1,
    asset_type: S2,
) -> AssetResult<Option<AssetScopeAttribute>> {
    may_query_scope_attribute_by_scope_address_and_asset_type(
        deps,
        asset_uuid_to_scope_address(asset_uuid)?,
        asset_type,
    )
}

/// Fetches an AssetScopeAttribute by the scope address value directly for a particular asset type.  The most efficient version
/// of these functions, but still has to do a couple of lookups.  This functionality should only be used
/// on a once-per-transaction basis, if possible. Returns a ContractResult<None> in the case of no attribute
/// of the specified asset type being associated with the scope.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `scope_address` Directly links to the [scope_address](crate::core::types::asset_scope_attribute::AssetScopeAttribute::scope_address)
/// value on an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
/// * `asset_type` The asset type to query for
pub fn may_query_scope_attribute_by_scope_address_and_asset_type<
    S1: Into<String>,
    S2: Into<String>,
>(
    deps: &DepsC,
    scope_address: S1,
    asset_type: S2,
) -> AssetResult<Option<AssetScopeAttribute>> {
    let querier = ProvenanceQuerier::new(&deps.querier);
    let scope_address_str: String = scope_address.into();

    // First, query up the scope to verify it exists
    let _scope = querier.get_scope(&scope_address_str)?;

    // Second, query up the asset definition by the asset type
    let asset_definition = load_asset_definition_by_type_v3(deps.storage, &asset_type.into())?;

    // Third, construct the attribute name that the scope attribute lives on by mixing the asset definition's asset type with state values
    let attribute_name = asset_definition.attribute_name(deps)?;

    // Fourth, query up scope attributes attached to the scope address under the name attribute.
    // In a proper scenario, there should only ever be one of these
    let scope_attributes = querier.get_json_attributes::<_, _, AssetScopeAttribute>(
        Addr::unchecked(&scope_address_str),
        &attribute_name,
    )?;
    // This is a very bad scenario - this means that the contract messed up and created multiple attributes under
    // the attribute name.  This should only ever happen in error, and would require a horrible cleanup process
    // that manually removed the bad attributes
    if scope_attributes.len() > 1 {
        return ContractError::generic(format!(
            "more than one asset scope attribute exists at address [{}]. data repair needed",
            scope_address_str
        ))
        .to_err();
    }
    // Retain ownership of the first and verified only scope attribute
    scope_attributes.first().map(|a| a.to_owned()).to_ok()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_binary, StdError};
    use provwasm_mocks::mock_dependencies;

    use crate::testutil::onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset};
    use crate::testutil::test_constants::{DEFAULT_ASSET_TYPE, DEFAULT_SCOPE_ADDRESS};
    use crate::testutil::test_utilities::setup_test_suite;
    use crate::{
        core::{
            error::ContractError,
            types::{
                asset_identifier::AssetIdentifier, asset_scope_attribute::AssetScopeAttribute,
            },
        },
        testutil::{
            test_constants::{DEFAULT_ASSET_UUID, DEFAULT_SCOPE_SPEC_ADDRESS},
            test_utilities::{mock_scope, test_instantiate_success, InstArgs},
        },
    };

    use super::query_asset_scope_attribute_by_asset_type;

    #[test]
    fn test_successful_query_result() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("expected the asset onboard to succeed");
        let binary_from_asset_uuid = query_asset_scope_attribute_by_asset_type(
            &deps.as_ref(),
            AssetIdentifier::asset_uuid(DEFAULT_ASSET_UUID),
            DEFAULT_ASSET_TYPE,
        )
        .expect("expected the scope attribute to be fetched as binary by asset uuid");
        let scope_attribute_from_asset_uuid =
            from_binary::<Option<AssetScopeAttribute>>(&binary_from_asset_uuid)
                .expect(
                    "expected the asset attribute fetched by asset uuid to deserialize properly",
                )
                .expect("expected the asset attribute to be present in the resulting Option");
        let binary_from_scope_address = query_asset_scope_attribute_by_asset_type(
            &deps.as_ref(),
            AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
            DEFAULT_ASSET_TYPE,
        )
        .expect("expected the scope attribute to be fetched as binary by scope address");
        let scope_attribute_from_scope_address = from_binary::<Option<AssetScopeAttribute>>(
            &binary_from_scope_address,
        )
        .expect("expected the asset attribute fetched by scope address to deserialize properly")
        .expect("expected the asset attribute fetched by scope address to be present in the resulting Option");
        assert_eq!(
            scope_attribute_from_asset_uuid, scope_attribute_from_scope_address,
            "expected the value fetched by scope address to equate to the original value appeneded to the scope",
        );
    }

    #[test]
    fn test_query_failure_for_missing_scope() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = query_asset_scope_attribute_by_asset_type(
            &deps.as_ref(),
            AssetIdentifier::scope_address("missing-scope-address"),
            DEFAULT_ASSET_TYPE,
        )
        .unwrap_err();
        match error {
            ContractError::Std(e) => match e {
                StdError::GenericErr { msg, .. } => {
                    assert!(
                        msg.contains("metadata not found"),
                        "the error should be from the metadata module",
                    );
                    assert!(
                        msg.contains("get_scope"),
                        "the error should denote that the scope fetch was the failure",
                    );
                }
                _ => panic!("unexpected StdError encountered: {:?}", e),
            },
            _ => panic!("unexpected error type encountered: {:?}", error),
        };
    }

    #[test]
    fn test_query_failure_for_missing_asset_definition() {
        let mut deps = mock_dependencies(&[]);
        mock_scope(
            &mut deps,
            "fake-scope-address",
            "some-scope-spec-address",
            "test-owner",
        );
        let error = query_asset_scope_attribute_by_asset_type(
            &deps.as_ref(),
            AssetIdentifier::scope_address("fake-scope-address"),
            "bogus-asset-type",
        )
        .unwrap_err();
        match error {
            ContractError::RecordNotFound { explanation } => {
                assert_eq!(
                    "no asset definition existed for asset type bogus-asset-type", explanation,
                    "incorrect record not found message encountered",
                );
            }
            _ => panic!("unexpected error encountered: {:?}", error),
        };
    }

    // Note - the mock querier does not allow the capability for mocking multiple attributes under a single name
    // so we cannot test the error where there are multiple attributes accidentally registered.  This is an
    // unfortunate lack of test coverage, but that error should only ever occur if this contract is mis-coded to
    // not remove existing attributes before adding new ones
    #[test]
    fn test_query_failure_for_missing_scope_attribute() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let scope_address = "scope-address".to_string();
        // Wire up the scope correctly to point to the correct address, but don't actually add the attribute
        // needed for the query to succeed
        mock_scope(
            &mut deps,
            &scope_address,
            DEFAULT_SCOPE_SPEC_ADDRESS,
            "test-owner",
        );
        let binary = query_asset_scope_attribute_by_asset_type(
            &deps.as_ref(),
            AssetIdentifier::scope_address(&scope_address),
            DEFAULT_ASSET_TYPE,
        )
        .expect("the query should execute without error");
        let result = from_binary::<Option<AssetScopeAttribute>>(&binary)
            .expect("expected the result to deserialize correctly");
        assert!(
            result.is_none(),
            "expected the result from the query to be missing because no scope attribute existed at the scope address",
        );
    }
}
