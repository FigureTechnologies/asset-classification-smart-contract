use crate::core::error::ContractError;
use crate::core::state::AssetScopeAttribute;
use crate::testutil::test_utilities::{
    mock_default_scope_attribute, mock_scope_attribute, MockOwnedDeps,
};
use crate::util::provenance_util::{ProvenanceUtil, ProvenanceUtilImpl, WriteAttributeMessages};
use cosmwasm_std::{CosmosMsg, Deps, QuerierWrapper, StdResult};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery, Scope};
use std::cell::RefCell;

pub struct MockProvenanceUtil {
    captured_attributes: RefCell<Vec<(String, AssetScopeAttribute)>>,
}
impl MockProvenanceUtil {
    pub fn new() -> MockProvenanceUtil {
        MockProvenanceUtil {
            captured_attributes: RefCell::new(vec![]),
        }
    }

    fn add_attribute(&self, scope_address: impl Into<String>, attribute: AssetScopeAttribute) {
        self.captured_attributes
            .borrow_mut()
            .push((scope_address.into(), attribute));
    }
}
impl ProvenanceUtil for MockProvenanceUtil {
    fn get_scope_by_id(
        &self,
        querier: &QuerierWrapper<ProvenanceQuery>,
        scope_id: impl Into<String>,
    ) -> StdResult<Scope> {
        ProvenanceUtilImpl.get_scope_by_id(querier, scope_id)
    }

    fn get_add_initial_attribute_to_scope_msg(
        &self,
        deps: &Deps<ProvenanceQuery>,
        scope_address: impl Into<String>,
        asset_type: impl Into<String>,
        attribute: &AssetScopeAttribute,
        contract_name: impl Into<String>,
    ) -> Result<CosmosMsg<ProvenanceMsg>, ContractError> {
        self.add_attribute(scope_address, attribute.clone());
        ProvenanceUtilImpl.get_add_initial_attribute_to_scope_msg(
            deps,
            scope_address,
            asset_type,
            attribute,
            contract_name,
        )
    }

    fn upsert_attribute_to_scope(
        &self,
        scope_address: impl Into<String>,
        asset_type: impl Into<String>,
        attribute: &AssetScopeAttribute,
        contract_name: impl Into<String>,
    ) -> Result<WriteAttributeMessages, ContractError> {
        self.add_attribute(scope_address, attribute.clone());
        ProvenanceUtilImpl.upsert_attribute_to_scope(
            scope_address,
            asset_type,
            attribute,
            contract_name,
        )
    }
}
impl MockProvenanceUtil {
    pub fn bind_captured_attribute(&self, deps: &mut MockOwnedDeps) {
        if let Some((scope_address, attr)) = self.captured_attributes.borrow().last() {
            mock_default_scope_attribute(deps, scope_address, attr);
        }
    }

    pub fn bind_captured_attribute_named(
        &self,
        deps: &mut MockOwnedDeps,
        contract_name: impl Into<String>,
    ) {
        if let Some((scope_address, attr)) = self.captured_attributes.borrow().last() {
            mock_scope_attribute(deps, contract_name, scope_address, attr);
        }
    }

    pub fn assert_attribute_matches_latest(&self, attribute: &AssetScopeAttribute) {
        if let Some((scope_address, attr)) = self.captured_attributes.borrow().last() {
            assert_eq!(
                attribute,
                attr,
                "the latest attribute captured via MockProvenanceUtil is not equivalent to the checked value",
            );
        } else {
            panic!("no attributes have ever been captured by MockProvenanceUtil");
        }
    }
}
