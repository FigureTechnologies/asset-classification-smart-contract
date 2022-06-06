use cosmwasm_std::{Addr, CosmosMsg};
use provwasm_std::{add_json_attribute, ProvenanceMsg};

use crate::core::{error::ContractError, types::asset_scope_attribute::AssetScopeAttribute};

use super::{aliases::AssetResult, functions::generate_asset_attribute_name};

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
) -> AssetResult<CosmosMsg<ProvenanceMsg>> {
    let get_msg = |attribute: &AssetScopeAttribute| {
        add_json_attribute(
            // Until there's a way to parse a scope address as an Addr, we must use Addr::unchecked.
            // It's not the best policy, but contract execution will fail if it's an incorrect address,
            // so it'll just fail later down the line with a less sane error message than if it was
            // being properly checked.
            Addr::unchecked(&attribute.scope_address),
            generate_asset_attribute_name(&attribute.asset_type, base_contract_name),
            attribute,
        )
        .map_err(ContractError::Std)
    };
    // Only clone and update the attribute if the latest_verifier_detail is populated.  This
    // ensures faster operations when storing an attribute without a latest_verifier_detail.
    if attribute.latest_verifier_detail.is_some() {
        // Ensures that the large latest_verifier_detail field is never populated when an
        // attribute is stored on the Provenance Blockchain.  The Attribute Metadata Module will
        // reject large payments (currently > 1kb as of the time of writing), so this verifier
        // detail value should be trimmed from all storage.
        let mut filtered_attribute = attribute.clone();
        filtered_attribute.latest_verifier_detail = None;
        get_msg(&filtered_attribute)
    } else {
        get_msg(attribute)
    }
}

/// Helper function to ensure that an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
/// has all of its large fields removed for storage on the Provenance Blockchain.  This is to ensure
/// that it does not get rejected from the Attribute metadata module for being too large.
///
/// # Parameters
///
/// * `attribute` An attribute that may or may not have
pub fn trim_attribute_for_storage(attribute: &AssetScopeAttribute) -> AssetScopeAttribute {
    let mut filtered_attribute = attribute.clone();
    filtered_attribute.latest_verifier_detail = None;
    filtered_attribute
}
