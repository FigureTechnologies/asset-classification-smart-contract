use cosmwasm_std::{to_binary, Addr, Binary};
use provwasm_std::ProvenanceQuerier;

use crate::{
    core::{
        asset::AssetScopeAttribute, error::ContractError, msg::AssetIdentifier,
        state::load_asset_definition_by_scope_spec,
    },
    util::{
        aliases::{ContractResult, DepsC},
        scope_address_utils::asset_uuid_to_scope_address,
        traits::ResultExtensions,
    },
};

/// Fetches an AssetScopeAttribute by either the asset uuid or the scope address
pub fn query_asset_scope_attribute(
    deps: &DepsC,
    identifier: AssetIdentifier,
) -> ContractResult<Binary> {
    let scope_attribute = match identifier {
        AssetIdentifier::AssetUuid { asset_uuid } => {
            query_scope_attribute_by_asset_uuid(deps, asset_uuid)
        }
        AssetIdentifier::ScopeAddress { scope_address } => {
            query_scope_attribute_by_scope_address(deps, scope_address)
        }
    }?;
    to_binary(&scope_attribute)?.to_ok()
}

/// Fetches an AssetScopeAttribute by the asset uuid value directly.  Useful for internal contract
/// functionality.
pub fn query_scope_attribute_by_asset_uuid<S: Into<String>>(
    deps: &DepsC,
    asset_uuid: S,
) -> ContractResult<AssetScopeAttribute> {
    query_scope_attribute_by_scope_address(deps, asset_uuid_to_scope_address(asset_uuid)?)
}

/// Fetches an AssetScopeAttribubte by the scope address value directly.  The most efficient version
/// of these functions, but still has to do quite a few lookups.  This functionality should only be used
/// on a once-per-transaction basis, if possible.
pub fn query_scope_attribute_by_scope_address<S: Into<String>>(
    deps: &DepsC,
    scope_address: S,
) -> ContractResult<AssetScopeAttribute> {
    let scope_address_str = scope_address.into();
    let scope_attribute =
        may_query_scope_attribute_by_scope_address(deps, scope_address_str.clone())?;
    // This is a normal scenario, which just means the scope didn't have an attribute.  This can happen if a
    // scope was created with a scope spec that is attached to the contract via AssetDefinition, but the scope was
    // never registered by using onboard_asset.
    if scope_attribute.is_none() {
        return ContractError::NotFound {
            explanation: format!(
                "scope at address [{}] did not include an asset scope attribute",
                scope_address_str
            ),
        }
        .to_err();
    }

    scope_attribute.unwrap().to_ok()
}

