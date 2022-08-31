use cosmwasm_std::{to_binary, Binary};

use crate::core::state::list_asset_definitions_v3;
use crate::util::{
    aliases::{AssetResult, DepsC},
    traits::ResultExtensions,
};

/// A query that fetches all [AssetDefinitionV2s](crate::core::types::asset_definition::AssetDefinitionV2)
/// from the contract's internal storage.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
pub fn query_asset_definitions(deps: &DepsC) -> AssetResult<Binary> {
    let asset_definitions = list_asset_definitions_v3(deps.storage);
    to_binary(&asset_definitions)?.to_ok()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{from_binary, Uint128};
    use provwasm_mocks::mock_dependencies;

    use crate::core::types::asset_definition::{AssetDefinitionInputV3, AssetDefinitionV3};
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::testutil::{
        test_constants::DEFAULT_VERIFIER_ADDRESS,
        test_utilities::{get_default_asset_definition, test_instantiate_success, InstArgs},
    };
    use crate::util::traits::OptionExtensions;

    use super::query_asset_definitions;

    #[test]
    fn test_empty_result() {
        let deps = mock_dependencies(&[]);
        let response_bin = query_asset_definitions(&deps.as_ref())
            .expect("expected the query to execute appropriately");
        let query_response = from_binary::<Vec<AssetDefinitionV3>>(&response_bin)
            .expect("expected the query to deserialize from binary correctly");
        assert!(
            query_response.is_empty(),
            "expected no asset definitions to exist due to the contract not being instantiated"
        );
    }

    #[test]
    fn test_default_instantiation_result() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let response_bin = query_asset_definitions(&deps.as_ref())
            .expect("expected the query to execute appropriately");
        let query_response = from_binary::<Vec<AssetDefinitionV3>>(&response_bin)
            .expect("expected the query to deserialize from binary correctly");
        assert_eq!(
            1,
            query_response.len(),
            "expected only one asset definition to be in the response",
        );
        let default_definition = get_default_asset_definition();
        assert_eq!(
            &default_definition,
            query_response.first().unwrap(),
            "expected the response to include the default asset definition used in default instantiation",
        );
    }

    #[test]
    fn test_many_definitions_result() {
        let mut deps = mock_dependencies(&[]);
        let mut def_ids = Vec::with_capacity(20);
        // Populate a vec with 0-19 just for different asset definitions
        def_ids.extend(0..20);
        let asset_definition_inputs = def_ids
            .into_iter()
            .map(|id| {
                AssetDefinitionInputV3::new(
                    format!("asset_type_{}", id),
                    vec![VerifierDetailV2::new(
                        DEFAULT_VERIFIER_ADDRESS,
                        Uint128::new(150),
                        "nhash",
                        vec![],
                        None,
                    )],
                    true.to_some(),
                    true.to_some(),
                )
            })
            .collect::<Vec<AssetDefinitionInputV3>>();
        test_instantiate_success(
            deps.as_mut(),
            InstArgs {
                asset_definitions: asset_definition_inputs.clone(),
                ..Default::default()
            },
        );
        let response_bin = query_asset_definitions(&deps.as_ref())
            .expect("expected the query to execute appropriately");
        let query_response = from_binary::<Vec<AssetDefinitionV3>>(&response_bin)
            .expect("expected the query to deserialize from binary correctly");
        assert_eq!(
            20,
            query_response.len(),
            "expected all 20 asset definitions to be included in the response",
        );
        asset_definition_inputs
            .into_iter()
            .map(|input| input.into_asset_definition())
            .for_each(|asset_definition| {
                assert!(
                    query_response.iter().any(|def| def == &asset_definition),
                    "expected the asset definition of type [{}] to be found in the query response",
                    asset_definition.asset_type,
                );
            });
    }
}
