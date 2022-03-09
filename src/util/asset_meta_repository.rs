use std::vec;

use cosmwasm_std::{Addr, CosmosMsg, QuerierWrapper, Storage};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

use crate::{
    core::{
        asset::{AssetOnboardingStatus, AssetScopeAttribute, ValidatorDetail},
        error::ContractError,
        state::{asset_meta, asset_meta_read, config_read, AssetMeta},
    },
    query::query_asset_scope_attribute::{
        may_query_scope_attribute_by_scope_address, query_scope_attribute_by_scope_address,
    },
};

use super::{
    aliases::{ContractResult, DepsC},
    message_gathering_service::MessageGatheringService,
    provenance_util::get_add_attribute_to_scope_msg,
    traits::ResultExtensions,
};

pub trait AssetMetaRepository {
    fn has_asset<S1: Into<String>>(&self, deps: &DepsC, scope_address: S1) -> ContractResult<bool>;

    fn add_asset<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        &mut self,
        _deps: &DepsC,
        scope_address: S1,
        asset_type: S2,
        validator_address: S3,
        onboarding_status: AssetOnboardingStatus,
        validator_detail: ValidatorDetail,
    ) -> ContractResult<()>;

    fn get_asset<S1: Into<String>>(&self, scope_address: S1)
        -> ContractResult<AssetScopeAttribute>;

    fn try_get_asset<S1: Into<String>>(&self, scope_address: S1) -> Option<AssetScopeAttribute>;

    fn validate_asset<S1: Into<String>>(
        &self,
        scope_address: S1,
        validation_result: bool,
    ) -> ContractResult<()>;
}

// An AssetMeta repository instance that stores the metadata on a scope attribute
pub struct AttributeOnlyAssetMeta {
    messages: Vec<CosmosMsg<ProvenanceMsg>>,
}

impl AttributeOnlyAssetMeta {
    pub fn new() -> Self {
        Self { messages: vec![] }
    }
}

impl AssetMetaRepository for AttributeOnlyAssetMeta {
    fn has_asset<S1: Into<String>>(&self, deps: &DepsC, scope_address: S1) -> ContractResult<bool> {
        // check for asset attribute existence
        may_query_scope_attribute_by_scope_address(deps, scope_address)?
            .is_some()
            .to_ok()
    }

    fn add_asset<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        &mut self,
        deps: &DepsC,
        scope_address: S1,
        asset_type: S2,
        validator_address: S3,
        onboarding_status: AssetOnboardingStatus,
        validator_detail: ValidatorDetail,
    ) -> ContractResult<()> {
        let scope_address_str = scope_address.into();
        if self.has_asset(deps, scope_address_str.clone())? {
            return ContractError::AssetAlreadyOnboarded {
                scope_address: scope_address_str,
            }
            .to_result();
        }

        // generate attribute -> scope bind message
        let contract_base_name = config_read(deps.storage).load()?.base_contract_name;
        let attribute = AssetScopeAttribute::new(
            asset_type,
            Addr::unchecked("todo"),
            Addr::unchecked(validator_address),
            Some(onboarding_status),
            validator_detail,
        )?;
        self.add_message(get_add_attribute_to_scope_msg(
            scope_address_str,
            &attribute,
            contract_base_name,
        )?);
        Ok(())
    }

    fn get_asset<S1: Into<String>>(
        &self,
        _scope_address: S1,
    ) -> ContractResult<AssetScopeAttribute> {
        todo!()
        // try to fetch asset from attribute meta, if found also fetch scope attribute and reconstruct AssetMeta from relevant pieces
    }

    fn try_get_asset<S1: Into<String>>(&self, _scope_address: S1) -> Option<AssetScopeAttribute> {
        todo!()
        // try/catch get_asset and transform to option
    }

    fn validate_asset<S1: Into<String>>(
        &self,
        _scope_address: S1,
        _validation_result: bool,
    ) -> ContractResult<()> {
        todo!()
        // set validation result on asset (add messages to message service)
    }
}

impl MessageGatheringService for AttributeOnlyAssetMeta {
    fn get_messages(&self) -> Vec<CosmosMsg<ProvenanceMsg>> {
        self.messages.clone()
    }

    fn add_message(&mut self, message: CosmosMsg<ProvenanceMsg>) {
        self.messages.push(message)
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::QuerierWrapper;
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::{
            asset::AssetOnboardingStatus,
            state::{asset_meta, AssetMeta},
        },
        testutil::{
            onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset},
            test_utilities::{
                get_default_asset_definition, get_default_validator_detail, setup_test_suite,
                InstArgs, DEFAULT_ASSET_TYPE, DEFAULT_SCOPE_ADDRESS, DEFAULT_VALIDATOR_ADDRESS,
            },
        },
        util::asset_meta_repository::AssetMetaRepository,
    };

    use super::AttributeOnlyAssetMeta;

    #[test]
    fn has_asset_returns_false_if_asset_does_not_have_the_attribute() {
        let mut deps = mock_dependencies(&[]);
        let repository = setup_test_suite(&mut deps, InstArgs::default());

        let result = repository
            .has_asset(&deps.as_ref(), DEFAULT_SCOPE_ADDRESS)
            .unwrap();

        assert_eq!(
            false, result,
            "Repository should return false when asset does not have attribute"
        );
    }

    #[test]
    fn has_asset_returns_true_if_asset_has_attribute() {
        let mut deps = mock_dependencies(&[]);
        let mut repository = setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, &mut repository, TestOnboardAsset::default());

        let result = repository
            .has_asset(&deps.as_ref(), DEFAULT_SCOPE_ADDRESS)
            .unwrap();

        assert_eq!(
            true, result,
            "Repository should return true when asset does have attribute"
        );
    }

    #[test]
    fn add_asset_fails_if_asset_already_exists() {
        let mut deps = mock_dependencies(&[]);
        let mut repository = setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, &mut repository, TestOnboardAsset::default());

        let err = repository
            .add_asset(
                &deps.as_ref(),
                DEFAULT_SCOPE_ADDRESS,
                DEFAULT_ASSET_TYPE,
                DEFAULT_VALIDATOR_ADDRESS,
                AssetOnboardingStatus::Pending,
                get_default_validator_detail(),
            )
            .unwrap_err();

        match err {
            crate::core::error::ContractError::AssetAlreadyOnboarded { scope_address } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS.to_string(),
                    scope_address,
                    "Scope address should be reflected in AssetAlreadyOnboarded error"
                )
            }
            _ => panic!("Received unknown error when onboarding already-onboarded asset"),
        }
    }

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
