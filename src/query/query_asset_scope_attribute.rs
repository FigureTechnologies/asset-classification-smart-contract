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
    // This is a normal scenario, which just means the scope didn't have an attribute.  This can happen if a
    // scope was created with a scope spec that is attached to the contract via AssetDefinition, but the scope was
    // never registered by using onboard_asset.
    if scope_attributes.is_empty() {
        return ContractError::NotFound {
            explanation: format!(
                "scope at address [{}] did not include an asset scope attribute",
                &scope.scope_id
            ),
        }
        .to_err();
    }
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
    scope_attributes.first().unwrap().to_owned().to_ok()
}
