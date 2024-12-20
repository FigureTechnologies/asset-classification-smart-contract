use std::collections::{HashMap, HashSet};

use cosmwasm_std::{from_json, to_json_binary, Binary, Deps};
use provwasm_std::types::provenance::{
    attribute::v1::{AttributeQuerier, AttributeType},
    metadata::v1::MetadataQuerier,
};
use result_extensions::ResultExtensions;

use crate::{
    core::{
        error::ContractError,
        state::{list_asset_definitions_v3, STATE_V2},
        types::{asset_identifier::AssetIdentifier, asset_scope_attribute::AssetScopeAttribute},
    },
    util::{aliases::AssetResult, scope_address_utils::asset_uuid_to_scope_address},
};

/// Fetches an AssetScopeAttribute by either the asset uuid or the scope address.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `identifier` Helps derive a unique key that can locate an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
pub fn query_asset_scope_attribute(
    deps: &Deps,
    identifier: AssetIdentifier,
) -> AssetResult<Binary> {
    let scope_attributes = match identifier {
        AssetIdentifier::AssetUuid(asset_uuid) => {
            may_query_scope_attribute_by_asset_uuid(deps, asset_uuid)
        }
        AssetIdentifier::ScopeAddress(scope_address) => {
            may_query_scope_attribute_by_scope_address(deps, scope_address)
        }
    }?;
    to_json_binary(&scope_attributes)?.to_ok()
}

/// Fetches an AssetScopeAttribute by the asset uuid value directly.  Useful for internal contract
/// functionality.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `asset_uuid` Directly links to the [asset_uuid](crate::core::types::asset_scope_attribute::AssetScopeAttribute::asset_uuid)
/// value on an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
pub fn query_scope_attribute_by_asset_uuid<S: Into<String>>(
    deps: &Deps,
    asset_uuid: S,
) -> AssetResult<Vec<AssetScopeAttribute>> {
    query_scope_attribute_by_scope_address(deps, asset_uuid_to_scope_address(asset_uuid)?)
}

/// Fetches an AssetScopeAttribute by the scope address value directly.  The most efficient version
/// of these functions, but still has to do quite a few lookups.  This functionality should only be used
/// on a once-per-transaction basis, if possible.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `scope_address` Directly links to the [scope_address](crate::core::types::asset_scope_attribute::AssetScopeAttribute::scope_address)
/// value on an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
pub fn query_scope_attribute_by_scope_address<S: Into<String>>(
    deps: &Deps,
    scope_address: S,
) -> AssetResult<Vec<AssetScopeAttribute>> {
    let scope_address_str = scope_address.into();
    let scope_attributes =
        may_query_scope_attribute_by_scope_address(deps, scope_address_str.clone())?;
    // This is a normal scenario, which just means the scope didn't have an attribute.  This can happen if a scope was
    // never registered by using onboard_asset.
    if let Some(attrs) = scope_attributes {
        attrs.to_ok()
    } else {
        ContractError::NotFound {
            explanation: format!(
                "scope at address [{}] did not include an asset scope attribute",
                scope_address_str
            ),
        }
        .to_err()
    }
}

/// Fetches an AssetScopeAttribute by the scope address value, derived from the asset uuid.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `asset_uuid` Directly links to the [asset_uuid](crate::core::types::asset_scope_attribute::AssetScopeAttribute::asset_uuid)
/// value on an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
pub fn may_query_scope_attribute_by_asset_uuid<S: Into<String>>(
    deps: &Deps,
    asset_uuid: S,
) -> AssetResult<Option<Vec<AssetScopeAttribute>>> {
    may_query_scope_attribute_by_scope_address(deps, asset_uuid_to_scope_address(asset_uuid)?)
}

