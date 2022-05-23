use cosmwasm_std::{to_binary, Binary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::state::asset_definitions_v2;
use crate::core::types::asset_definition::AssetDefinitionV2;
use crate::util::{
    aliases::{AssetResult, DepsC},
    traits::ResultExtensions,
};

/// A simple wrapper for all asset definitions returned as a result of the [query_asset_definitions](self::query_asset_definitions)
/// function.
#[derive(Serialize, Deserialize, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryAssetDefinitionsResponse {
    /// All derived asset definitions derived from the [query_asset_definitions](self::query_asset_definitions)
    /// function.
    pub asset_definitions: Vec<AssetDefinitionV2>,
}
impl QueryAssetDefinitionsResponse {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `asset_definitions` All derived asset definitions derived from the [query_asset_definitions](self::query_asset_definitions)
    /// function.
    pub fn new(asset_definitions: Vec<AssetDefinitionV2>) -> Self {
        Self { asset_definitions }
    }
}

/// A query that fetches all [AssetDefinitionV2s](crate::core::types::asset_definition::AssetDefinitionV2)
/// from the contract's internal storage.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
pub fn query_asset_definitions(deps: &DepsC) -> AssetResult<Binary> {
    let asset_definitions = asset_definitions_v2()
        .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
        .into_iter()
        .filter(|result| result.is_ok())
        .map(|result| result.unwrap().1)
        .collect::<Vec<AssetDefinitionV2>>();
    to_binary(&QueryAssetDefinitionsResponse::new(asset_definitions))?.to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::{from_binary, Uint128};
    use provwasm_mocks::mock_dependencies;
    use uuid::Uuid;

    use crate::core::types::asset_definition::AssetDefinitionInputV2;
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::util::traits::OptionExtensions;
    use crate::{
        core::types::scope_spec_identifier::ScopeSpecIdentifier,
        query::query_asset_definitions::QueryAssetDefinitionsResponse,
        testutil::{
            test_constants::DEFAULT_VERIFIER_ADDRESS,
            test_utilities::{get_default_asset_definition, test_instantiate_success, InstArgs},
        },
    };

    use super::query_asset_definitions;

    #[test]
    fn test_empty_result() {
        let deps = mock_dependencies(&[]);
        let response_bin = query_asset_definitions(&deps.as_ref())
            .expect("expected the query to execute appropriately");
        let query_response = from_binary::<QueryAssetDefinitionsResponse>(&response_bin)
            .expect("expected the query to deserialize from binary correctly");
        assert!(
            query_response.asset_definitions.is_empty(),
            "expected no asset definitions to exist due to the contract not being instantiated"
        );
    }

    #[test]
    fn test_default_instantiation_result() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let response_bin = query_asset_definitions(&deps.as_ref())
            .expect("expected the query to execute appropriately");
        let query_response = from_binary::<QueryAssetDefinitionsResponse>(&response_bin)
            .expect("expected the query to deserialize from binary correctly");
        assert_eq!(
            1,
            query_response.asset_definitions.len(),
            "expected only one asset definition to be in the response",
        );
        let default_definition = get_default_asset_definition();
        assert_eq!(
            &default_definition,
            query_response.asset_definitions.first().unwrap(),
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
                AssetDefinitionInputV2::new(
                    format!("asset_type_{}", id),
                    ScopeSpecIdentifier::Uuid(Uuid::new_v4().to_string()).to_serialized_enum(),
                    vec![VerifierDetailV2::new(
                        DEFAULT_VERIFIER_ADDRESS,
                        Uint128::new(150),
                        "nhash",
                        Uint128::zero(),
                        vec![],
                        None,
                    )],
                    true.to_some(),
                    true.to_some(),
                )
            })
            .collect::<Vec<AssetDefinitionInputV2>>();
        test_instantiate_success(
            deps.as_mut(),
            InstArgs {
                asset_definitions: asset_definition_inputs.clone(),
                ..Default::default()
            },
        );
        let response_bin = query_asset_definitions(&deps.as_ref())
            .expect("expected the query to execute appropriately");
        let query_response = from_binary::<QueryAssetDefinitionsResponse>(&response_bin)
            .expect("expected the query to deserialize from binary correctly");
        assert_eq!(
            20,
            query_response.asset_definitions.len(),
            "expected all 20 asset definitions to be included in the response",
        );
        asset_definition_inputs
            .into_iter()
            .map(|input| {
                input
                    .into_asset_definition()
                    .expect("expected the input to correctly translate to an asset definition")
            })
            .for_each(|asset_definition| {
                assert!(
                    query_response
                        .asset_definitions
                        .iter()
                        .any(|def| def == &asset_definition),
                    "expected the asset definition of type [{}] to be found in the query response",
                    asset_definition.asset_type,
                );
            });
    }
}
