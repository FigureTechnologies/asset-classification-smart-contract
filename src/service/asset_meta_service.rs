use std::collections::HashSet;

use cosmwasm_std::CosmosMsg;
use provwasm_std::{delete_attributes, ProvenanceMsg};

use crate::{
    core::{
        asset::{
            AccessDefinition, AccessDefinitionType, AssetOnboardingStatus, AssetScopeAttribute,
            AssetVerificationResult,
        },
        error::ContractError,
        state::config_read,
    },
    query::query_asset_scope_attribute::{
        may_query_scope_attribute_by_scope_address, query_scope_attribute_by_scope_address,
    },
    util::aliases::{AssetResult, DepsMutC},
    util::deps_container::DepsContainer,
    util::traits::ResultExtensions,
    util::vec_container::VecContainer,
    util::{fees::calculate_verifier_cost_messages, functions::generate_asset_attribute_name},
    util::{
        provenance_util::get_add_attribute_to_scope_msg, scope_address_utils::bech32_string_to_addr,
    },
};

use super::{
    asset_meta_repository::AssetMetaRepository, deps_manager::DepsManager,
    message_gathering_service::MessageGatheringService,
};

pub struct AssetMetaService<'a> {
    deps: DepsContainer<'a>,
    messages: VecContainer<CosmosMsg<ProvenanceMsg>>,
}
impl<'a> AssetMetaService<'a> {
    pub fn new(deps: DepsMutC<'a>) -> Self {
        Self {
            deps: DepsContainer::new(deps),
            messages: VecContainer::new(),
        }
    }
}
impl<'a> AssetMetaRepository for AssetMetaService<'a> {
    fn has_asset<S1: Into<String>>(&self, scope_address: S1) -> AssetResult<bool> {
        let scope_address_string: String = scope_address.into();
        // check for asset attribute existence
        self.use_deps(|d| {
            may_query_scope_attribute_by_scope_address(&d.as_ref(), &scope_address_string)
        })?
        .is_some()
        .to_ok()
    }

    fn onboard_asset(&self, attribute: &AssetScopeAttribute, is_retry: bool) -> AssetResult<()> {
        // Verify that the attribute does or does not exist.  This check verifies that the value equivalent to is_retry:
        // If the asset exists, this should be a retry, because a subsequent onboard should only occur for that purpose
        // If the asset does not exist, this should not be a retry, because this is the first time the attribute is being attempted
        if self.has_asset(&attribute.scope_address)? != is_retry {
            return if is_retry {
                ContractError::generic(format!("unexpected state! asset scope [{}] was processed as new onboard, but the scope was not populated with asset classification data", &attribute.scope_address))
            } else {
                ContractError::AssetAlreadyOnboarded {
                    scope_address: attribute.scope_address.clone(),
                }
            }.to_err();
        }

        // generate attribute -> scope bind messages
        // On a retry, update the existing attribute with the given values
        if is_retry {
            self.update_attribute(attribute)?;
        } else {
            // On a first time execution, simply add the attribute to the scope - it's already been
            // verified that the attribute does not yet exist
            let contract_base_name = self
                .use_deps(|d| config_read(d.storage).load())?
                .base_contract_name;
            self.add_message(get_add_attribute_to_scope_msg(
                attribute,
                contract_base_name,
            )?);
        }
        Ok(())
    }

    fn update_attribute(&self, attribute: &AssetScopeAttribute) -> AssetResult<()> {
        let contract_base_name = self
            .use_deps(|d| config_read(d.storage).load())?
            .base_contract_name;
        let attribute_name =
            generate_asset_attribute_name(&attribute.asset_type, &contract_base_name);
        self.add_message(delete_attributes(
            bech32_string_to_addr(&attribute.scope_address)?,
            &attribute_name,
        )?);
        self.add_message(get_add_attribute_to_scope_msg(
            attribute,
            &contract_base_name,
        )?);
        Ok(())
    }

    fn get_asset<S1: Into<String>>(&self, scope_address: S1) -> AssetResult<AssetScopeAttribute> {
        let scope_address_string: String = scope_address.into();
        // try to fetch asset from attribute meta, if found also fetch scope attribute and reconstruct AssetMeta from relevant pieces
        self.use_deps(|d| {
            query_scope_attribute_by_scope_address(&d.as_ref(), &scope_address_string)
        })
    }