/// Fetches an AssetScopeAttribubte by the scope address value directly.  The most efficient version
/// of these functions, but still has to do quite a few lookups.  This functionality should only be used
/// on a once-per-transaction basis, if possible. Returns ContractResult<None> in the case of
pub fn may_query_scope_attribute_by_scope_address<S: Into<String>>(
    deps: &DepsC,
    scope_address: S,
) -> ContractResult<Option<AssetScopeAttribute>> {
    let querier = ProvenanceQuerier::new(&deps.querier);
    // First, query up the scope in order to find the asset definition's type
    let scope = querier.get_scope(scope_address.into())?;
    // Second, query up the asset definition by the scope spec, which is a unique characteristic to the scope spec
    let asset_definition =
        load_asset_definition_by_scope_spec(deps.storage, scope.specification_id)?;
    // Third, construct the attribute name that the scope attribute lives on by mixing the asset definition's asset type with state values
    let attribute_name = asset_definition.attribute_name(deps)?;
    // Fourth, query up scope attributes attached to the scope address under the name attribute.
    // In a proper scenario, there should only ever be one of these
    let scope_attributes = querier.get_json_attributes::<_, _, AssetScopeAttribute>(
        Addr::unchecked(&scope.scope_id),
        &attribute_name,
    )?;
    // This is a very bad scenario - this means that the contract messed up and created multiple attributes under
    // the attribute name.  This should only ever happen in error, and would require a horrible cleanup process
    // that manually removed the bad attributes
    if scope_attributes.len() > 1 {
        return ContractError::std_err(format!(
            "more than one asset scope attribute exists at address [{}]. data repair needed",
            &scope.scope_id
        ))
        .to_err();
    }
    // Retain ownership of the first and verified only scope attribute and return it
    scope_attributes.first().map(|a| a.to_owned()).to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::{from_binary, StdError};
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::{
            asset::AssetScopeAttribute, error::ContractError, msg::AssetIdentifier,
            state::load_asset_definition_by_type,
        },
        testutil::{
            test_constants::{
                DEFAULT_ASSET_TYPE, DEFAULT_ASSET_UUID, DEFAULT_SCOPE_SPEC_ADDRESS,
                DEFAULT_SENDER_ADDRESS, DEFAULT_VALIDATOR_ADDRESS,
            },
            test_utilities::{
                mock_scope, mock_scope_attribute, test_instantiate_success, InstArgs,
            },
        },
        util::scope_address_utils::asset_uuid_to_scope_address,
    };

    use super::query_asset_scope_attribute;

    #[test]
    fn test_successful_query_result() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let asset_uuid = "0caf9164-9f16-11ec-9a49-2b2175f69a81".to_string();
        let scope_address = asset_uuid_to_scope_address(&asset_uuid)
            .expect("expected uuid to scope address conversion to work properly");
        // TDOO: Use the onboard_asset code here when it's ready with mock attribute tracking and such
        let asset_def = load_asset_definition_by_type(deps.as_ref().storage, DEFAULT_ASSET_TYPE)
            .expect(
                "the default asset definition should be available in storage after instantiation",
            );
        // Simulate an asset onboard by building our own attribute
        let asset_attribute = AssetScopeAttribute::new(
            &AssetIdentifier::asset_uuid(DEFAULT_ASSET_UUID),
            DEFAULT_ASSET_TYPE,
            DEFAULT_SENDER_ADDRESS,
            DEFAULT_VALIDATOR_ADDRESS,
            None, // No onboarding status will default to pending
            asset_def
                .validators
                .first()
                .expect("the default asset definition should have a single validator")
                .to_owned(),
        )
        .expect("expected asset attribute to be created properly");
        // Setup mocks
        mock_scope(
            &mut deps,
            &scope_address,
            // Important - scope spec address must be default because we instantiated the default asset definition with this value
            // during the call to test_instantiate_success
            DEFAULT_SCOPE_SPEC_ADDRESS,
            "test-owner",
        );
        mock_scope_attribute(&mut deps, &asset_attribute, &scope_address);
        let binary_from_asset_uuid =
            query_asset_scope_attribute(&deps.as_ref(), AssetIdentifier::asset_uuid(&asset_uuid))
                .expect("expected the scope attribute to be fetched as binary by asset uuid");
        let scope_attribute_from_asset_uuid = from_binary::<AssetScopeAttribute>(
            &binary_from_asset_uuid,
        )
        .expect("expected the asset attribute fetched by asset uuid to deserialize properly");
        assert_eq!(
            asset_attribute, scope_attribute_from_asset_uuid,
            "expected the value fetched by asset uuid to equate to the original value appended to the scope",
        );
        let binary_from_scope_address = query_asset_scope_attribute(
            &deps.as_ref(),
            AssetIdentifier::scope_address(&scope_address),
        )
        .expect("expected the scope attribute to be fetched as binary by scope address");
        let scope_attribute_from_scope_address = from_binary::<AssetScopeAttribute>(
            &binary_from_scope_address,
        )
        .expect("expected the asset attribute fetched by scope address to deserialize properly");
        assert_eq!(
            asset_attribute, scope_attribute_from_scope_address,
            "expected the value fetched by scope address to equate to the original value appeneded to the scope",
        );
    }

    #[test]
    fn test_query_failure_for_missing_scope() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        match query_asset_scope_attribute(
            &deps.as_ref(),
            AssetIdentifier::scope_address("missing-scope-address"),
        )
        .unwrap_err()
        {
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
                _ => panic!("unexpected StdError encountered"),
            },
            _ => panic!("unexpected error type encountered"),
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
        match query_asset_scope_attribute(
            &deps.as_ref(),
            AssetIdentifier::scope_address("fake-scope-address"),
        )
        .unwrap_err()
        {
            ContractError::RecordNotFound { explanation } => {
                assert_eq!(
                    "no asset definition existed for scope spec address some-scope-spec-address",
                    explanation,
                    "incorrect record not found message encountered",
                );
            }
            _ => panic!("unexpected error encountered"),
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
        // Wire up the scope correctly to point to the correct address and scope spec, but don't actually add the attribute
        // needed for the query to succeed
        mock_scope(
            &mut deps,
            &scope_address,
            // Important - scope spec address must be default because we instantiated the default asset definition with this value
            // during the call to test_instantiate_success
            DEFAULT_SCOPE_SPEC_ADDRESS,
            "test-owner",
        );
        match query_asset_scope_attribute(
            &deps.as_ref(),
            AssetIdentifier::scope_address(&scope_address),
        )
        .unwrap_err()
        {
            ContractError::NotFound { explanation } => {
                assert_eq!(
                    "scope at address [scope-address] did not include an asset scope attribute",
                    explanation,
                    "incorrect not found message encountered",
                );
            }
            _ => panic!("unexpected error encountered"),
        };
    }
}
