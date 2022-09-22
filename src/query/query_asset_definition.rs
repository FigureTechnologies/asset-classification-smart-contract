use crate::core::state::may_load_asset_definition_by_type_v3;
use crate::util::aliases::{AssetResult, DepsC};

use cosmwasm_std::{to_binary, Binary};
use result_extensions::ResultExtensions;

/// A query that fetches a target [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3)
/// from the contract's internal storage.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `asset_type` The asset type corresponding to the stored
/// [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3) to look up.
pub fn query_asset_definition(deps: &DepsC, asset_type: &str) -> AssetResult<Binary> {
    let asset_definition = may_load_asset_definition_by_type_v3(deps.storage, asset_type)?;
    to_binary(&asset_definition)?.to_ok()
}

#[cfg(test)]
mod tests {
    use crate::core::state::insert_asset_definition_v3;
    use crate::core::types::asset_definition::AssetDefinitionV3;
    use crate::query::query_asset_definition::query_asset_definition;
    use crate::testutil::test_utilities::{
        get_default_asset_definition, test_instantiate_success, InstArgs,
    };
    use crate::util::aliases::DepsC;
    use cosmwasm_std::from_binary;
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
    fn test_successful_query_from_direct_serialization_for_asset_type() {
        let mut deps = mock_dependencies(&[]);
        let asset_def = get_default_asset_definition();
        insert_asset_definition_v3(deps.as_mut().storage, &asset_def)
            .expect("expected the asset definition to be properly saved to state");
        let query_def = get_asset_from_query_by_asset_type(&deps.as_ref(), &asset_def.asset_type);
        assert_eq!(
            asset_def, query_def,
            "expected the query value to equate to the value directly added to the state",
        );
    }

    #[test]
    fn test_none_is_returned_when_asset_definition_is_not_found_by_asset_type() {
        let binary = query_asset_definition(&mock_dependencies(&[]).as_ref(), "fakeloan")
            .expect("the query should execute without error");
        let result = from_binary::<Option<AssetDefinitionV3>>(&binary)
            .expect("expected the binary to deserialize appropriately");
        assert!(
            result.is_none(),
            "the resulting binary should be an empty Option",
        );
    }

    fn get_asset_from_query_by_asset_type<S: Into<String>>(
        deps: &DepsC,
        asset_type: S,
    ) -> AssetDefinitionV3 {
        let bin = query_asset_definition(deps, &asset_type.into()).expect(
            "the query should successfully serialize the value in storage as binary without error",
        );
        from_binary::<Option<AssetDefinitionV3>>(&bin)
            .expect("binary deserialization should succeed")
            .expect("expected the deserialized option to be populated")
    }
}