    fn try_get_asset<S1: Into<String>>(
        &self,
        scope_address: S1,
    ) -> AssetResult<Option<AssetScopeAttribute>> {
        let scope_address_string: String = scope_address.into();
        self.use_deps(|d| {
            may_query_scope_attribute_by_scope_address(&d.as_ref(), &scope_address_string)
        })
    }

    fn verify_asset<S1: Into<String>, S2: Into<String>>(
        &self,
        scope_address: S1,
        success: bool,
        verification_message: Option<S2>,
        access_routes: Vec<String>,
    ) -> AssetResult<()> {
        // set verification result on asset (add messages to message service)
        let scope_address_str = scope_address.into();
        let mut attribute = self.get_asset(scope_address_str.clone())?;
        let message = verification_message.map(|m| m.into()).unwrap_or_else(|| {
            match success {
                true => "verification successful",
                false => "verification failure",
            }
            .to_string()
        });
        if let Some(verifier_detail) = attribute.latest_verifier_detail {
            attribute.latest_verifier_detail = None;
            attribute.latest_verification_result =
                Some(AssetVerificationResult { message, success });

            // change the onboarding status based on how the verifier specified the success status
            attribute.onboarding_status = if success {
                AssetOnboardingStatus::Approved
            } else {
                AssetOnboardingStatus::Denied
            };

            let verifier_address = verifier_detail.address.clone();

            let filtered_access_routes = access_routes
                .into_iter()
                .map(|r| r.trim().to_owned())
                .filter(|r| !r.is_empty())
                .collect::<Vec<String>>();

            // check for existing verifier-linked access route collection
            if let Some(access_definition) = attribute
                .access_definitions
                .iter()
                .find(|ar| ar.owner_address == verifier_address)
            {
                let mut distinct_routes = [
                    &access_definition.access_routes[..],
                    &filtered_access_routes[..],
                ]
                .concat()
                .iter()
                .collect::<HashSet<_>>()
                .into_iter()
                .cloned()
                .collect::<Vec<String>>();
                distinct_routes.sort();

                let mut new_access_definitions = attribute
                    .access_definitions
                    .iter()
                    .filter(|ar| ar.owner_address != verifier_address)
                    .cloned()
                    .collect::<Vec<AccessDefinition>>();

                new_access_definitions.push(AccessDefinition {
                    access_routes: distinct_routes,
                    ..access_definition.to_owned()
                });

                attribute.access_definitions = new_access_definitions;
            } else if !filtered_access_routes.is_empty() {
                attribute.access_definitions.push(AccessDefinition {
                    owner_address: verifier_address,
                    access_routes: filtered_access_routes,
                    definition_type: AccessDefinitionType::Verifier,
                });
            }

            // Remove the old scope attribute and append a new one that overwrites existing data
            // with the changes made to the attribute
            self.update_attribute(&attribute)?;

            // distribute fees now that verification has happened
            self.append_messages(&calculate_verifier_cost_messages(&verifier_detail)?);
        } else {
            return ContractError::UnexpectedState {
                explanation: format!(
                    "Verifier detail not present on asset [{}] being verified",
                    scope_address_str
                ),
            }
            .to_err();
        }

        Ok(())
    }
}
impl<'a> DepsManager<'a> for AssetMetaService<'a> {
    fn use_deps<T, F>(&self, deps_fn: F) -> T
    where
        F: FnMut(&mut DepsMutC) -> T,
    {
        self.deps.use_deps(deps_fn)
    }

    fn into_deps(self) -> DepsMutC<'a> {
        self.deps.get()
    }
}
impl<'a> MessageGatheringService for AssetMetaService<'a> {
    fn get_messages(&self) -> Vec<CosmosMsg<ProvenanceMsg>> {
        self.messages.get_cloned()
    }

    fn add_message(&self, message: CosmosMsg<ProvenanceMsg>) {
        self.messages.push(message);
    }

    fn append_messages(&self, messages: &[CosmosMsg<ProvenanceMsg>]) {
        self.messages.append(&mut messages.to_vec());
    }

