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

pub struct FinalizeClassificationV1 {
    pub identifier: AssetIdentifier,
}
impl FinalizeClassificationV1 {
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
            .set_verifier(&attribute.scope_address)
            .set_scope_owner(&attribute.requestor_address.as_str()),
        )
        .add_messages(repository.get_messages())
        .to_ok()
}
