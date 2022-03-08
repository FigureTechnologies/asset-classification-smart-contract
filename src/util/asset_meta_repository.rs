use std::vec;

use cosmwasm_std::{QuerierWrapper, Storage};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

use crate::core::{error::ContractError, state::asset_meta_read};

use super::{
    aliases::ContractResult, message_gathering_service::MessageGatheringService,
    traits::ResultExtensions,
};

#[derive(Clone, PartialEq)]
pub struct AssetMeta {
    pub scope_address: String,
    pub asset_type: String,
    pub validator_address: String,
    pub onboarding_fee: String,
    pub validated: String,
}

pub trait AssetMetaRepository {
    fn has_asset(
        &self,
        storage: &dyn Storage,
        querier: &QuerierWrapper<ProvenanceQuery>,
        scope_address: String,
    ) -> ContractResult<bool>;

    fn add_asset(
        &self,
        scope_address: String,
        asset_type: String,
        validator_address: String,
    ) -> ContractResult<()>;

    fn get_asset(&self, scope_address: String) -> ContractResult<AssetMeta>;

    fn try_get_asset(&self, scope_address: String) -> Option<AssetMeta>;

    fn validate_asset(&self, scope_address: String, validation_result: bool) -> ContractResult<()>;
}

// An AssetMeta repository instance that stores the metadata split between contract storage and scope attribute
pub struct ContractAndAttributeAssetMeta {
    messages: Vec<ProvenanceMsg>,
}

impl ContractAndAttributeAssetMeta {
    pub fn new() -> Self {
        Self { messages: vec![] }
    }
}

impl AssetMetaRepository for ContractAndAttributeAssetMeta {
    fn has_asset(
        &self,
        storage: &dyn Storage,
        _querier: &QuerierWrapper<ProvenanceQuery>,
        scope_address: String,
    ) -> ContractResult<bool> {
        let asset_meta = asset_meta_read(storage);
        match asset_meta.may_load(scope_address.as_bytes()) {
            Ok(contains) => ContractResult::Ok(contains.is_some()),
            Err(err) => ContractError::Std(err).to_err(),
        }
        // check for asset in storage (and check for scope attribute existence if found?)
    }

    fn add_asset(
        &self,
        _scope_address: String,
        _asset_type: String,
        _validator_address: String,
    ) -> ContractResult<()> {
        todo!()
        // insert asset meta and generate attribute -> scope bind message
    }

    fn get_asset(&self, _scope_address: String) -> ContractResult<AssetMeta> {
        todo!()
        // try to fetch asset from attribute meta, if found also fetch scope attribute and reconstruct AssetMeta from relevant pieces
    }

    fn try_get_asset(&self, _scope_address: String) -> Option<AssetMeta> {
        todo!()
        // try/catch get_asset and transform to option
    }

    fn validate_asset(
        &self,
        _scope_address: String,
        _validation_result: bool,
    ) -> ContractResult<()> {
        todo!()
        // set validation result on asset (add messages to message service)
    }
}

impl MessageGatheringService for ContractAndAttributeAssetMeta {
    fn get_messages(&self) -> Vec<ProvenanceMsg> {
        self.messages.clone()
    }

    fn add_message(&mut self, message: ProvenanceMsg) {
        self.messages.push(message)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::QuerierWrapper;
    use provwasm_mocks::mock_dependencies;

    use crate::util::asset_meta_repository::AssetMetaRepository;

    use super::ContractAndAttributeAssetMeta;

    #[test]
    fn has_asset_returns_false_if_asset_not_in_storage() {
        let deps = mock_dependencies(&[]);
        let querier = QuerierWrapper::new(&deps.querier);

        let repository = ContractAndAttributeAssetMeta::new();

        let result = repository
            .has_asset(&deps.storage, &querier, "bogus".to_string())
            .unwrap();

        assert_eq!(
            false, result,
            "Repository should return false when asset not in storage"
        );
    }

    #[test]
    fn has_asset_returns_false_if_asset_in_storage_but_no_attribute() {
        // should probably not ever happen
    }

    #[test]
    fn has_asset_returns_true_if_asset_in_storage_and_has_attribute() {}

    #[test]
    fn add_asset_fails_if_asset_already_exists() {}

    #[test]
    fn add_asset_adds_to_storage_and_generates_proper_attribute_message() {}

    #[test]
    fn get_asset_returns_error_if_asset_does_not_exist() {}

    #[test]
    fn get_asset_returns_asset_if_exists() {}

    #[test]
    fn try_get_asset_returns_none_if_asset_does_not_exist() {}

    #[test]
    fn try_get_asset_returns_asset_if_exists() {}

    #[test]
    fn validate_asset_returns_error_if_asset_not_onboarded() {}

    #[test]
    fn validate_asset_generates_attribute_update_message_sequence() {}
}