    fn clear_messages(&self) {
        self.messages.clear();
    }
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::{from_binary, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{
        AttributeMsgParams, AttributeValueType, ProvenanceMsg, ProvenanceMsgParams,
    };
    use serde_json_wasm::to_string;

    use crate::{
        core::{
            asset::{
                AccessDefinition, AccessDefinitionType, AssetIdentifier, AssetOnboardingStatus,
                AssetScopeAttribute, AssetVerificationResult, VerifierDetail,
            },
            error::ContractError,
            state::config_read,
        },
        execute::verify_asset::VerifyAssetV1,
        service::{
            asset_meta_repository::AssetMetaRepository, asset_meta_service::AssetMetaService,
            deps_manager::DepsManager, message_gathering_service::MessageGatheringService,
        },
        testutil::{
            onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset},
            test_constants::{
                DEFAULT_ACCESS_ROUTE, DEFAULT_ASSET_TYPE, DEFAULT_ASSET_UUID,
                DEFAULT_CONTRACT_BASE_NAME, DEFAULT_FEE_PERCENT, DEFAULT_ONBOARDING_COST,
                DEFAULT_ONBOARDING_DENOM, DEFAULT_SCOPE_ADDRESS, DEFAULT_SENDER_ADDRESS,
                DEFAULT_VERIFIER_ADDRESS,
            },
            test_utilities::{
                get_default_asset_scope_attribute, get_default_verifier_detail, setup_test_suite,
                test_instantiate_success, InstArgs,
            },
            verify_asset_helpers::{test_verify_asset, TestVerifyAsset},
        },
        util::{functions::generate_asset_attribute_name, traits::OptionExtensions},
    };

    #[test]
    fn has_asset_returns_false_if_asset_does_not_have_the_attribute() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        let repository = AssetMetaService::new(deps.as_mut());
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

        let repository = AssetMetaService::new(deps.as_mut());

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

        let repository = AssetMetaService::new(deps.as_mut());

        let err = repository
            .onboard_asset(&get_default_test_attribute(), false)
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

        let repository = AssetMetaService::new(deps.as_mut());

        repository
            .onboard_asset(&get_default_test_attribute(), false)
            .unwrap();

        let messages = repository.get_messages();

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
        let repository = AssetMetaService::new(deps.as_mut());

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
        let repository = AssetMetaService::new(deps.as_mut());

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
        let repository = AssetMetaService::new(deps.as_mut());

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
        let repository = AssetMetaService::new(deps.as_mut());

        let result = repository
            .try_get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("asset result should return without error")
            .expect("encapsulated asset should be present in the Option");

