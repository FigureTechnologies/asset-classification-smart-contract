use cosmwasm_std::{Addr, CosmosMsg};

use crate::core::{error::ContractError, types::asset_scope_attribute::AssetScopeAttribute};

use super::{
    aliases::AssetResult,
    functions::{add_json_attribute, generate_asset_attribute_name},
};

/// Helper function to generate an "add attribute" message, as the functionality is re-used across
/// multiple functions.
///
/// # Parameters
///
/// * `attribute` The scope attribute to be added to a Provenance Metadata Scope.
/// * `base_contract_name` The base name of the contract, defined in the [base_contract_name](crate::core::state::StateV2::base_contract_name)
/// property of the [StateV2](crate::core::state::StateV2) value stored internally in the contract.
pub fn get_add_attribute_to_scope_msg(
    attribute: &AssetScopeAttribute,
    base_contract_name: impl Into<String>,
    contract_address: Addr,
) -> AssetResult<CosmosMsg> {
    add_json_attribute(
        // Until there's a way to parse a scope address as an Addr, we must use Addr::unchecked.
        // It's not the best policy, but contract execution will fail if it's an incorrect address,
        // so it'll just fail later down the line with a less sane error message than if it was
        // being properly checked.
        Addr::unchecked(&attribute.scope_address),
        contract_address,
        generate_asset_attribute_name(&attribute.asset_type, base_contract_name),
        attribute,
    )
    .map_err(ContractError::Std)
}
