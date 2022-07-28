use std::collections::HashSet;

use cosmwasm_std::{to_binary, CosmosMsg, Env};
use provwasm_std::{update_attribute, AttributeValueType, ProvenanceMsg};

use crate::core::state::load_asset_definition_v2_by_type;
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
        env: &Env,
        attribute: &AssetScopeAttribute,
        verifier_detail: &VerifierDetailV2,
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
            // If the onboarding account trusts the verifier, then the verifier gets paid in
            // Provenance Blockchain FeeMsg messages upfront
            if attribute.trust_verifier {
                self.append_messages(&calculate_verifier_cost_messages(env, verifier_detail)?);
            }
        }
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
            to_binary(&updated_attribute)?,
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
        let mut attribute = self.get_asset(scope_address_str)?;
        let message = verification_message.map(|m| m.into()).unwrap_or_else(|| {
            match success {
                true => "verification successful",
                false => "verification failure",
            }
            .to_string()
        });

        attribute.latest_verification_result = Some(AssetVerificationResult { message, success });

        // change the onboarding status based on how the verifier specified the success status
        attribute.onboarding_status = match success {
            true => {
                // if the verifier has been trusted, then the process is over as of verification.
                // if not, the finalize classification step needs to run before approval can be
                // solidified
                if attribute.trust_verifier {
                    AssetOnboardingStatus::Approved
                } else {
                    AssetOnboardingStatus::AwaitingFinalization
                }
            }
            false => AssetOnboardingStatus::Denied,
        };

        let verifier_address = attribute.verifier_address.as_str();

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
                owner_address: verifier_address.to_string(),
                access_routes: filtered_access_routes,
                definition_type: AccessDefinitionType::Verifier,
            });
        }
        // Remove the old scope attribute and append a new one that overwrites existing data
        // with the changes made to the attribute
        self.update_attribute(&attribute)?;

        Ok(())
    }

    fn finalize_classification(
        &self,
        env: &Env,
        attribute: &AssetScopeAttribute,
    ) -> AssetResult<()> {
        let mut attribute = attribute.to_owned();
        attribute.onboarding_status = AssetOnboardingStatus::Approved;
        self.update_attribute(&attribute)?;
        // If the target verifier detail still exists when the finalize classification step is
        // reached, then the verifier and all other fee destinations are subsequently paid by the
        // requestor.  This route can only be reached if the requestor decides NOT to trust the
        // verifier to complete its work.  Due to this, if a verifier detail ceases to exist after
        // these processes have been completed, then the untrusting requestor lucked out and does
        // not have to pay any fees!
        if let Ok(verifier_detail) = self.use_deps(|deps| {
            load_asset_definition_v2_by_type(deps.storage, &attribute.asset_type)
                .and_then(|asset_def| asset_def.get_verifier_detail(&attribute.verifier_address))
        }) {
            // Pay the verifier detail fees after verification has successfully been completed
            self.append_messages(&calculate_verifier_cost_messages(env, &verifier_detail)?);
        }
        ().to_ok()
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
    use crate::execute::update_asset_definition::{
        update_asset_definition, UpdateAssetDefinitionV1,
    };
    use crate::testutil::test_constants::{
        DEFAULT_ADMIN_ADDRESS, DEFAULT_ONBOARDING_DENOM, DEFAULT_TRUST_VERIFIER,
    };
    use crate::testutil::test_utilities::{empty_mock_info, get_default_asset_definition};
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
                DEFAULT_ONBOARDING_COST, DEFAULT_SCOPE_ADDRESS, DEFAULT_SENDER_ADDRESS,
                DEFAULT_VERIFIER_ADDRESS,
            },
            test_utilities::{
                assert_single_item, get_default_access_routes, get_default_asset_scope_attribute,
                get_default_verifier_detail, setup_test_suite, test_instantiate_success, InstArgs,
            },
            verify_asset_helpers::{test_verify_asset, TestVerifyAsset},
        },
        util::{functions::generate_asset_attribute_name, traits::OptionExtensions},
    };
    use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{from_binary, to_binary, Addr, CosmosMsg};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{
        AttributeMsgParams, AttributeValueType, MsgFeesMsgParams, ProvenanceMsg,
        ProvenanceMsgParams,
    };
    use serde_json_wasm::to_string;

    #[test]
    fn has_asset_returns_false_if_asset_does_not_have_the_attribute() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        let repository = AssetMetaService::new(deps.as_mut());
        let result = repository.has_asset(DEFAULT_SCOPE_ADDRESS).unwrap();
        assert!(
            !result,
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

        assert!(
            result,
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
                &mock_env(),
                &get_default_test_attribute(false),
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
    fn add_asset_generates_proper_attribute_message_with_trust_verifier() {
        test_add_asset_message_generation(true);
    }

    #[test]
    fn add_asset_generates_proper_messages_with_dont_trust_verifier() {
        test_add_asset_message_generation(false);
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
                    trust_verifier: DEFAULT_TRUST_VERIFIER,
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
                vec![AccessRoute::route_only("newroute")],
            )
            .unwrap();

        let messages = repository.messages.get();

        assert_eq!(
            1,
            messages.len(),
            "verify asset should produce 1 message (update attribute msg)"
        );

        let first_message = &messages[0];
        match first_message {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::UpdateAttribute {
                        update_value,
                        ..
                    }),
                ..
            }) => {
                let deserialized: AssetScopeAttribute = from_binary(update_value).unwrap();
                assert_eq!(
                    2,
                    deserialized.access_definitions.len(),
                    "Modified scope attribute should only have 2 access route groups listed",
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
                    "sender access route should be unchanged after verifier routes updated",
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
                    "sender access route should be unchanged after verifier routes updated",
                );
            }
            _ => panic!(
                "Unexpected first message type for verify_asset: {:?}",
                first_message,
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
        assert!(
            !attribute
                .access_definitions
                .into_iter()
                .any(|d| d.owner_address.as_str() == DEFAULT_VERIFIER_ADDRESS),
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
                    .unwrap_or_else(|| "not the right name".to_string())
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

    fn test_add_asset_message_generation(trust_verifier: bool) {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let repository = AssetMetaService::new(deps.as_mut());

        let verifier_detail = get_default_verifier_detail();
        repository
            .onboard_asset(
                &mock_env(),
                &get_default_test_attribute(trust_verifier),
                &verifier_detail,
                false,
            )
            .unwrap();

        let messages = repository.get_messages();

        assert_eq!(
            1 + if trust_verifier { 1 } else { 0 },
            messages.len(),
            "add_asset should generate the correct number of messages"
        );
        messages.iter().for_each(|msg| match &msg {
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
                    &generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME),
                    name,
                    "attribute name should match what is expected"
                );
                let deserialized: AssetScopeAttribute = from_binary(value).unwrap();
                let mut expected = get_default_asset_scope_attribute();
                expected.trust_verifier = trust_verifier;
                assert_eq!(
                    expected, deserialized,
                    "attribute should contain proper values"
                );
                assert_eq!(
                    &AttributeValueType::Json,
                    value_type,
                    "generated attribute value_type should be Json"
                );
            }
            CosmosMsg::Custom(ProvenanceMsg {
                                  params: ProvenanceMsgParams::MsgFees(MsgFeesMsgParams::AssessCustomFee {
                                                                           amount,
                                                                           name,
                                                                           from,
                                                                           recipient,
                                                                       }),
                                  ..
                              }) => {
                if !trust_verifier {
                    panic!("a custom fee message should not be generated when the sender does not trust the verifier, but got: {:?}", msg);
                }
                assert_eq!(
                    DEFAULT_ONBOARDING_COST * 2,
                    amount.amount.u128(),
                    "double the onboarding cost should be charged to account for provenance fees",
                );
                assert!(
                    name.is_some(),
                    "a fee name should be provided",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    from.as_str(),
                    "the contract address should be set as the from value",
                );
                assert_eq!(
                    DEFAULT_VERIFIER_ADDRESS,
                    recipient.to_owned().expect("a recipient should be defined").as_str(),
                    "the verifier should receive the fees",
                );
            }
            msg => panic!("Unexpected message type resulting from add_asset: {:?}", msg),
        });
    }

    fn test_verification_result(message: Option<&str>, result: bool) {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let repository = AssetMetaService::new(deps.as_mut());
        let original_attribute_value = repository.get_asset(DEFAULT_SCOPE_ADDRESS).expect(
            "original attribute value should load from Provenance Blockchain without issue",
        );
        repository
            .verify_asset::<&str, &str>(DEFAULT_SCOPE_ADDRESS, result, message, vec![])
            .unwrap();

        let messages = repository.get_messages();

        assert_eq!(
            1,
            messages.len(),
            "verify asset should produce one message (update attribute msg)"
        );
        let first_message = &messages[0];
        match first_message {
            CosmosMsg::Custom(ProvenanceMsg {
                params: ProvenanceMsgParams::Attribute(msg),
                ..
            }) => {
                let mut value = original_attribute_value.clone();
                value.latest_verification_result = AssetVerificationResult {
                    message: message
                        .unwrap_or(if result {
                            "verification successful"
                        } else {
                            "verification failure"
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
                    AttributeMsgParams::UpdateAttribute {
                        address: Addr::unchecked(DEFAULT_SCOPE_ADDRESS),
                        name: generate_asset_attribute_name(
                            DEFAULT_ASSET_TYPE,
                            DEFAULT_CONTRACT_BASE_NAME
                        ),
                        original_value: to_binary(&original_attribute_value).unwrap(),
                        original_value_type: AttributeValueType::Json,
                        update_value: to_binary(&value).unwrap(),
                        update_value_type: AttributeValueType::Json
                    },
                    msg.to_owned(),
                    "add attribute message should match what is expected"
                );
            }
            _ => panic!(
                "Unexpected first message type for verify_asset: {:?}",
                first_message
            ),
        }
    }

    fn get_default_test_attribute(trust_verifier: bool) -> AssetScopeAttribute {
        AssetScopeAttribute::new(
            &AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
            DEFAULT_ASSET_TYPE,
            DEFAULT_SENDER_ADDRESS,
            DEFAULT_VERIFIER_ADDRESS,
            AssetOnboardingStatus::Pending.to_some(),
            get_default_access_routes(),
            trust_verifier,
        )
        .expect("failed to instantiate default asset scope attribute")
    }

    #[test]
    fn test_finalize_classification_success_with_retained_verifier() {
        assert_finalize_classification_success(false);
    }

    #[test]
    fn test_finalize_classification_success_with_deleted_verifier() {
        assert_finalize_classification_success(true);
    }

    fn assert_finalize_classification_success(simulate_deleted_verifier: bool) {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(
            &mut deps,
            TestOnboardAsset::default_with_trust_verifier(false),
        )
        .unwrap();
        test_verify_asset(&mut deps, TestVerifyAsset::default()).unwrap();
        // Overwrite the default asset definition with a new verifier detail that's identical to the
        // original value, with the exception of having a new address.  This will effectively
        // simulate the situation where a verifier "disappears" after an asset has been verified.
        if simulate_deleted_verifier {
            let mut definition = get_default_asset_definition();
            definition.verifiers.clear();
            let mut verifier = get_default_verifier_detail();
            verifier.address = "otheraddress".to_string();
            definition.verifiers.push(verifier);
            update_asset_definition(
                deps.as_mut(),
                empty_mock_info(DEFAULT_ADMIN_ADDRESS),
                UpdateAssetDefinitionV1::new(definition),
            )
            .expect("updating the asset definition to remove the verifier should succeed");
        }
        let service = AssetMetaService::new(deps.as_mut());
        let asset = service
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("the asset should be available after verification");
        assert_eq!(
            AssetOnboardingStatus::AwaitingFinalization,
            asset.onboarding_status,
            "sanity check: the asset should be in AwaitingFinalization status",
        );
        service
            .finalize_classification(&mock_env(), &asset)
            .expect("finalize classification should succeed");
        let messages = service.get_messages();
        assert_eq!(
            2 - if simulate_deleted_verifier { 1 } else { 0 },
            messages.len(),
            "the correct number of messages should be generated",
        );
        let update_attribute_msg = &messages[0];
        match update_attribute_msg {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::UpdateAttribute {
                        address,
                        name,
                        original_value,
                        original_value_type,
                        update_value,
                        update_value_type,
                    }),
                ..
            }) => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS,
                    address.as_str(),
                    "the update attribute message should target the scope",
                );
                assert_eq!(
                    &generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME),
                    name,
                    "the correct attribute name should be included in the update",
                );
                assert_eq!(
                    asset,
                    from_binary(original_value).expect("the original_value should deserialize without error"),
                    "the asset value before the update was made should be used as the original_value",
                );
                assert_eq!(
                    &AttributeValueType::Json,
                    original_value_type,
                    "the json value type should be used for the original_value_type",
                );
                let mut updated_asset = from_binary::<AssetScopeAttribute>(update_value)
                    .expect("the update_value should deserialize without error");
                assert_eq!(
                    AssetOnboardingStatus::Approved,
                    updated_asset.onboarding_status,
                    "the updated asset's onboarding status should be changed to approve",
                );
                updated_asset.onboarding_status = AssetOnboardingStatus::AwaitingFinalization;
                assert_eq!(
                    asset, updated_asset,
                    "the only field that should change in the update is the onboarding status",
                );
                assert_eq!(
                    &AttributeValueType::Json,
                    update_value_type,
                    "the json value type should be used for the update_value_type",
                );
            }
            msg => panic!(
                "the first message generated should be an update attribute msg, but got: {:?}",
                msg
            ),
        };
        // If the verifier was deleted before the asset was finalized, no fee can be charged
        if simulate_deleted_verifier {
            return;
        }
        let fee_payment_msg = &messages[1];
        match fee_payment_msg {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::MsgFees(MsgFeesMsgParams::AssessCustomFee {
                        amount,
                        name,
                        from,
                        recipient,
                    }),
                ..
            }) => {
                assert_eq!(
                    DEFAULT_ONBOARDING_COST * 2,
                    amount.amount.u128(),
                    "the fee amount should equate to double the onboarding cost to cover provenance's fee cut",
                );
                assert_eq!(
                    DEFAULT_ONBOARDING_DENOM, amount.denom,
                    "the fee should use the correct denom",
                );
                assert!(name.is_some(), "the name should be set on the fee",);
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    from.as_str(),
                    "the contract address should always be set in the 'from' field",
                );
                assert_eq!(
                    DEFAULT_VERIFIER_ADDRESS,
                    recipient
                        .to_owned()
                        .expect("a recipient should be set")
                        .as_str(),
                    "the recipient of the fee should be the verifier",
                );
            }
            msg => panic!(
                "the second message generated should be a custom fee msg, but got: {:?}",
                msg,
            ),
        };
    }
}
