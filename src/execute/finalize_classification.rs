use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::types::asset_identifier::AssetIdentifier;
use crate::core::types::asset_onboarding_status::AssetOnboardingStatus;
use crate::service::asset_meta_repository::AssetMetaRepository;
use crate::service::deps_manager::DepsManager;
use crate::service::message_gathering_service::MessageGatheringService;
use crate::util::aliases::{AssetResult, EntryPointResponse};
use crate::util::contract_helpers::check_funds_are_empty;
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Env, MessageInfo, Response};

/// A transformation of [ExecuteMsg::FinalizeClassification](crate::core::msg::ExecuteMsg::FinalizeClassification)
/// for ease of use in the underlying [finalize_classification](self::finalize_classification) function.
pub struct FinalizeClassificationV1 {
    /// An instance of the asset identifier enum that helps the contract identify which
    /// scope that the requestor is referring to in the request.
    pub identifier: AssetIdentifier,
}
impl FinalizeClassificationV1 {
    /// Attempts to create an instance of this struct from a provided execute msg.  If the provided
    /// value is not of the [FinalizeClassification](crate::core::msg::ExecuteMsg::FinalizeClassification)
    /// variant, then an [InvalidMessageType](crate::core::error::ContractError::InvalidMessageType)
    /// error will be returned.
    ///
    /// # Parameters
    ///
    /// * `msg` An execute msg provided by the contract's [execute](crate::contract::execute) function.
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<FinalizeClassificationV1> {
        match msg {
            ExecuteMsg::FinalizeClassification { identifier } => FinalizeClassificationV1 {
                identifier: identifier.to_asset_identifier()?,
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::FinalizeClassification".to_string(),
            }
            .to_err(),
        }
    }
}

