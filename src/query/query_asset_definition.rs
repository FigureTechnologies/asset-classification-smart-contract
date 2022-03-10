use crate::core::msg::AssetQualifier;
use crate::core::state::{load_asset_definition_by_scope_spec, load_asset_definition_by_type};
use crate::util::aliases::{ContractResult, DepsC};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{to_binary, Binary};

pub fn query_asset_definition(deps: &DepsC, qualifier: AssetQualifier) -> ContractResult<Binary> {
    let asset_definition = match qualifier {
        AssetQualifier::AssetType { asset_type } => {
            load_asset_definition_by_type(deps.storage, asset_type)
        }
        AssetQualifier::ScopeSpecAddress { scope_spec_address } => {
            load_asset_definition_by_scope_spec(deps.storage, scope_spec_address)
        }
    }?;
    to_binary(&asset_definition)?.to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::core::asset::AssetDefinition;
    use crate::core::error::ContractError::{self, Std};
    use crate::core::msg::AssetQualifier;
    use crate::core::state::insert_asset_definition;
    use crate::query::query_asset_definition::query_asset_definition;
    use crate::testutil::test_utilities::{
        get_default_asset_definition, test_instantiate_success, InstArgs,
    };
    use crate::util::aliases::DepsC;
    use cosmwasm_std::{from_binary, StdError};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_successful_query_from_instantiation_for_asset_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        // This is the same value that gets added when the contract is instantiated with InstArgs::default()
        let default_asset_definition = get_default_asset_definition();
        let from_query_def = get_asset_from_query_by_asset_type(
            &deps.as_ref(),
            &default_asset_definition.asset_type,
        );
        assert_eq!(
            default_asset_definition, from_query_def,
            "expected the query value to equate to the value added during instantiation",
        );
    }

    #[test]
    fn test_successful_query_from_instantiation_for_scope_spec() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        // This is the same value that gets added when the contract is instantiated with InstArgs::default()
        let default_asset_definition = get_default_asset_definition();
        let from_query_def = get_asset_from_query_by_scope_spec(
            &deps.as_ref(),
            &default_asset_definition.scope_spec_address,
        );
        assert_eq!(
            default_asset_definition, from_query_def,
            "expected the query value to equate to the value added during instantiation",
        );
    }

    #[test]
    fn test_successful_query_from_direct_serialization_for_asset_type() {
        let mut deps = mock_dependencies(&[]);
        let asset_def = get_default_asset_definition();
        insert_asset_definition(deps.as_mut().storage, &asset_def)
            .expect("expected the asset definition to be properly saved to state");
        let query_def = get_asset_from_query_by_asset_type(&deps.as_ref(), &asset_def.asset_type);
        assert_eq!(
            asset_def, query_def,
            "expected the query value to equate to the value directly added to the state",
        );
    }

    #[test]
    fn test_successful_query_from_direct_serialization_for_scope_spec() {
        let mut deps = mock_dependencies(&[]);
        let asset_def = get_default_asset_definition();
        insert_asset_definition(deps.as_mut().storage, &asset_def)
            .expect("expected the asset definition to be properly saved to state");
        let query_def =
            get_asset_from_query_by_scope_spec(&deps.as_ref(), &asset_def.scope_spec_address);
        assert_eq!(
            asset_def, query_def,
            "expected the query value to equate to the value directly added to the state",
        );
    }

    #[test]
    fn test_error_is_returned_when_asset_definition_is_not_found_by_asset_type() {
        let error = query_asset_definition(
            &mock_dependencies(&[]).as_ref(),
            AssetQualifier::asset_type("fakeloan"),
        )
        .unwrap_err();
        assert!(
            matches!(error, Std(StdError::NotFound { .. })),
            "a not found error should be returned when the asset type is not registered, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_error_is_returned_when_asset_definition_is_not_found_by_scope_spec() {
        let error = query_asset_definition(
            &mock_dependencies(&[]).as_ref(),
            AssetQualifier::scope_spec_address("fakescopespec"),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::RecordNotFound { .. }),
            "a record not found error should be returned when the scope spec is not registered, but got: {:?}",
            error,
        );
    }

    fn get_asset_from_query_by_asset_type<S: Into<String>>(
        deps: &DepsC,
        asset_type: S,
    ) -> AssetDefinition {
        let bin = query_asset_definition(deps, AssetQualifier::asset_type(asset_type))
            .expect("the query should successfully serialize the value in storage as binary");
        from_binary::<AssetDefinition>(&bin).expect("binary deserialization should succeed")
    }

    fn get_asset_from_query_by_scope_spec<S: Into<String>>(
        deps: &DepsC,
        scope_spec_address: S,
    ) -> AssetDefinition {
        let bin =
            query_asset_definition(deps, AssetQualifier::scope_spec_address(scope_spec_address))
                .expect("the query should successfully serialize the value in storage as binary");
        from_binary::<AssetDefinition>(&bin).expect("binary deserialization should succeed")
    }
}