        assert_eq!(
            get_default_asset_scope_attribute(),
            result,
            "try_get_asset should return attribute for an onboarded asset"
        );
    }

    #[test]
    fn verify_asset_returns_error_if_asset_not_onboarded() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        let repository = AssetMetaService::new(deps.as_mut());

        let err = repository
            .verify_asset::<&str, &str>(DEFAULT_SCOPE_ADDRESS, true, None, vec![])
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
                "Unexpected error type returned from verify_asset on non-existant asset {:?}",
                err
            ),
        }
    }

    #[test]
    fn verify_asset_generates_attribute_update_message_sequence_successful_verification_with_message(
    ) {
        test_verification_result("cool good job".to_some(), true);
    }

    #[test]
    fn verify_asset_generates_attribute_update_message_sequence_successful_verification_no_message()
    {
        test_verification_result(None, true);
    }

    #[test]
    fn verify_asset_generates_attribute_update_message_sequence_negative_verification_with_message()
    {
        test_verification_result("you suck".to_some(), false);
    }

    #[test]
    fn verify_asset_generates_attribute_update_message_sequence_negative_verification_no_message() {
        test_verification_result(None, false);
    }

    #[test]
    fn test_into_deps() {
        let mut mock_deps = mock_dependencies(&[]);
        test_instantiate_success(mock_deps.as_mut(), InstArgs::default());
        let service = AssetMetaService::new(mock_deps.as_mut());
        let deps = service.into_deps();
        config_read(deps.storage)
            .load()
            .expect("expected storage to load from relinquished deps");
    }

    #[test]
    fn test_existing_verifier_detail_access_routes_merged() {
        let mut deps = mock_dependencies(&[]);
        // set up existing attribute with pre-existing access routes
        deps.querier.with_attributes(
            DEFAULT_SCOPE_ADDRESS,
            &[(
                generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME)
                    .as_str(),
                to_string(&AssetScopeAttribute {
                    asset_uuid: DEFAULT_ASSET_UUID.to_string(),
                    scope_address: DEFAULT_SCOPE_ADDRESS.to_string(),
                    asset_type: DEFAULT_ASSET_TYPE.to_string(),
                    requestor_address: Addr::unchecked(DEFAULT_SENDER_ADDRESS),
                    verifier_address: Addr::unchecked(DEFAULT_VERIFIER_ADDRESS),
                    onboarding_status: AssetOnboardingStatus::Pending,
                    latest_verifier_detail: VerifierDetail {
                        address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                        onboarding_cost: Uint128::new(DEFAULT_ONBOARDING_COST),
                        onboarding_denom: DEFAULT_ONBOARDING_DENOM.to_string(),
                        fee_percent: Decimal::percent(DEFAULT_FEE_PERCENT),
                        fee_destinations: vec![],
                    }
                    .to_some(),
                    latest_verification_result: None,
                    access_definitions: vec![
                        AccessDefinition {
                            owner_address: DEFAULT_SENDER_ADDRESS.to_string(),
                            access_routes: vec!["ownerroute1".to_string()],
                            definition_type: AccessDefinitionType::Requestor,
                        },
                        AccessDefinition {
                            owner_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                            access_routes: vec!["existingroute".to_string()],
                            definition_type: AccessDefinitionType::Verifier,
                        },
                    ],
                })
                .unwrap()
                .as_str(),
                "json",
            )],
        );

        setup_test_suite(&mut deps, InstArgs::default());
        let repository = AssetMetaService::new(deps.as_mut());

        repository
            .verify_asset::<&str, &str>(
                DEFAULT_SCOPE_ADDRESS,
                true,
                "Great jaerb there Hamstar".to_some(),
                vec!["newroute".to_string()],
            )
            .unwrap();

        let messages = repository.messages.get();

        assert_eq!(3, messages.len(),
        "verify asset should produce 3 messages (attribute delete/update combo and fee distribution to default verifier w/ no additional fee destinations)");

        let second_message = &messages[1];
        match second_message {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::AddAttribute { value, .. }),
                ..
            }) => {
                let deserialized: AssetScopeAttribute = from_binary(value).unwrap();
                assert_eq!(
                    2,
                    deserialized.access_definitions.len(),
                    "Modified scope attribute should only have 2 access route groups listed"
                );
                assert_eq!(
                    &AccessDefinition {
                        owner_address: DEFAULT_SENDER_ADDRESS.to_string(),
                        access_routes: vec!["ownerroute1".to_string()],
                        definition_type: AccessDefinitionType::Requestor,
                    },
                    deserialized
                        .access_definitions
                        .iter()
                        .find(|r| r.owner_address == DEFAULT_SENDER_ADDRESS)
                        .unwrap(),
                    "sender access route should be unchanged after verifier routes updated"
                );
                assert_eq!(
                    &AccessDefinition {
                        owner_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                        access_routes: vec!["existingroute".to_string(), "newroute".to_string()],
                        definition_type: AccessDefinitionType::Verifier,
                    },
                    deserialized
                        .access_definitions
                        .iter()
                        .find(|r| r.owner_address == DEFAULT_VERIFIER_ADDRESS)
                        .unwrap(),
                    "sender access route should be unchanged after verifier routes updated"
                );
            }
            _ => panic!(
                "Unexpected second message type for verify_asset: {:?}",
                second_message
            ),
        }
    }

    #[test]
    fn test_verify_with_invalid_access_routes_filters_them_out() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        // Establish some access routes with blank strings to prove that they get filtered out in the verification process
        test_verify_asset(
            &mut deps,
            TestVerifyAsset {
                verify_asset: VerifyAssetV1 {
                    // All invalid (empty or whitespace-only strings) access routes should be filtered from output
                    access_routes: vec![
                        "   ".to_string(),
                        "       ".to_string(),
                        "".to_string(),
                        "real route".to_string(),
                    ],
                    ..TestVerifyAsset::default_verify_asset()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("the scope attribute should be fetched");
        let verifier_access_definitions = attribute
            .access_definitions
            .into_iter()
            .filter(|d| d.owner_address.as_str() == DEFAULT_VERIFIER_ADDRESS)
            .collect::<Vec<AccessDefinition>>();
        assert_eq!(
            1,
            verifier_access_definitions.len(),
            "there should only be one entry for verifier access definitions",
        );
        let verifier_definition = verifier_access_definitions.first().unwrap();
        assert_eq!(
            1,
            verifier_definition.access_routes.len(),
            "only one access definition route should be added because the empty strings should be filtered out of the result",
        );
        assert_eq!(
            "real route",
            verifier_definition.access_routes.first().unwrap(),
            "the only route in the verifier's access definition should be the non-blank string provided",
        );
    }

    #[test]
    fn test_verify_with_only_invalid_access_routes_adds_no_access_definition_for_the_verifier() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        // Establish some access routes with blank strings to prove that they get filtered out in the verification process
        test_verify_asset(
            &mut deps,
            TestVerifyAsset {
                verify_asset: VerifyAssetV1 {
                    // Only invalid access routes should yield no access definition for the verifier
                    access_routes: vec!["   ".to_string(), "       ".to_string(), "".to_string()],
                    ..TestVerifyAsset::default_verify_asset()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("the scope attribute should be fetched");
        let verifier_access_definitions = attribute
            .access_definitions
            .into_iter()
            .filter(|d| d.owner_address.as_str() == DEFAULT_VERIFIER_ADDRESS)
            .collect::<Vec<AccessDefinition>>();
        assert!(
            verifier_access_definitions.is_empty(),
            "when no valid access routes for the verifier are provided, no access definition record should be added",
        );
    }

    #[test]
    fn test_verify_with_no_access_routes_adds_no_access_definition_for_the_verifier() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        // Establish some access routes with blank strings to prove that they get filtered out in the verification process
        test_verify_asset(
            &mut deps,
            TestVerifyAsset {
                verify_asset: VerifyAssetV1 {
                    // Completely empty access routes should yield no access definition for the verifier
                    access_routes: vec![],
                    ..TestVerifyAsset::default_verify_asset()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("the scope attribute should be fetched");
        let verifier_access_definitions = attribute
            .access_definitions
            .into_iter()
            .filter(|d| d.owner_address.as_str() == DEFAULT_VERIFIER_ADDRESS)
            .collect::<Vec<AccessDefinition>>();
        assert!(
            verifier_access_definitions.is_empty(),
            "when no access routes for the verifier are provided, no access definition record should be added",
        );
    }

    fn test_verification_result(message: Option<&str>, result: bool) {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let repository = AssetMetaService::new(deps.as_mut());
        repository
            .verify_asset::<&str, &str>(DEFAULT_SCOPE_ADDRESS, result, message, vec![])
            .unwrap();

        let messages = repository.get_messages();

        assert_eq!(
            3,
            messages.len(),
            "verify asset should produce three messages (attribute delete/update combo and fee distribution to default verifier w/ no additional fee destinations)"
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
                "Unexpected first message type for verify_asset: {:?}",
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
                value.latest_verifier_detail = None;
                value.latest_verification_result = AssetVerificationResult {
                    message: message
                        .unwrap_or_else(|| match result {
                            true => "verification successful",
                            false => "verification failure",
                        })
                        .to_string(),
                    success: result,
                }
                .to_some();
                // The onboarding status is based on whether or not the verifier approved the asset
                // Dynamically swap between expected statuses based on the input
                value.onboarding_status = if result {
                    AssetOnboardingStatus::Approved
                } else {
                    AssetOnboardingStatus::Denied
                };
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
                "Unexpected second message type for verify_asset: {:?}",
                second_message
            ),
        }
        let third_message = &messages[2];
        match third_message {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    DEFAULT_VERIFIER_ADDRESS, to_address,
                    "verification fee message should send to default verifier"
                );
                assert_eq!(
                    &vec![Coin {
                        denom: DEFAULT_ONBOARDING_DENOM.to_string(),
                        amount: Uint128::new(DEFAULT_ONBOARDING_COST)
                    }],
                    amount,
                    "verification fee message should match what is configured"
                )
            }
            _ => panic!(
                "Unexpected third message type for verify_asset: {:?}",
                third_message
            ),
        }
    }

    fn get_default_test_attribute() -> AssetScopeAttribute {
        AssetScopeAttribute::new(
            &AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
            DEFAULT_ASSET_TYPE,
            DEFAULT_SENDER_ADDRESS,
            DEFAULT_VERIFIER_ADDRESS,
            AssetOnboardingStatus::Pending.to_some(),
            get_default_verifier_detail(),
            vec![DEFAULT_ACCESS_ROUTE.to_string()],
        )
        .expect("failed to instantiate default asset scope attribute")
    }
}
