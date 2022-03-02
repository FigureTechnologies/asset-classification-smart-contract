use crate::core::state::asset_state_read;
use crate::util::aliases::{ContractResult, DepsC};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{to_binary, Binary};

pub fn query_asset_definition<S: Into<String>>(
    deps: &DepsC,
    asset_type: S,
) -> ContractResult<Binary> {
    let asset_definition = asset_state_read(deps.storage, asset_type).load()?;
    to_binary(&asset_definition)?.to_ok()
}

#[cfg(test)]
mod tests {
    use crate::core::error::ContractError::Std;
    use crate::core::state::{asset_state, AssetDefinition};
    use crate::query::query_asset_definition::query_asset_definition;
    use crate::testutil::test_utilities::{
        get_default_asset_definition, test_instantiate_success, InstArgs,
    };
    use crate::util::aliases::DepsC;
    use cosmwasm_std::{from_binary, StdError};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_successful_query_from_instantiation() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        // This is the same value that gets added when the contract is instantiated with InstArgs::default()
        let default_asset_definition = get_default_asset_definition();
        let from_query_def =
            get_asset_from_query(&deps.as_ref(), &default_asset_definition.asset_type);
        assert_eq!(
            default_asset_definition, from_query_def,
            "expected the query value to equate to the value added during instantiation",
        );
    }

    #[test]
    fn test_successful_query_from_direct_serialization() {
        let mut deps = mock_dependencies(&[]);
        let asset_def = get_default_asset_definition();
        asset_state(deps.as_mut().storage, &asset_def.asset_type)
            .save(&asset_def)
            .expect("expected the asset definition to be properly saved to state");
        let query_def = get_asset_from_query(&deps.as_ref(), &asset_def.asset_type);
        assert_eq!(
            asset_def, query_def,
            "expected the query value to equate to the value directly added to the state",
        );
    }

    #[test]
    fn test_error_is_returned_when_asset_state_is_not_found() {
        let error =
            query_asset_definition(&mock_dependencies(&[]).as_ref(), "fakeloan").unwrap_err();
        assert!(
            matches!(error, Std(StdError::NotFound { .. })),
            "a not found error should be returned when the loan type is not registered",
        );
    }

    fn get_asset_from_query<S: Into<String>>(deps: &DepsC, asset_type: S) -> AssetDefinition {
        let bin = query_asset_definition(deps, asset_type)
            .expect("the query should successfully serialize the value in storage as binary");
        from_binary::<AssetDefinition>(&bin).expect("binary deserialization should succeed")
    }
}
