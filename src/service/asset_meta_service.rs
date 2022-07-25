use std::collections::HashSet;

use cosmwasm_std::{to_binary, CosmosMsg};
use provwasm_std::{update_attribute, AttributeValueType, ProvenanceMsg};

use crate::core::state::{delete_latest_verifier_detail, insert_latest_verifier_detail};
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::{
    core::{
        error::ContractError,
        state::config_read_v2,
        types::{
            access_definition::{AccessDefinition, AccessDefinitionType},
            access_route::AccessRoute,
            asset_onboarding_status::AssetOnboardingStatus,
            asset_scope_attribute::AssetScopeAttribute,
            asset_verification_result::AssetVerificationResult,
        },
    },
    query::query_asset_scope_attribute::{
        may_query_scope_attribute_by_scope_address, query_scope_attribute_by_scope_address,
    },
    util::aliases::{AssetResult, DepsMutC},
    util::deps_container::DepsContainer,
    util::vec_container::VecContainer,
    util::{fees::calculate_verifier_cost_messages, functions::generate_asset_attribute_name},
    util::{functions::filter_valid_access_routes, traits::ResultExtensions},
    util::{
        provenance_util::get_add_attribute_to_scope_msg, scope_address_utils::bech32_string_to_addr,
    },
};

use super::{
    asset_meta_repository::AssetMetaRepository, deps_manager::DepsManager,
    message_gathering_service::MessageGatheringService,
};

/// Ties all service code together into a cohesive struct to use for complex operations during the
/// onboarding and verification processes.
pub struct AssetMetaService<'a> {
    /// A wrapper for a [DepsMutC](core::util::aliases::DepsMutC] that allows access to it without
    /// moving the value.
    deps: DepsContainer<'a>,
    /// All messages generated over the course of function invocations.
    messages: VecContainer<CosmosMsg<ProvenanceMsg>>,
}
impl<'a> AssetMetaService<'a> {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `deps` The cosmwasm deps that will be moved into a [DepsContainer](crate::util::deps_container::DepsContainer)
    /// for future access.
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

    fn onboard_asset(
        &self,
        attribute: &AssetScopeAttribute,
        latest_verifier_detail: &VerifierDetailV2,
        is_retry: bool,
    ) -> AssetResult<()> {
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
                .use_deps(|d| config_read_v2(d.storage).load())?
                .base_contract_name;
            self.add_message(get_add_attribute_to_scope_msg(
                attribute,
                contract_base_name,
            )?);
        }