/// Fetches a list of AssetScopeAttribute by the scope address value directly.  The most efficient version
/// of these functions.  This functionality should only be used
/// on a once-per-transaction basis, if possible. Returns Empty in the case of no attributes
/// being associated with the scope.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `scope_address` Directly links to the [scope_address](crate::core::types::asset_scope_attribute::AssetScopeAttribute::scope_address)
/// value on an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
pub fn may_query_scope_attribute_by_scope_address<S: Into<String>>(
    deps: &Deps,
    scope_address: S,
) -> AssetResult<Option<Vec<AssetScopeAttribute>>> {
    let metadata_querier = MetadataQuerier::new(&deps.querier);
    let scope_address_str: String = scope_address.into();

    // First, query up the scope to verify it exists
    let _scope = metadata_querier.scope(
        scope_address_str.to_owned(),
        String::from(""),
        String::from(""),
        false,
        false,
        false,
        false,
    )?;

    // Second, query up all possible asset definition names
    let state = STATE_V2.load(deps.storage)?;
    let asset_definitions: HashSet<String> = list_asset_definitions_v3(deps.storage)
        .iter()
        .map(|def| def.attribute_name_state(&state))
        .collect();

    // Third, query up asset scope attributes attached to the scope address under the name attribute.
    // In a proper scenario, there should only ever be one of each type of these
    let attribute_querier = AttributeQuerier::new(&deps.querier);
    let scope_attributes_v: Vec<(String, AssetScopeAttribute)> = attribute_querier
        .attributes(scope_address_str.to_owned(), None)?
        .attributes
        .iter()
        .filter(|attr| {
            asset_definitions.contains(&attr.name) && attr.attribute_type() == AttributeType::Json
        })
        .map(|attr| {
            from_json::<AssetScopeAttribute>(&attr.value)
                .map(|v| (attr.name.clone(), v))
                .map_err(ContractError::Std)
        })
        .collect::<AssetResult<_>>()?;
    let scope_attributes = &mut HashMap::new();
    let scope_attributes =
        scope_attributes_v
            .iter()
            .fold(scope_attributes, |acc, (name, value)| {
                acc.entry(name.to_owned())
                    .or_insert_with(Vec::new)
                    .push(value.to_owned());
                acc
            });

    for (name, scope_attributes) in scope_attributes.iter() {
        // This is a very bad scenario - this means that the contract messed up and created multiple attributes under
        // the attribute name.  This should only ever happen in error, and would require a horrible cleanup process
        // that manually removed the bad attributes
        if scope_attributes.len() > 1 {
            return ContractError::generic(format!(
                "more than one asset scope attribute for name [{}] exists at address [{}]. data repair needed",
                name,
                scope_address_str,
            ))
            .to_err();
        }
    }

    let single_attributes = scope_attributes
        .iter()
        .map(|(_, attributes)| attributes.first().unwrap().to_owned())
        .collect::<Vec<AssetScopeAttribute>>();
    if single_attributes.is_empty() {
        None.to_ok()
    } else {
        Some(single_attributes).to_ok()
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_json, StdError};
    use provwasm_mocks::mock_provenance_dependencies;
    use provwasm_std::types::provenance::metadata::v1::ScopeRequest;

    use crate::core::types::access_definition::{AccessDefinition, AccessDefinitionType};
    use crate::core::types::asset_onboarding_status::AssetOnboardingStatus;
    use crate::testutil::onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset};
    use crate::testutil::test_constants::DEFAULT_SCOPE_ADDRESS;
    use crate::testutil::test_utilities::{
        mock_single_scope_attribute, setup_no_attribute_response, setup_test_suite,
    };
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

    use super::query_asset_scope_attribute;

    #[test]
    fn test_successful_query_result() {
        let mut deps = mock_provenance_dependencies();
        setup_test_suite(&mut deps, &InstArgs::default());
        setup_no_attribute_response(&mut deps, None);
        let onboard_asset_request = TestOnboardAsset::default();
        test_onboard_asset(&mut deps, onboard_asset_request.to_owned())
            .expect("expected the asset onboard to succeed");
        mock_single_scope_attribute(
            &mut deps,
            &AssetScopeAttribute {
                asset_uuid: DEFAULT_ASSET_UUID.to_string(),
                scope_address: DEFAULT_SCOPE_ADDRESS.to_string(),
                asset_type: onboard_asset_request.onboard_asset.asset_type,
                requestor_address: onboard_asset_request.info.sender.to_owned(),
                verifier_address: cosmwasm_std::Addr::unchecked(
                    onboard_asset_request.onboard_asset.verifier_address,
                ),
                onboarding_status: AssetOnboardingStatus::Pending,
                latest_verification_result: None,
                access_definitions: vec![AccessDefinition::new_checked(
                    onboard_asset_request.info.sender,
                    onboard_asset_request.onboard_asset.access_routes,
                    AccessDefinitionType::Requestor,
                )
                .unwrap()],
            },
            DEFAULT_SCOPE_ADDRESS,
        );
        let binary_from_asset_uuid = query_asset_scope_attribute(
            &deps.as_ref(),
            AssetIdentifier::asset_uuid(DEFAULT_ASSET_UUID),
        )
        .expect("expected the scope attribute to be fetched as binary by asset uuid");
        let scope_attribute_from_asset_uuid =
            from_json::<Option<Vec<AssetScopeAttribute>>>(&binary_from_asset_uuid)
                .expect(
                    "expected the asset attribute fetched by asset uuid to deserialize properly",
                )
                .expect("expected the asset attribute to be present in the resulting Option");
        let binary_from_scope_address = query_asset_scope_attribute(
            &deps.as_ref(),
            AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
        )
        .expect("expected the scope attribute to be fetched as binary by scope address");
        let scope_attribute_from_scope_address = from_json::<Option<Vec<AssetScopeAttribute>>>(
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
    fn test_query_failure_for_invalid_scope_address() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        ScopeRequest::mock_failed_response(
            &mut deps.querier,
            String::from("scope fetch failed due to invalid address"),
        );
        let error = query_asset_scope_attribute(
            &deps.as_ref(),
            AssetIdentifier::scope_address("missing-scope-address"),
        )
        .unwrap_err();
        match error {
            ContractError::Std(e) => match e {
                StdError::GenericErr { msg, .. } => {
                    assert!(
                        msg.contains("scope fetch failed due to invalid address"),
                        "the error should be due to the metadata module scope fetch"
                    );
                }
                _ => panic!("unexpected StdError encountered: {:?}", e),
            },
            _ => panic!("unexpected error type encountered: {:?}", error),
        };
    }

    // Note - the mock querier does not allow the capability for mocking multiple attributes under a single name
    // so we cannot test the error where there are multiple attributes accidentally registered.  This is an
    // unfortunate lack of test coverage, but that error should only ever occur if this contract is mis-coded to
    // not remove existing attributes before adding new ones
    #[test]
    fn test_query_failure_for_missing_scope_attribute() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let scope_address = "scope-address".to_string();
        // Wire up the scope correctly to point to the correct address
        mock_scope(
            &mut deps,
            &scope_address,
            DEFAULT_SCOPE_SPEC_ADDRESS,
            "test-owner",
        );
        setup_no_attribute_response(&mut deps, Some(scope_address.to_owned()));
        let binary = query_asset_scope_attribute(
            &deps.as_ref(),
            AssetIdentifier::scope_address(&scope_address),
        )
        .expect("the query should execute without error");
        let result = from_json::<Option<AssetScopeAttribute>>(&binary)
            .expect("expected the result to deserialize correctly");
        assert!(
            result.is_none(),
            "expected the result from the query to be missing because no scope attribute existed at the scope address",
        );
    }
}
