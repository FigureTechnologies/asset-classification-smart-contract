use crate::{
    core::{error::ContractError, state::AssetScopeAttribute},
    query::query_asset_by_scope_address::query_asset_attribute_by_scope_address,
};
use cosmwasm_std::{Addr, CosmosMsg, Deps, QuerierWrapper, StdResult};
use provwasm_std::{
    add_json_attribute, delete_attributes, ProvenanceMsg, ProvenanceQuerier, ProvenanceQuery, Scope,
};

use super::functions::generate_asset_attribute_name;

/// Defines a ProvenanceUtil instance.  This value should be used to query provenance modules or to
/// generate messages utilizing provwasm helper functions.
pub trait ProvenanceUtil {
    /// Returns a provwasm Scope struct for the given scope_id, which is a bech32 address prefixed
    /// with "scope"
    fn get_scope_by_id(
        &self,
        querier: &QuerierWrapper<ProvenanceQuery>,
        scope_id: impl Into<String>,
    ) -> StdResult<Scope>;

    /// Derives a CosmosMsg<ProvenanceMsg> Custom wrapper that will add a AssetScopeAttribute to
    /// a target scope.  The target scope should be defined by the scope_id value within the
    /// AssetScopeAttribute parameter.
    fn get_add_initial_attribute_to_scope_msg(
        &self,
        deps: &Deps<ProvenanceQuery>,
        scope_address: impl Into<String>,
        asset_type: impl Into<String>,
        attribute: &AssetScopeAttribute,
        contract_name: impl Into<String>,
    ) -> Result<CosmosMsg<ProvenanceMsg>, ContractError>;

    /// Provwasm currently does not expose an "update attribute" functionality, so this function is
    /// a placeholder that should delete all attributes listed until the provided contract_name, and
    /// add a new attribute correlating to the json values of the provided AssetScopeAttribute.
    /// The target scope should be defined by the scope_id value within the AssetScopeAttribute
    /// parameter.
    fn upsert_attribute_to_scope(
        &self,
        scope_address: impl Into<String>,
        asset_type: impl Into<String>,
        attribute: &AssetScopeAttribute,
        contract_name: impl Into<String>,
    ) -> Result<WriteAttributeMessages, ContractError>;
}

/// The core production ProvenanceUtil instance.  A static struct instance for re-use throughout the
/// various execution flows.
pub struct ProvenanceUtilImpl;

impl ProvenanceUtil for ProvenanceUtilImpl {
    /// Simply generates a ProvenanceQuerier from the QuerierWrapper and invokes get_scope for the
    /// given scope_id.
    fn get_scope_by_id(
        &self,
        querier: &QuerierWrapper<ProvenanceQuery>,
        scope_id: impl Into<String>,
    ) -> StdResult<Scope> {
        ProvenanceQuerier::new(querier).get_scope(scope_id)
    }

    /// Checks to determine if the scope has already been registered with an attribute for this
    /// contract.  If so, returns a ContractError.  If not, generates an add attribute message.
    fn get_add_initial_attribute_to_scope_msg(
        &self,
        deps: &Deps<ProvenanceQuery>,
        scope_address: impl Into<String>,
        asset_type: impl Into<String>,
        attribute: &AssetScopeAttribute,
        contract_name: impl Into<String>,
    ) -> Result<CosmosMsg<ProvenanceMsg>, ContractError> {
        if query_asset_attribute_by_scope_address(deps, &scope_address, asset_type).is_ok() {
            return ContractError::AssetAlreadyOnboarded {
                scope_address: scope_address.into(),
            }
            .to_result();
        }
        super::provenance_util::get_add_attribute_to_scope_msg(
            scope_address,
            attribute,
            contract_name,
        )
    }

    /// Forgoes validation on whether or not the scope exists, because the current attribute (if any)
    /// on the scope will be deleted.  Generates a deletion and addition message, as the trait
    /// documentation implies.
    fn upsert_attribute_to_scope(
        &self,
        scope_address: impl Into<String>,
        asset_type: impl Into<String>,
        attribute: &AssetScopeAttribute,
        contract_base_name: impl Into<String>,
    ) -> Result<WriteAttributeMessages, ContractError> {
        let contract_name = contract_name.into();
        let delete_attributes_msg = delete_attributes(
            Addr::unchecked(scope_address),
            generate_asset_attribute_name(asset_type, contract_base_name),
        )
        .map_err(ContractError::Std)?;
        let add_attribute_msg = super::provenance_util::get_add_attribute_to_scope_msg(
            scope_address,
            asset_type,
            attribute,
            &contract_name,
        )?;
        Ok(WriteAttributeMessages {
            delete_attributes_msg,
            add_attribute_msg,
        })
    }
}

/// Helper function to generate an "add attribute" message, as the functionality is re-used across
/// multiple functions.
fn get_add_attribute_to_scope_msg(
    scope_address: impl Into<String>,
    asset_type: impl Into<String>,
    attribute: &AssetScopeAttribute,
    contract_base_name: impl Into<String>,
) -> Result<CosmosMsg<ProvenanceMsg>, ContractError> {
    add_json_attribute(
        // Until there's a way to parse a scope address as an Addr, we must use Addr::unchecked.
        // It's not the best policy, but contract execution will fail if it's an incorrect address,
        // so it'll just fail later down the line with a less sane error message than if it was
        // being properly checked.
        Addr::unchecked(scope_address),
        generate_asset_attribute_name(asset_type, contract_base_name),
        attribute,
    )
    .map_err(ContractError::Std)
}

/// Helper struct - contains both a delete and add attribute message for the response of
/// upsert_attribute_to_scope in the ProvenanceUtil trait.
pub struct WriteAttributeMessages {
    delete_attributes_msg: CosmosMsg<ProvenanceMsg>,
    add_attribute_msg: CosmosMsg<ProvenanceMsg>,
}
impl WriteAttributeMessages {
    /// Helper function to convert both messages to a properly ordered Vec for easy insertion into
    /// cosomwasm Response structs.
    pub fn to_vec(self) -> Vec<CosmosMsg<ProvenanceMsg>> {
        vec![self.delete_attributes_msg, self.add_attribute_msg]
    }
}
