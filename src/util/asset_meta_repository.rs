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
    aliases::{ContractResult, DepsMutC},
    deps_container::DepsContainer,
    provenance_util::get_add_attribute_to_scope_msg,
    traits::ResultExtensions,
    vec_container::VecContainer,
};

pub struct AssetMetaRepository<'a> {
    pub deps: DepsContainer<'a>,
    pub messages: VecContainer<CosmosMsg<ProvenanceMsg>>,
}
impl<'a> AssetMetaRepository<'a> {
    pub fn new(deps: &'a mut DepsMutC<'a>) -> Self {
        Self {
            deps: DepsContainer::new(deps),
            messages: VecContainer::new(),
        }
    }

    pub fn has_asset<S1: Into<String>>(&self, scope_address: S1) -> ContractResult<bool> {
        // check for asset attribute existence
        self.deps
            .use_deps(|d| may_query_scope_attribute_by_scope_address(&d.as_ref(), scope_address))?
            .is_some()
            .to_ok()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_asset<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        &self,
        identifier: &AssetIdentifier,
        asset_type: S1,
        validator_address: S2,
        requestor_address: S3,
        onboarding_status: AssetOnboardingStatus,
        validator_detail: ValidatorDetail,
    ) -> ContractResult<()> {
        // generate attribute -> scope bind message
        let contract_base_name = self
            .deps
            .use_deps(|d| config_read(d.storage))
            .load()?
            .base_contract_name;
        let attribute = AssetScopeAttribute::new(
            identifier,
            asset_type,
            requestor_address,
            validator_address,
            Some(onboarding_status),
            validator_detail,
        )?;

        if self.has_asset(&attribute.scope_address)? {
            return ContractError::AssetAlreadyOnboarded {
                scope_address: attribute.scope_address,
            }
            .to_err();
        }
        self.messages.push(get_add_attribute_to_scope_msg(
            &attribute,
            contract_base_name,
        )?);
        Ok(())
    }

    pub fn get_asset<S1: Into<String>>(
        &self,
        scope_address: S1,
    ) -> ContractResult<AssetScopeAttribute> {
        let scope_address_string: String = scope_address.into();
        // try to fetch asset from attribute meta, if found also fetch scope attribute and reconstruct AssetMeta from relevant pieces
        self.deps.use_deps(|d| {
            query_scope_attribute_by_scope_address(&d.as_ref(), &scope_address_string)
        })
    }

    pub fn try_get_asset<S1: Into<String>>(
        &self,
        scope_address: S1,
    ) -> ContractResult<Option<AssetScopeAttribute>> {
        let scope_address_string: String = scope_address.into();
        self.deps.use_deps(|d| {
            may_query_scope_attribute_by_scope_address(&d.as_ref(), &scope_address_string)
        })
    }

    pub fn validate_asset<S1: Into<String>, S2: Into<String>>(
        &self,
        scope_address: S1,
        success: bool,
        validation_message: Option<S2>,
    ) -> ContractResult<()> {
        // set validation result on asset (add messages to message service)
        let scope_address_str = scope_address.into();
        let mut attribute = self.get_asset(scope_address_str.clone())?;
        let contract_base_name = self
            .deps
            .use_deps(|d| config_read(d.storage))
            .load()?
            .base_contract_name;
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
            test_constants::{
                DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME, DEFAULT_SCOPE_ADDRESS,
                DEFAULT_SENDER_ADDRESS, DEFAULT_VALIDATOR_ADDRESS,
            },
            test_utilities::{
                get_default_asset_scope_attribute, get_default_validator_detail, setup_test_suite,
                InstArgs,
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
        setup_test_suite(&mut deps, InstArgs::default());
        let repository = AssetMetaRepository::new(&mut deps.as_mut());
        let result = repository.has_asset(DEFAULT_SCOPE_ADDRESS).unwrap();
        assert_eq!(
            false, result,
            "Repository should return false when asset does not have attribute"
        );
    }

    #[test]
    fn has_asset_returns_true_if_asset_has_attribute() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();

        let repository = AssetMetaRepository::new(&mut deps.as_mut());

        let result = repository.has_asset(DEFAULT_SCOPE_ADDRESS).unwrap();

        assert_eq!(
            true, result,
            "Repository should return true when asset does have attribute"
        );
    }

    #[test]
    fn add_asset_fails_if_asset_already_exists() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();

