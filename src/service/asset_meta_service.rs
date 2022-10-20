use std::collections::HashSet;

use cosmwasm_std::{to_binary, Coin, CosmosMsg, Env, Uint128};
use provwasm_std::{assess_custom_fee, update_attribute, AttributeValueType, ProvenanceMsg};
use result_extensions::ResultExtensions;

use crate::core::state::{
    delete_fee_payment_detail, insert_fee_payment_detail, load_fee_payment_detail,
};
use crate::core::types::fee_payment_detail::FeePaymentDetail;
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::query::query_asset_scope_attribute_by_asset_type::{
    may_query_scope_attribute_by_scope_address_and_asset_type,
    query_scope_attribute_by_scope_address_and_asset_type,
};
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
    util::functions::filter_valid_access_routes,
    util::functions::generate_asset_attribute_name,
    util::vec_container::VecContainer,
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
    fn has_asset<S1: Into<String>, S2: Into<String>>(
        &self,
        scope_address: S1,
        asset_type: S2,
    ) -> AssetResult<bool> {
        let scope_address: String = scope_address.into();
        let asset_type = asset_type.into();
        // check for asset attribute existence
        self.use_deps(|d| {
            may_query_scope_attribute_by_scope_address_and_asset_type(
                &d.as_ref(),
                &scope_address,
                &asset_type,
            )
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
        if self.has_asset(&attribute.scope_address, &attribute.asset_type)? != is_retry {
            return if is_retry {
                ContractError::generic(format!("unexpected state! asset scope [{}] was processed as new onboard, but the scope was not populated with asset classification data", &attribute.scope_address))
            } else {
                ContractError::AssetAlreadyOnboarded {
                    scope_address: attribute.scope_address.clone(),
                    asset_type: attribute.asset_type.clone(),
                }
            }.to_err();
        }

        // Fetch any existing scope attributes for use in calculating the onboarding cost, which
        // may change if an existing scope attribute on this asset has used a different asset type
        // from the same verifier address.
        let existing_scope_attributes = self
            .use_deps(|deps| {
                may_query_scope_attribute_by_scope_address(&deps.as_ref(), &attribute.scope_address)
            })?
            .unwrap_or_default();

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

        // Retry fees should only be used when an asset is classified as a specific asset type with
        // a specific verifier and rejected.  After rejection, the retry fee amount should be used
        // in place of normal onboarding costs ONLY if the asset is onboarded as the same type of
        // asset with the same verifier.  Without this check, an asset could fail onboarding with
        // one verifier, and then take advantage of a retry fee reduction by using a wholly
        // different verifier.
        let calculate_retry_fees = is_retry
            && existing_scope_attributes
                .iter()
                .find(|attr| attr.asset_type == attribute.asset_type)
                .map(|attr| attr.verifier_address.as_str() == attribute.verifier_address.as_str())
                .unwrap_or(false);

        let payment_detail = FeePaymentDetail::new(
            &attribute.scope_address,
            verifier_detail,
            calculate_retry_fees,
            &attribute.asset_type,
            &existing_scope_attributes,
        )?;
        // No need to assess a fee from the onboarding user if there is no requested fee
        if !payment_detail.payments.is_empty() {
            self.append_messages(&[assess_custom_fee(
                Coin {
                    denom: verifier_detail.onboarding_denom.clone(),
                    // The payment detail now contains the originally-specified fee to be charged,
                    // but halved.  Charge a fee to the onboarding requestor using double the value
                    // derived in the payment detail to ensure the correct funds are sent to the
                    // contract
                    amount: Uint128::new(payment_detail.sum_costs() * 2),
                },
                Some("Asset Classification Onboarding Fee"),
                // The contract address must always be used as the "from" value to ensure that
                // permission issues do not occur when submitting the message.
                env.contract.address.to_owned(),
                Some(env.contract.address.to_owned()),
            )?]);
        }
        self.use_deps(|deps| {
            insert_fee_payment_detail(deps.storage, &payment_detail, &attribute.asset_type)
        })?;
        Ok(())
    }

    fn update_attribute(&self, updated_attribute: &AssetScopeAttribute) -> AssetResult<()> {
        let contract_base_name = self
            .use_deps(|d| config_read_v2(d.storage).load())?
            .base_contract_name;
        let original_attribute = self.get_asset_by_asset_type(
            &updated_attribute.scope_address,
            &updated_attribute.asset_type,
        )?;
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

    fn get_asset<S1: Into<String>>(
        &self,
        scope_address: S1,
    ) -> AssetResult<Vec<AssetScopeAttribute>> {
        let scope_address_string: String = scope_address.into();
        // try to fetch asset from attribute meta, if found also fetch scope attribute and reconstruct AssetMeta from relevant pieces
        self.use_deps(|d| {
            query_scope_attribute_by_scope_address(&d.as_ref(), &scope_address_string)
        })
    }

    fn try_get_asset<S1: Into<String>>(
        &self,
        scope_address: S1,
    ) -> AssetResult<Option<Vec<AssetScopeAttribute>>> {
        let scope_address_string: String = scope_address.into();
        self.use_deps(|d| {
            may_query_scope_attribute_by_scope_address(&d.as_ref(), &scope_address_string)
        })
    }

    fn get_asset_by_asset_type<S1: Into<String>, S2: Into<String>>(
        &self,
        scope_address: S1,
        asset_type: S2,
    ) -> AssetResult<AssetScopeAttribute> {
        let scope_address = scope_address.into();
        let asset_type = asset_type.into();
        // try to fetch asset from attribute meta, if found also fetch scope attribute and reconstruct AssetMeta from relevant pieces
        self.use_deps(|d| {
            query_scope_attribute_by_scope_address_and_asset_type(
                &d.as_ref(),
                &scope_address,
                &asset_type,
            )
        })
    }

    fn try_get_asset_by_asset_type<S1: Into<String>, S2: Into<String>>(
        &self,
        scope_address: S1,
        asset_type: S2,
    ) -> AssetResult<Option<AssetScopeAttribute>> {
        let scope_address_string: String = scope_address.into();
        let asset_type_string: String = asset_type.into();
        self.use_deps(|d| {
            may_query_scope_attribute_by_scope_address_and_asset_type(
                &d.as_ref(),
                &scope_address_string,
                &asset_type_string,
            )
        })
    }

    fn verify_asset<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        &self,
        scope_address: S1,
        asset_type: S2,
        success: bool,
        verification_message: Option<S3>,
        access_routes: Vec<AccessRoute>,
    ) -> AssetResult<()> {
        // set verification result on asset (add messages to message service)
        let scope_address_str = scope_address.into();
        let mut attribute = self.get_asset_by_asset_type(scope_address_str, asset_type)?;
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
            true => AssetOnboardingStatus::Approved,
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

        // Retrieve fee breakdown and use it to emit message fees
        let payment_detail = self.use_deps(|deps| {
            load_fee_payment_detail(
                deps.storage,
                &attribute.scope_address,
                &attribute.asset_type,
            )
        })?;
        // Pay the verifier detail fees after verification has successfully been completed
        self.append_messages(&payment_detail.to_bank_send_msgs()?);

        // Remove the fee payment detail after it has been used for verification.
        // Stored fee payment amounts are no longer needed after the custom bank send messages have been
        // used, as it can easily become outdated in the future
        self.use_deps(|deps| {
            delete_fee_payment_detail(
                deps.storage,
                &attribute.scope_address,
                &attribute.asset_type,
            )
        })?;

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
    use crate::core::state::{insert_fee_payment_detail, load_fee_payment_detail};
    use crate::execute::update_asset_definition::{
        update_asset_definition, UpdateAssetDefinitionV1,
    };
    use crate::testutil::test_constants::{
        DEFAULT_ADMIN_ADDRESS, DEFAULT_ONBOARDING_DENOM, DEFAULT_SECONDARY_ASSET_TYPE,
    };
    use crate::testutil::test_utilities::{
        empty_mock_info, get_default_asset_definition, get_duped_fee_payment_detail,
    };
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
    use cosmwasm_std::{from_binary, to_binary, Addr, BankMsg, Coin, CosmosMsg, StdError};
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
        let result = repository
            .has_asset(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .unwrap();
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

        let result = repository
            .has_asset(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .unwrap();

        assert!(
            result,
            "Repository should return true when asset does have attribute"
        );
    }

    #[test]
    fn has_asset_returns_false_if_asset_has_attribute_for_different_type() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(
            &mut deps,
            InstArgs::default_with_additional_asset_types(vec![DEFAULT_SECONDARY_ASSET_TYPE]),
        );
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();

        let repository = AssetMetaService::new(deps.as_mut());

        let result = repository
            .has_asset(DEFAULT_SCOPE_ADDRESS, DEFAULT_SECONDARY_ASSET_TYPE)
            .unwrap();

        assert!(
            !result,
            "Repository should return false when asset doesn't have attribute for specified type (but does for another type)"
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
                &get_default_test_attribute(),
                &get_default_verifier_detail(),
                false,
            )
            .unwrap_err();

        match err {
            ContractError::AssetAlreadyOnboarded {
                scope_address,
                asset_type,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS.to_string(),
                    scope_address,
                    "Scope address should be reflected in AssetAlreadyOnboarded error"
                );
                assert_eq!(
                    DEFAULT_ASSET_TYPE.to_string(),
                    asset_type,
                    "Asset type should be reflected in AssetAlreadyOnboarded error"
                )
            }
            _ => panic!(
                "Received unknown error when onboarding already-onboarded asset: {:?}",
                err
            ),
        }
    }

    #[test]
    fn add_asset_generates_proper_messages() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let repository = AssetMetaService::new(deps.as_mut());

        let verifier_detail = get_default_verifier_detail();
        repository
            .onboard_asset(
                &mock_env(),
                &get_default_test_attribute(),
                &verifier_detail,
                false,
            )
            .unwrap();

        let messages = repository.get_messages();

        assert_eq!(
            2,
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
                let expected = get_default_asset_scope_attribute();
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
                    DEFAULT_ONBOARDING_COST,
                    amount.amount.u128(),
                    "double the onboarding cost should be charged to account for provenance fees",
                );
                assert!(name.is_some(), "a fee name should be provided",);
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    from.as_str(),
                    "the contract address should be set as the from value",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    recipient
                        .to_owned()
                        .expect("a recipient should be defined")
                        .as_str(),
                    "the verifier should receive the fees",
                );
            }
            msg => panic!(
                "Unexpected message type resulting from add_asset: {:?}",
                msg
            ),
        });
    }

    #[test]
    fn get_asset_returns_error_if_asset_does_not_exist() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        let repository = AssetMetaService::new(deps.as_mut());

        let err = repository
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .unwrap_err();

        match err {
            ContractError::NotFound { explanation } => assert_eq!(
                format!(
                    "scope at address [{}] did not include an asset scope attribute for asset type [{}]",
                    DEFAULT_SCOPE_ADDRESS,
                    DEFAULT_ASSET_TYPE
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

        let attribute = repository
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
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
        setup_test_suite(&mut deps, InstArgs::default());
        let repository = AssetMetaService::new(deps.as_mut());

        let result = repository
            .try_get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .unwrap();

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
            .try_get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
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
            .verify_asset::<&str, &str, &str>(
                DEFAULT_SCOPE_ADDRESS,
                DEFAULT_ASSET_TYPE,
                true,
                None,
                vec![],
            )
            .unwrap_err();

        match err {
            ContractError::NotFound { explanation } => assert_eq!(
                explanation,
                format!(
                    "scope at address [{}] did not include an asset scope attribute for asset type [{}]",
                    DEFAULT_SCOPE_ADDRESS,
                    DEFAULT_ASSET_TYPE
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
                })
                .unwrap()
                .as_str(),
                "json",
            )],
        );

        setup_test_suite(&mut deps, InstArgs::default());
        let mut fee_payment_detail = get_duped_fee_payment_detail(DEFAULT_SCOPE_ADDRESS);
        fee_payment_detail.payments = vec![fee_payment_detail.payments[0].clone()];
        fee_payment_detail.payments[0].recipient = Addr::unchecked(DEFAULT_VERIFIER_ADDRESS);
        fee_payment_detail.payments[0].amount =
            Coin::new(DEFAULT_ONBOARDING_COST / 2, DEFAULT_ONBOARDING_DENOM);
        insert_fee_payment_detail(&mut deps.storage, &fee_payment_detail, DEFAULT_ASSET_TYPE)
            .unwrap();
        let repository = AssetMetaService::new(deps.as_mut());

        repository
            .verify_asset::<&str, &str, &str>(
                DEFAULT_SCOPE_ADDRESS,
                DEFAULT_ASSET_TYPE,
                true,
                "Great jaerb there Hamstar".to_some(),
                vec![AccessRoute::route_only("newroute")],
            )
            .unwrap();

        let messages = repository.messages.get();

        assert_eq!(
            2,
            messages.len(),
            "verify asset should produce 2 messages (update attribute msg and 1 bank send msg)"
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
        let second_message = &messages[1];
        match second_message {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    to_address,
                    &DEFAULT_VERIFIER_ADDRESS.to_string(),
                    "fee message should go to default verifier address"
                );
                assert_eq!(
                    1,
                    amount.len(),
                    "exactly one coin type should be present on bank send fee message"
                );
                assert_eq!(
                    amount.first().unwrap().amount.u128(),
                    DEFAULT_ONBOARDING_COST / 2,
                    "bank send fee message should be half the default onboarding cost"
                );
                assert_eq!(
                    amount.first().unwrap().denom,
                    DEFAULT_ONBOARDING_DENOM.to_string(),
                    "bank send fee message should use the default onboarding denom"
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
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
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
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
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
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
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
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
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
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
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
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
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
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
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

    #[test]
    fn test_finalize_classification_success_with_retained_verifier() {
        assert_verify_classification_success(false);
    }

    #[test]
    fn test_finalize_classification_success_with_deleted_verifier() {
        assert_verify_classification_success(true);
    }

    fn test_verification_result(message: Option<&str>, result: bool) {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let repository = AssetMetaService::new(deps.as_mut());
        let original_attribute_value = repository
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect(
                "original attribute value should load from Provenance Blockchain without issue",
            );
        repository
            .verify_asset::<&str, &str, &str>(
                DEFAULT_SCOPE_ADDRESS,
                DEFAULT_ASSET_TYPE,
                result,
                message,
                vec![],
            )
            .unwrap();

        let messages = repository.get_messages();

        assert_eq!(
            2,
            messages.len(),
            "verify asset should produce two message (update attribute msg and one bank send message)"
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
        let second_message = &messages[1];
        match second_message {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    &DEFAULT_VERIFIER_ADDRESS.to_string(),
                    to_address,
                    "fee message should go to the verifier"
                );
                assert_eq!(
                    &vec![Coin::new(
                        DEFAULT_ONBOARDING_COST / 2,
                        DEFAULT_ONBOARDING_DENOM
                    )],
                    amount,
                    "fee message should be of the proper amount"
                );
            }
            _ => panic!(
                "Unexpected second message type for verify_asset: {:?}",
                second_message
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
            get_default_access_routes(),
        )
        .expect("failed to instantiate default asset scope attribute")
    }

    fn assert_verify_classification_success(simulate_deleted_verifier: bool) {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect("fee payment detail should be stored");
        // test_verify_asset(&mut deps, TestVerifyAsset::default()).unwrap();
        // Overwrite the default asset definition with a new verifier detail that's identical to the
        // original value, with the exception of having a new address.  This will effectively
        // simulate the situation where a verifier "disappears" after an asset has been verified.
        // This situation, due to stored fee information, should still be identical to the scenario
        // where the verifier remains in storage
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
            .get_asset_by_asset_type(DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE)
            .expect("the asset should be available after verification");
        assert_eq!(
            AssetOnboardingStatus::Pending,
            asset.onboarding_status,
            "sanity check: the asset should be in Pending status",
        );
        service
            .verify_asset(
                DEFAULT_SCOPE_ADDRESS,
                DEFAULT_ASSET_TYPE,
                true,
                Some("great jaerb there hamstar"),
                get_default_access_routes(),
            )
            .expect("finalize classification should succeed");
        let messages = service.get_messages();
        assert_eq!(
            2,
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
                let updated_asset = from_binary::<AssetScopeAttribute>(update_value)
                    .expect("the update_value should deserialize without error");
                assert_eq!(
                    AssetOnboardingStatus::Approved,
                    updated_asset.onboarding_status,
                    "the updated asset's onboarding status should be changed to approve",
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
        let fee_payment_msg = &messages[1];
        match fee_payment_msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(
                    1,
                    amount.len(),
                    "exactly one coin amount should be present on bank send message"
                );
                assert_eq!(
                    DEFAULT_ONBOARDING_COST / 2,
                    amount.first().unwrap().amount.u128(),
                    "the fee amount should equate to half the onboarding cost due to provenance's 50% fee cut",
                );
                assert_eq!(
                    DEFAULT_ONBOARDING_DENOM,
                    amount.first().unwrap().denom,
                    "the fee should use the correct denom",
                );
                assert_eq!(
                    DEFAULT_VERIFIER_ADDRESS,
                    to_address.as_str(),
                    "the recipient of the fee should be the verifier",
                );
            }
            msg => panic!(
                "the second message generated should be a bank send msg, but got: {:?}",
                msg,
            ),
        };
        let err = service
            .use_deps(|deps| {
                load_fee_payment_detail(
                    deps.as_ref().storage,
                    DEFAULT_SCOPE_ADDRESS,
                    DEFAULT_ASSET_TYPE,
                )
            })
            .expect_err(
                "an error should occur when trying to fetch payment detail after finalization",
            );
        assert!(
            matches!(err, ContractError::Std(StdError::NotFound { .. })),
            "a not found error should occur for the fee payment detail after finalization completes, but got: {:?}",
            err,
        );
    }
}
