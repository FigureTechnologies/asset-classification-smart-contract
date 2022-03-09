use crate::core::{asset::AssetScopeAttribute, error::ContractError};
use cosmwasm_std::{Addr, CosmosMsg};
use provwasm_std::{add_json_attribute, ProvenanceMsg};

use super::functions::generate_asset_attribute_name;

/// Helper function to generate an "add attribute" message, as the functionality is re-used across
/// multiple functions.
pub fn get_add_attribute_to_scope_msg(
    scope_address: impl Into<String>,
    attribute: &AssetScopeAttribute,
    contract_base_name: impl Into<String>,
) -> Result<CosmosMsg<ProvenanceMsg>, ContractError> {
    add_json_attribute(
        // Until there's a way to parse a scope address as an Addr, we must use Addr::unchecked.
        // It's not the best policy, but contract execution will fail if it's an incorrect address,
        // so it'll just fail later down the line with a less sane error message than if it was
        // being properly checked.
        Addr::unchecked(scope_address),
        generate_asset_attribute_name(attribute.asset_type.clone(), contract_base_name),
        attribute,
    )
    .map_err(ContractError::Std)
}