        let repository = AssetMetaRepository::new(&mut deps.as_mut());

        let err = repository
            .add_asset(
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
            _ => panic!(
                "Received unknown error when onboarding already-onboarded asset: {:?}",
                err
            ),
        }
    }

    #[test]
    fn add_asset_generates_proper_attribute_message() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let repository = AssetMetaRepository::new(&mut deps.as_mut());

        repository
            .add_asset(
                &AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                DEFAULT_ASSET_TYPE,
                DEFAULT_VALIDATOR_ADDRESS,
                DEFAULT_SENDER_ADDRESS,
                AssetOnboardingStatus::Pending,
                get_default_validator_detail(),
            )
            .unwrap();

        let messages = repository.messages.get();

        assert_eq!(
            1,
            messages.len(),
            "add_asset should only generate one message"
        );
        let message = messages
            .first()
            .expect("expected a first message to be added")
            .to_owned();
        match message {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::AddAttribute {
                        name,
                        value,
                        value_type,
                        ..
                    }),
                ..
            }) => {
                assert_eq!(
                    generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME),
                    name.to_owned(),
                    "attribute name should match what is expected"
                );
                let deserialized: AssetScopeAttribute = from_binary(&value).unwrap();
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
            _ => panic!(
                "Unexpected message type resulting from add_asset: {:?}",
                message
            ),
        }
    }

    #[test]
    fn get_asset_returns_error_if_asset_does_not_exist() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        let repository = AssetMetaRepository::new(&mut deps.as_mut());

        let err = repository.get_asset(DEFAULT_SCOPE_ADDRESS).unwrap_err();

        match err {
            ContractError::NotFound { explanation } => assert_eq!(
                format!(
                    "scope at address [{}] did not include an asset scope attribute",
                    DEFAULT_SCOPE_ADDRESS
                ),
                explanation
            ),
            _ => panic!(
                "Unexpected error type returned from get_asset on non-existant asset {:?}",
                err
            ),
        }
    }

    #[test]
    fn get_asset_returns_asset_if_exists() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let repository = AssetMetaRepository::new(&mut deps.as_mut());

        let attribute = repository.get_asset(DEFAULT_SCOPE_ADDRESS).unwrap();

        assert_eq!(
            get_default_asset_scope_attribute(),
            attribute,
            "Attribute returned from get_asset should match what is expected"
        );
    }

    #[test]
    fn try_get_asset_returns_none_if_asset_does_not_exist() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        let repository = AssetMetaRepository::new(&mut deps.as_mut());

        let result = repository.try_get_asset(DEFAULT_SCOPE_ADDRESS).unwrap();

        assert_eq!(
            None, result,
            "try_get_asset should return None for a non-onboarded asset"
        );
    }

    #[test]
    fn try_get_asset_returns_asset_if_exists() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let repository = AssetMetaRepository::new(&mut deps.as_mut());

        let result = repository.try_get_asset(DEFAULT_SCOPE_ADDRESS).unwrap();

        assert_eq!(
            Some(get_default_asset_scope_attribute()),
            result,
            "try_get_asset should return attribute for an onboarded asset"
        );
    }

    #[test]
    fn validate_asset_returns_error_if_asset_not_onboarded() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        let repository = AssetMetaRepository::new(&mut deps.as_mut());

        let err = repository
            .validate_asset::<&str, &str>(DEFAULT_SCOPE_ADDRESS, true, None)
            .unwrap_err();

        match err {
            ContractError::NotFound { explanation } => assert_eq!(
                explanation,
                format!(
                    "scope at address [{}] did not include an asset scope attribute",
                    DEFAULT_SCOPE_ADDRESS
                )
            ),
            _ => panic!(
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
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let repository = AssetMetaRepository::new(&mut deps.as_mut());
        repository
            .validate_asset::<&str, &str>(DEFAULT_SCOPE_ADDRESS, result, message)
            .unwrap();

        let messages = repository.messages.get();

        assert_eq!(
            2,
            messages.len(),
            "validate asset should produce 2 messages for scope update (delete/write combination)"
        );
        let first_message = &messages[0];
        match first_message {
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
            _ => panic!(
                "Unexpected first message type for validate_asset: {:?}",
                first_message,
            ),
        }
        let second_message = &messages[1];
        match second_message {
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
            _ => panic!(
                "Unexpected second message type for validate_asset: ${:?}",
                second_message
            ),
        }
    }
}