/// The function used by [execute](crate::contract::execute) when an [ExecuteMsg::FinalizeClassification](crate::core::msg::ExecuteMsg::FinalizeClassification)
/// message is provided.  This route is to be used exclusively in the circumstance when a requestor
/// chooses a [trust_verifier](crate::core::types::asset_scope_attribute::AssetScopeAttribute::trust_verifier)
/// value of `false`.  After the verification process is completed when trust verifier is `false`,
/// the scope attribute will be moved to an [onboarding_status](crate::core::types::asset_scope_attribute::AssetScopeAttribute::onboarding_status)
/// of [AwaitingFinalization](crate::core::types::asset_onboarding_status::AssetOnboardingStatus::AwaitingFinalization).
/// This route charges the requestor funds equivalent to the stored [FeePaymentDetail](crate::core::types::fee_payment_detail::FeePaymentDetail)
/// in the form of Provenance Blockchain custom MsgFees, and then changes the status from awaiting
/// finalization to [Approved](crate::core::types::asset_onboarding_status::AssetOnboardingStatus::Approved),
/// effectively classifying the asset.
///
/// # Parameters
///
/// * `repository` A helper collection of traits that allows complex lookups of scope values and
/// emits messages to construct the process of finalization as a collection of messages to produce
/// in the function's result.
/// * `env` The environment value provided during message execution, used to derive the contract's
/// bech32 address for use in custom message fees.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` An instance of the finalize classification v1 struct, provided by conversion from an
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).
pub fn finalize_classification<'a, T>(
    repository: T,
    env: Env,
    info: MessageInfo,
    msg: FinalizeClassificationV1,
) -> EntryPointResponse
where
    T: AssetMetaRepository + MessageGatheringService + DepsManager<'a>,
{
    check_funds_are_empty(&info)?;
    let attribute = repository.get_asset(msg.identifier.get_scope_address()?)?;
    if info.sender != attribute.requestor_address {
        return ContractError::Unauthorized {
            explanation: format!(
                "only the requestor [{}] may finalize classification for asset [{}]",
                attribute.requestor_address.as_str(),
                attribute.scope_address,
            ),
        }
        .to_err();
    }
    if attribute.onboarding_status != AssetOnboardingStatus::AwaitingFinalization {
        return ContractError::InvalidFinalization {
            explanation: format!(
                "finalization can run for assets with an onboarding status of [{}], but the status was [{}]",
                AssetOnboardingStatus::AwaitingFinalization,
                attribute.onboarding_status,
            )
        }.to_err();
    }
    repository.finalize_classification(&env, &attribute)?;
    Response::new()
        .add_attributes(
            EventAttributes::for_asset_event(
                EventType::FinalizeClassification,
                &attribute.asset_type,
                &attribute.scope_address,
            )
            .set_verifier(attribute.verifier_address.as_str())
            .set_scope_owner(&attribute.requestor_address.as_str()),
        )
        .add_messages(repository.get_messages())
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::core::error::ContractError;
    use crate::core::state::load_fee_payment_detail;
    use crate::core::types::asset_onboarding_status::AssetOnboardingStatus;
    use crate::core::types::asset_scope_attribute::AssetScopeAttribute;
    use crate::service::asset_meta_repository::AssetMetaRepository;
    use crate::service::asset_meta_service::AssetMetaService;
    use crate::testutil::finalize_classification_helpers::{
        test_finalize_classification, TestFinalizeClassification,
    };
    use crate::testutil::onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset};
    use crate::testutil::test_constants::{
        DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME, DEFAULT_ONBOARDING_COST,
        DEFAULT_ONBOARDING_DENOM, DEFAULT_SCOPE_ADDRESS, DEFAULT_SENDER_ADDRESS,
        DEFAULT_VERIFIER_ADDRESS,
    };
    use crate::testutil::test_utilities::{
        empty_mock_info, setup_test_suite, single_attribute_for_key, InstArgs, MockOwnedDeps,
    };
    use crate::testutil::verify_asset_helpers::{test_verify_asset, TestVerifyAsset};
    use crate::util::constants::{
        ASSET_EVENT_TYPE_KEY, ASSET_SCOPE_ADDRESS_KEY, ASSET_TYPE_KEY, SCOPE_OWNER_KEY, USD,
        VERIFIER_ADDRESS_KEY,
    };
    use crate::util::event_attributes::EventType;
    use crate::util::functions::generate_asset_attribute_name;
    use cosmwasm_std::testing::{mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, from_binary, CosmosMsg, StdError};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{
        AttributeMsgParams, AttributeValueType, MsgFeesMsgParams, ProvenanceMsg,
        ProvenanceMsgParams,
    };

    #[test]
    fn test_failure_for_provided_funds() {
        let mut deps = mock_dependencies(&[]);
        let err = test_finalize_classification(
            &mut deps,
            TestFinalizeClassification {
                info: mock_info(DEFAULT_SENDER_ADDRESS, &coins(100, USD)),
                ..TestFinalizeClassification::default()
            },
        )
        .expect_err("an error should occur when funds are provided to finalize classification");
        assert!(
            matches!(err, ContractError::InvalidFunds(_)),
            "an invalid funds error should occur, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_failure_for_missing_asset() {
        let mut deps = mock_dependencies(&[]);
        let err = test_finalize_classification(&mut deps, TestFinalizeClassification::default())
            .expect_err("an error should occur when no scope attribute exists on the target asset");
        assert!(
            matches!(err, ContractError::Std(StdError::GenericErr { .. })),
            "a generic StdError should occur when the scope is not found, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_failure_for_wrong_sender_address() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        let err = test_finalize_classification(&mut deps, TestFinalizeClassification {
            info: empty_mock_info("some rando"),
            ..TestFinalizeClassification::default()
        }).expect_err("an error should occur when an account that does not own the asset tries to finalize classification");
        assert!(
            matches!(err, ContractError::Unauthorized { .. }),
            "an unauthorized error should occur, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_failure_for_pending_asset() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        assert_eq!(
            AssetOnboardingStatus::Pending,
            get_asset(&mut deps).onboarding_status,
            "sanity check: the asset should be in pending onboarding status",
        );
        let err = test_finalize_classification(&mut deps, TestFinalizeClassification::default())
            .expect_err("an error should occur when the asset is in pending status");
        assert!(
            matches!(err, ContractError::InvalidFinalization { .. }),
            "an invalid finalization error should occur, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_failure_for_approved_asset() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_verify_asset(&mut deps, TestVerifyAsset::default()).unwrap();
        assert_eq!(
            AssetOnboardingStatus::Approved,
            get_asset(&mut deps).onboarding_status,
            "sanity check: the asset should be in approved onboarding status",
        );
        let err = test_finalize_classification(&mut deps, TestFinalizeClassification::default())
            .expect_err("an error should occur when the asset is in approved status");
        assert!(
            matches!(err, ContractError::InvalidFinalization { .. }),
            "an invalid finalization error should occur, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_failure_for_denied_asset() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();
        test_verify_asset(&mut deps, TestVerifyAsset::default_with_success(false)).unwrap();
        assert_eq!(
            AssetOnboardingStatus::Denied,
            get_asset(&mut deps).onboarding_status,
            "sanity check: the asset should be in denied onboarding status",
        );
        let err = test_finalize_classification(&mut deps, TestFinalizeClassification::default())
            .expect_err("an error should occur when the asset is in denied status");
        assert!(
            matches!(err, ContractError::InvalidFinalization { .. }),
            "an invalid finalization error should occur, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_successful_finalize_classification() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(
            &mut deps,
            // Finalize classification can only ever occur if the asset is put into AwaitingFinalization
            // status, which only occurs when it is onboarded with trust_verifier = false.
            TestOnboardAsset::default_with_trust_verifier(false),
        )
        .unwrap();
        load_fee_payment_detail(deps.as_ref().storage, DEFAULT_SCOPE_ADDRESS).expect(
            "a fee payment detail should be created after onboarding with no trust for verifier",
        );
        test_verify_asset(&mut deps, TestVerifyAsset::default()).unwrap();
        let asset_before_update = get_asset(&mut deps);
        assert_eq!(
            AssetOnboardingStatus::AwaitingFinalization,
            asset_before_update.onboarding_status,
            "sanity check: the asset should have the awaiting finalization onboarding status",
        );
        let response =
            test_finalize_classification(&mut deps, TestFinalizeClassification::default())
                .expect("finalizing classification should succeed without issue");
        assert_eq!(
            5,
            response.attributes.len(),
            "the correct number of attributes should be emitted",
        );
        assert_eq!(
            EventType::FinalizeClassification.event_name(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "the event type key should have the correct value",
        );
        assert_eq!(
            DEFAULT_ASSET_TYPE,
            single_attribute_for_key(&response, ASSET_TYPE_KEY),
            "the asset type key should have the correct value",
        );
        assert_eq!(
            DEFAULT_SCOPE_ADDRESS,
            single_attribute_for_key(&response, ASSET_SCOPE_ADDRESS_KEY),
            "the scope address key should have the correct value",
        );
        assert_eq!(
            DEFAULT_VERIFIER_ADDRESS,
            single_attribute_for_key(&response, VERIFIER_ADDRESS_KEY),
            "the verifier address key should have the correct value",
        );
        assert_eq!(
            DEFAULT_SENDER_ADDRESS,
            single_attribute_for_key(&response, SCOPE_OWNER_KEY),
            "the scope owner key should have the correct value",
        );
        assert_eq!(
            2,
            response.messages.len(),
            "the response should have the correct number of messages",
        );
        response.messages.iter().for_each(|msg| match &msg.msg {
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
                    asset_before_update,
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
                    asset_before_update,
                    updated_asset,
                    "the only field that should change in the update is the onboarding status",
                );
                assert_eq!(
                    &AttributeValueType::Json,
                    update_value_type,
                    "the json value type should be used for the update_value_type",
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
                    DEFAULT_ONBOARDING_COST * 2,
                    amount.amount.u128(),
                    "the fee amount should equate to double the onboarding cost to cover provenance's fee cut",
                );
                assert_eq!(
                    DEFAULT_ONBOARDING_DENOM,
                    amount.denom,
                    "the fee should use the correct denom",
                );
                assert!(
                    name.is_some(),
                    "the name should be set on the fee",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    from.as_str(),
                    "the contract address should always be set in the 'from' field",
                );
                assert_eq!(
                    DEFAULT_VERIFIER_ADDRESS,
                    recipient.to_owned().expect("a recipient should be set").as_str(),
                    "the recipient of the fee should be the verifier",
                );
            }
            msg => panic!(
                "unexpected message after finalizing classification: {:?}",
                msg
            ),
        });
        let err = load_fee_payment_detail(deps.as_ref().storage, DEFAULT_SCOPE_ADDRESS).expect_err(
            "an error should occur when trying to fetch payment detail after finalization",
        );
        assert!(
            matches!(err, ContractError::Std(StdError::NotFound { .. })),
            "a not found error should occur for the fee payment detail after finalization completes, but got: {:?}",
            err,
        );
    }

    fn get_asset(deps: &mut MockOwnedDeps) -> AssetScopeAttribute {
        AssetMetaService::new(deps.as_mut())
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("the asset should be available by the default scope address")
    }
}