        // Store the latest verifier detail for use when verification occurs, ensuring that the
        // proper fees from when onboarding occurred are used
        self.use_deps(|deps| {
            insert_latest_verifier_detail(
                deps.storage,
                &attribute.scope_address,
                latest_verifier_detail,
            )
        })?;
        Ok(())
    }

    fn update_attribute(&self, updated_attribute: &AssetScopeAttribute) -> AssetResult<()> {
        let contract_base_name = self
            .use_deps(|d| config_read_v2(d.storage).load())?
            .base_contract_name;
        let original_attribute = self.get_asset(&updated_attribute.scope_address)?;
        self.add_message(update_attribute(
            // address: Target address - the scope with the attribute on it
            bech32_string_to_addr(&original_attribute.scope_address)?,
            // name: Attribute name - use the same value as before
            generate_asset_attribute_name(&original_attribute.asset_type, &contract_base_name),
            // original_value: The unmodified original attribute
            to_binary(&original_attribute)?,
            // original_value_type
            AttributeValueType::Json,
            // update_value: The attribute with changes
            to_binary(updated_attribute)?,
            // update_value_type: Maintain Json typing. it's awesome that this can change between updates,
            // but this code doesn't want that
            AttributeValueType::Json,
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
        access_routes: Vec<AccessRoute>,
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
        if let Some(verifier_detail) =
            self.use_deps(|deps| attribute.get_latest_verifier_detail(deps.storage))
        {
            attribute.latest_verification_result =
                Some(AssetVerificationResult { message, success });

            // change the onboarding status based on how the verifier specified the success status
            attribute.onboarding_status = if success {
                AssetOnboardingStatus::Approved
            } else {
                AssetOnboardingStatus::Denied
            };

            let verifier_address = verifier_detail.address.clone();

            let filtered_access_routes = filter_valid_access_routes(access_routes);

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
                .collect::<Vec<AccessRoute>>();
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

            // Remove the latest verifier detail from storage - it's only needed for discovering
            // fees, so its existence is no longer relevant after verification completes.
            self.use_deps(|deps| delete_latest_verifier_detail(deps.storage, &scope_address_str))?;
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
mod tests {
    use cosmwasm_std::{from_binary, to_binary, Addr, BankMsg, Coin, CosmosMsg, Uint128};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{
        AttributeMsgParams, AttributeValueType, ProvenanceMsg, ProvenanceMsgParams,
    };
    use serde_json_wasm::to_string;

    use crate::core::state::{
        delete_latest_verifier_detail, insert_latest_verifier_detail,
        latest_verifier_detail_store_ro,
    };
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::testutil::test_utilities::get_default_asset_scope_attribute_and_detail;
    use crate::{
        core::{
            error::ContractError,
            state::config_read_v2,
            types::{
                access_definition::{AccessDefinition, AccessDefinitionType},
                access_route::AccessRoute,
                asset_identifier::AssetIdentifier,
                asset_onboarding_status::AssetOnboardingStatus,
                asset_scope_attribute::AssetScopeAttribute,
                asset_verification_result::AssetVerificationResult,
            },
        },
        execute::verify_asset::VerifyAssetV1,
        service::{
            asset_meta_repository::AssetMetaRepository, asset_meta_service::AssetMetaService,
            deps_manager::DepsManager, message_gathering_service::MessageGatheringService,
        },
        testutil::{
            onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset},
            test_constants::{
                DEFAULT_ASSET_TYPE, DEFAULT_ASSET_UUID, DEFAULT_CONTRACT_BASE_NAME,
                DEFAULT_ONBOARDING_COST, DEFAULT_ONBOARDING_DENOM, DEFAULT_SCOPE_ADDRESS,
                DEFAULT_SENDER_ADDRESS, DEFAULT_VERIFIER_ADDRESS,
            },
            test_utilities::{
                assert_single_item, get_default_access_routes, get_default_asset_scope_attribute,
                get_default_entity_detail, get_default_verifier_detail, setup_test_suite,
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
            .onboard_asset(
                &get_default_test_attribute(),
                &get_default_verifier_detail(),
                false,
            )
            .unwrap_err();

        match err {
            ContractError::AssetAlreadyOnboarded { scope_address } => {
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

        let verifier_detail = get_default_verifier_detail();
        repository
            .onboard_asset(&get_default_test_attribute(), &verifier_detail, false)
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
        let latest_verifier_detail = latest_verifier_detail_store_ro(deps.as_ref().storage)
            .load(DEFAULT_SCOPE_ADDRESS.as_bytes())
            .expect("the verifier detail should be in storage after onboarding completes");
        assert_eq!(
            verifier_detail, latest_verifier_detail,
            "expected the value in storage to equate to the value passed into the onboard function",
        );
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
            get_default_asset_scope_attribute_and_detail(true),
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
            get_default_asset_scope_attribute_and_detail(true),
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
        config_read_v2(deps.storage)
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
                    latest_verifier_detail: None,
                    latest_verification_result: None,
                    access_definitions: vec![
                        AccessDefinition {
                            owner_address: DEFAULT_SENDER_ADDRESS.to_string(),
                            access_routes: vec![AccessRoute::route_only("ownerroute1")],
                            definition_type: AccessDefinitionType::Requestor,
                        },
                        AccessDefinition {
                            owner_address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                            access_routes: vec![AccessRoute::route_only("existingroute")],
                            definition_type: AccessDefinitionType::Verifier,
                        },
                    ],
                })
                .unwrap()
                .as_str(),
                "json",
            )],
        );

        insert_latest_verifier_detail(
            deps.as_mut().storage,
            DEFAULT_SCOPE_ADDRESS,
            &VerifierDetailV2 {
                address: DEFAULT_VERIFIER_ADDRESS.to_string(),
                onboarding_cost: Uint128::new(DEFAULT_ONBOARDING_COST),
                onboarding_denom: DEFAULT_ONBOARDING_DENOM.to_string(),
                fee_destinations: vec![],
                entity_detail: get_default_entity_detail().to_some(),
            },
        )
        .expect("expected the latest verifier detail to be properly stored");

        setup_test_suite(&mut deps, InstArgs::default());
        let repository = AssetMetaService::new(deps.as_mut());

        repository
            .verify_asset::<&str, &str>(
                DEFAULT_SCOPE_ADDRESS,
                true,
                "Great jaerb there Hamstar".to_some(),
                vec![AccessRoute::route_only("newroute")],
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
                        access_routes: vec![AccessRoute::route_only("ownerroute1")],
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
                        access_routes: vec![
                            AccessRoute::route_only("existingroute"),
                            AccessRoute::route_only("newroute")
                        ],
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
                        AccessRoute::route_only("   "),
                        AccessRoute::route_only("       "),
                        AccessRoute::route_only(""),
                        AccessRoute::route_only("real route"),
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
            verifier_definition.access_routes.first().unwrap().route,
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
                    access_routes: vec![
                        AccessRoute::route_only("   "),
                        AccessRoute::route_only("       "),
                        AccessRoute::route_only(""),
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
        assert!(
            verifier_access_definitions.is_empty(),
            "when no valid access routes for the verifier are provided, no access definition record should be added",
        );
    }

    #[test]
    fn test_verify_new_access_routes_are_trimmed() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        // Establish some access routes with blank strings to prove that they get filtered out in the verification process
        test_verify_asset(
            &mut deps,
            TestVerifyAsset {
                verify_asset: VerifyAssetV1 {
                    // Only invalid access routes should yield no access definition for the verifier
                    access_routes: vec![AccessRoute::route_and_name("route       ", "   name   ")],
                    ..TestVerifyAsset::default_verify_asset()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let attribute = AssetMetaService::new(deps.as_mut())
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("the scope attribute should be fetched");
        let access_routes = assert_single_item(
            &attribute
                .access_definitions
                .into_iter()
                .filter(|d| d.owner_address.as_str() == DEFAULT_VERIFIER_ADDRESS)
                .collect::<Vec<AccessDefinition>>(),
            "expected only a single access definition to be provided for the verifier",
        )
        .access_routes;
        let access_route = assert_single_item(
            &access_routes,
            "expected only a sigle access route to exist for the verifier",
        );
        assert_eq!(
            "route", access_route.route,
            "the route value should be trimmed of all whitespace",
        );
        assert_eq!(
            "name",
            access_route.name.expect("the name value should be set"),
            "the name value should be trimmed of all whitespice",
        );
    }

    #[test]
    fn test_verify_with_duplicate_access_routes_and_different_names_keeps_all_routes() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        // Use two AccessRoutes with the same route but different names
        test_verify_asset(
            &mut deps,
            TestVerifyAsset {
                verify_asset: VerifyAssetV1 {
                    // Only invalid access routes should yield no access definition for the verifier
                    access_routes: vec![
                        AccessRoute::route_and_name("test-route", "name1"),
                        AccessRoute::route_and_name("test-route", "name2"),
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
        let access_routes = assert_single_item(
            &attribute
                .access_definitions
                .into_iter()
                .filter(|d| d.owner_address.as_str() == DEFAULT_VERIFIER_ADDRESS)
                .collect::<Vec<AccessDefinition>>(),
            "expected only a single access definition to be provided for the verifier",
        )
        .access_routes;
        assert!(
            access_routes.iter().any(|r| r.route == "test-route"
                && r.to_owned().name.expect("all names should be Some") == "name1"),
            "the first name route should be included in the access routes",
        );
        assert!(
            access_routes.iter().any(|r| r.route == "test-route"
                && r.to_owned().name.expect("all names should be Some") == "name2"),
            "the second name route should be included in the access routes",
        );
    }

    #[test]
    fn test_verify_with_duplicate_routes_one_some_name_one_none_name() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        // Use two AccessRoutes with the same route but different names
        test_verify_asset(
            &mut deps,
            TestVerifyAsset {
                verify_asset: VerifyAssetV1 {
                    // Only invalid access routes should yield no access definition for the verifier
                    access_routes: vec![
                        AccessRoute::route_and_name("test-route", "name"),
                        AccessRoute::route_only("test-route"),
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
        let access_routes = assert_single_item(
            &attribute
                .access_definitions
                .into_iter()
                .filter(|d| d.owner_address.as_str() == DEFAULT_VERIFIER_ADDRESS)
                .collect::<Vec<AccessDefinition>>(),
            "expected only a single access definition to be provided for the verifier",
        )
        .access_routes;
        assert!(
            access_routes.iter().any(|r| r.route == "test-route"
                && r.to_owned()
                    .name
                    .unwrap_or("not the right name".to_string())
                    == "name"),
            "the named route should be kept",
        );
        assert!(
            access_routes
                .iter()
                .any(|r| r.route == "test-route" && r.name.is_none()),
            "the unnamed route should be kept",
        );
    }

    #[test]
    fn test_verify_skips_duplicates_after_trimming() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        // Use two AccessRoutes with the same route but different names
        test_verify_asset(
            &mut deps,
            TestVerifyAsset {
                verify_asset: VerifyAssetV1 {
                    // Only invalid access routes should yield no access definition for the verifier
                    access_routes: vec![
                        AccessRoute::route_and_name("test-route            ", "name       "),
                        AccessRoute::route_and_name("      test-route", "          name"),
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
        let access_routes = assert_single_item(
            &attribute
                .access_definitions
                .into_iter()
                .filter(|d| d.owner_address.as_str() == DEFAULT_VERIFIER_ADDRESS)
                .collect::<Vec<AccessDefinition>>(),
            "expected only a single access definition to be provided for the verifier",
        )
        .access_routes;
        let access_route = assert_single_item(
            &access_routes,
            format!("only a single access route should remain due to them being duplicates after trimming, but found {access_routes:?}"),
        );
        assert_eq!(
            "test-route", access_route.route,
            "the access route should have the trimmed route",
        );
        assert_eq!(
            "name",
            access_route.name.expect("the route's name should be set"),
            "the access route should have the trimmed name",
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
        assert!(
            latest_verifier_detail_store_ro(deps.as_ref().storage).may_load(DEFAULT_SCOPE_ADDRESS.as_bytes())
                .expect("attempting a may_load on the latest verifier detail should succeed")
                .is_some(),
            "the latest verifier detail should successfully load and be populated after a successful onboard",
        );

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
                delete_latest_verifier_detail(deps.as_mut().storage, DEFAULT_SCOPE_ADDRESS)
                    .expect("latest verifier detail deletion should succeed");
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
        assert!(
            latest_verifier_detail_store_ro(deps.as_ref().storage).may_load(DEFAULT_SCOPE_ADDRESS.as_bytes())
                .expect("attempting a may_load on the default scope address should succeed")
                .is_none(),
            "the latest verifier detail should not be present in contract storage after verification completes",
        );
    }

    fn get_default_test_attribute() -> AssetScopeAttribute {
        AssetScopeAttribute::new(
            &AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
            DEFAULT_ASSET_TYPE,
            DEFAULT_SENDER_ADDRESS,
            DEFAULT_VERIFIER_ADDRESS,
            AssetOnboardingStatus::Pending.to_some(),
            &get_default_verifier_detail(),
            get_default_access_routes(),
        )
        .expect("failed to instantiate default asset scope attribute")
    }
}
