use std::vec;

use cosmwasm_std::{Addr, CosmosMsg};
use provwasm_std::{delete_attributes, ProvenanceMsg};

use crate::{
    core::{
        asset::{
            AssetOnboardingStatus, AssetScopeAttribute, AssetValidationResult, ValidatorDetail,
        },
        error::ContractError,
        msg::AssetIdentifier,
        state::config_read,
    },
    query::query_asset_scope_attribute::{
        may_query_scope_attribute_by_scope_address, query_scope_attribute_by_scope_address,
    },
    util::functions::generate_asset_attribute_name,
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
        identifier: &AssetIdentifier,
        asset_type: S1,
        validator_address: S2,
        requestor_address: S3,
        onboarding_status: AssetOnboardingStatus,
        validator_detail: ValidatorDetail,
    ) -> ContractResult<()>;

    fn get_asset<S1: Into<String>>(
        &self,
        deps: &DepsC,
        scope_address: S1,
    ) -> ContractResult<AssetScopeAttribute>;

    fn try_get_asset<S1: Into<String>>(
        &self,
        deps: &DepsC,
        scope_address: S1,
    ) -> ContractResult<Option<AssetScopeAttribute>>;

    fn validate_asset<S1: Into<String>, S2: Into<String>>(
        &mut self,
        deps: &DepsC,
        scope_address: S1,
        success: bool,
        validation_message: Option<S2>,
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
impl Default for AttributeOnlyAssetMeta {
    fn default() -> Self {
        Self::new()
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
        identifier: &AssetIdentifier,
        asset_type: S1,
        validator_address: S2,
        requestor_address: S3,
        onboarding_status: AssetOnboardingStatus,
        validator_detail: ValidatorDetail,
    ) -> ContractResult<()> {
        // generate attribute -> scope bind message
        let contract_base_name = config_read(deps.storage).load()?.base_contract_name;
        let attribute = AssetScopeAttribute::new(
            identifier,
            asset_type,
            requestor_address,
            validator_address,
            Some(onboarding_status),
            validator_detail,
        )?;

        if self.has_asset(deps, &attribute.scope_address)? {
            return ContractError::AssetAlreadyOnboarded {
                scope_address: attribute.scope_address,
            }
            .to_err();
        }
        self.add_message(get_add_attribute_to_scope_msg(
            &attribute,
            contract_base_name,
        )?);
        Ok(())
    }

    fn get_asset<S1: Into<String>>(
        &self,
        deps: &DepsC,
        scope_address: S1,
    ) -> ContractResult<AssetScopeAttribute> {
        // try to fetch asset from attribute meta, if found also fetch scope attribute and reconstruct AssetMeta from relevant pieces
        query_scope_attribute_by_scope_address(deps, scope_address)
    }

    fn try_get_asset<S1: Into<String>>(
        &self,
        deps: &DepsC,
        scope_address: S1,
    ) -> ContractResult<Option<AssetScopeAttribute>> {
        may_query_scope_attribute_by_scope_address(deps, scope_address)
    }

    fn validate_asset<S1: Into<String>, S2: Into<String>>(
        &mut self,
        deps: &DepsC,
        scope_address: S1,
        success: bool,
        validation_message: Option<S2>,
    ) -> ContractResult<()> {
        // set validation result on asset (add messages to message service)
        let scope_address_str = scope_address.into();
        let mut attribute = self.get_asset(deps, scope_address_str.clone())?;

        let contract_base_name = config_read(deps.storage).load()?.base_contract_name;

        let attribute_name =
            generate_asset_attribute_name(attribute.asset_type.clone(), contract_base_name.clone());
        self.messages.push(delete_attributes(
            Addr::unchecked(scope_address_str),
            attribute_name,
        )?);

        let message = validation_message.map(|m| m.into()).unwrap_or_else(|| {
            match success {
                true => "validation successful",
                false => "validation failure",
            }
            .to_string()
        });

        attribute.latest_validator_detail = None;
        attribute.latest_validation_result = Some(AssetValidationResult { message, success });
        self.messages.push(get_add_attribute_to_scope_msg(
            &attribute,
            contract_base_name,
        )?);
        Ok(())
    }
}

impl MessageGatheringService for AttributeOnlyAssetMeta {
    fn get_messages(&self) -> Vec<CosmosMsg<ProvenanceMsg>> {
        self.messages.clone()
    }

    fn add_message(&mut self, message: CosmosMsg<ProvenanceMsg>) {
        self.messages.push(message)
    }

    fn drain_messages(&mut self) {
        self.messages.drain(..);
    }
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::{from_binary, to_binary, Addr, CosmosMsg};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{
        AttributeMsgParams, AttributeValueType, ProvenanceMsg, ProvenanceMsgParams,
    };

    use crate::{
        core::{
            asset::{AssetOnboardingStatus, AssetScopeAttribute, AssetValidationResult},
            error::ContractError,
            msg::AssetIdentifier,
        },
        testutil::{
            onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset},
            test_utilities::{
                get_default_asset_scope_attribute, get_default_validator_detail, setup_test_suite,
                InstArgs, DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME, DEFAULT_SCOPE_ADDRESS,
                DEFAULT_SENDER_ADDRESS, DEFAULT_VALIDATOR_ADDRESS,
            },
        },
        util::{
            asset_meta_repository::AssetMetaRepository, functions::generate_asset_attribute_name,
            message_gathering_service::MessageGatheringService, traits::OptionExtensions,
        },
    };

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
        test_onboard_asset(&mut deps, &mut repository, TestOnboardAsset::default()).unwrap();

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
        test_onboard_asset(&mut deps, &mut repository, TestOnboardAsset::default()).unwrap();

        let err = repository
            .add_asset(
                &deps.as_ref(),
                &AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                DEFAULT_ASSET_TYPE,
                DEFAULT_VALIDATOR_ADDRESS,
                DEFAULT_SENDER_ADDRESS,
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
    fn add_asset_generates_proper_attribute_message() {
        let mut deps = mock_dependencies(&[]);
        let mut repository = setup_test_suite(&mut deps, InstArgs::default());

        repository
            .add_asset(
                &deps.as_ref(),
                &AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                DEFAULT_ASSET_TYPE,
                DEFAULT_VALIDATOR_ADDRESS,
                DEFAULT_SENDER_ADDRESS,
                AssetOnboardingStatus::Pending,
                get_default_validator_detail(),
            )
            .unwrap();

        assert_eq!(
            1,
            repository.get_messages().len(),
            "add_asset should only generate one message"
        );
        match repository.get_messages().first() {
            Some(CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::AddAttribute {
                        name,
                        value,
                        value_type,
                        ..
                    }),
                ..
            })) => {
                assert_eq!(
                    generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME),
                    name.to_owned(),
                    "attribute name should match what is expected"
                );
                let deserialized: AssetScopeAttribute = from_binary(value).unwrap();
                let expected = get_default_asset_scope_attribute();
                assert_eq!(
                    expected, deserialized,
                    "attribute should contain proper values"
                );
                assert_eq!(
                    AttributeValueType::Json,
                    value_type.to_owned(),
                    "generated attribute value_type should be Json"
                );
            }
            _ => panic!("Unexpected message type resultig from add_asset"),
        }
    }

    #[test]
    fn get_asset_returns_error_if_asset_does_not_exist() {
        let mut deps = mock_dependencies(&[]);
        let repository = setup_test_suite(&mut deps, InstArgs::default());

        let err = repository
            .get_asset(&deps.as_ref(), DEFAULT_SCOPE_ADDRESS)
            .unwrap_err();

        match err {
            ContractError::NotFound { explanation } => assert_eq!(
                format!(
                    "scope at address [{}] did not include an asset scope attribute",
                    DEFAULT_SCOPE_ADDRESS
                ),
                explanation
            ),
            err => panic!(
                "Unexpected error type returned from get_asset on non-existant asset {:?}",
                err
            ),
        }
    }

    #[test]
    fn get_asset_returns_asset_if_exists() {
        let mut deps = mock_dependencies(&[]);
        let mut repository = setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, &mut repository, TestOnboardAsset::default()).unwrap();

        let attribute = repository
            .get_asset(&deps.as_ref(), DEFAULT_SCOPE_ADDRESS)
            .unwrap();

        assert_eq!(
            get_default_asset_scope_attribute(),
            attribute,
            "Attribute returned from get_asset should match what is expected"
        );
    }

    #[test]
    fn try_get_asset_returns_none_if_asset_does_not_exist() {
        let mut deps = mock_dependencies(&[]);
        let repository = setup_test_suite(&mut deps, InstArgs::default());

        let result = repository
            .try_get_asset(&deps.as_ref(), DEFAULT_SCOPE_ADDRESS)
            .unwrap();

        assert_eq!(
            None, result,
            "try_get_asset should return None for a non-onboarded asset"
        );
    }

    #[test]
    fn try_get_asset_returns_asset_if_exists() {
        let mut deps = mock_dependencies(&[]);
        let mut repository = setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, &mut repository, TestOnboardAsset::default()).unwrap();

        let result = repository
            .try_get_asset(&deps.as_ref(), DEFAULT_SCOPE_ADDRESS)
            .unwrap();

        assert_eq!(
            Some(get_default_asset_scope_attribute()),
            result,
            "try_get_asset should return attribute for an onboarded asset"
        );
    }

    #[test]
    fn validate_asset_returns_error_if_asset_not_onboarded() {
        let mut deps = mock_dependencies(&[]);
        let mut repository = setup_test_suite(&mut deps, InstArgs::default());

        let err = repository
            .validate_asset::<&str, &str>(&mut deps.as_ref(), DEFAULT_SCOPE_ADDRESS, true, None)
            .unwrap_err();

        match err {
            ContractError::NotFound { explanation } => assert_eq!(
                explanation,
                format!(
                    "scope at address [{}] did not include an asset scope attribute",
                    DEFAULT_SCOPE_ADDRESS
                )
            ),
            err => panic!(
                "Unexpected error type returned from validate_asset on non-existant asset {:?}",
                err
            ),
        }
    }

    #[test]
    fn validate_asset_generates_attribute_update_message_sequence_successful_validation_with_message(
    ) {
        test_validation_result("cool good job".to_option(), true);
    }

    #[test]
    fn validate_asset_generates_attribute_update_message_sequence_successful_validation_no_message()
    {
        test_validation_result(None, true);
    }

    #[test]
    fn validate_asset_generates_attribute_update_message_sequence_negative_validation_with_message()
    {
        test_validation_result("you suck".to_option(), false);
    }

    #[test]
    fn validate_asset_generates_attribute_update_message_sequence_negative_validation_no_message() {
        test_validation_result(None, false);
    }

    fn test_validation_result(message: Option<&str>, result: bool) {
        let mut deps = mock_dependencies(&[]);
        let mut repository = setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, &mut repository, TestOnboardAsset::default()).unwrap();
        repository.messages.drain(..); // remove onboarding messages

        repository
            .validate_asset::<&str, &str>(
                &mut deps.as_ref(),
                DEFAULT_SCOPE_ADDRESS,
                result,
                message,
            )
            .unwrap();

        assert_eq!(
            2,
            repository.get_messages().len(),
            "validate asset should produce 2 messages for scope update (delete/write combination)"
        );
        match &repository.get_messages()[0] {
            CosmosMsg::Custom(ProvenanceMsg {
                params: ProvenanceMsgParams::Attribute(msg),
                ..
            }) => {
                assert_eq!(
                    AttributeMsgParams::DeleteAttribute {
                        address: Addr::unchecked(DEFAULT_SCOPE_ADDRESS),
                        name: generate_asset_attribute_name(
                            DEFAULT_ASSET_TYPE,
                            DEFAULT_CONTRACT_BASE_NAME
                        )
                    },
                    msg.to_owned(),
                    "delete attribute message should match what is expected"
                );
            }
            _ => panic!("Unexpected first message type for validate_asset"),
        }
        match &repository.get_messages()[1] {
            CosmosMsg::Custom(ProvenanceMsg {
                params: ProvenanceMsgParams::Attribute(msg),
                ..
            }) => {
                let mut value = get_default_asset_scope_attribute();
                value.latest_validator_detail = None;
                value.latest_validation_result = Some(AssetValidationResult {
                    message: message
                        .unwrap_or_else(|| match result {
                            true => "validation successful",
                            false => "validation failure",
                        })
                        .to_string(),
                    success: result,
                });
                assert_eq!(
                    AttributeMsgParams::AddAttribute {
                        address: Addr::unchecked(DEFAULT_SCOPE_ADDRESS),
                        name: generate_asset_attribute_name(
                            DEFAULT_ASSET_TYPE,
                            DEFAULT_CONTRACT_BASE_NAME
                        ),
                        value: to_binary(&value).unwrap(),
                        value_type: AttributeValueType::Json
                    },
                    msg.to_owned(),
                    "add attribute message should match what is expected"
                );
            }
            _ => panic!("Unexpected second message type for validate_asset"),
        }
    }
}
