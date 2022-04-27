use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::config_read_v2;
use crate::core::types::access_definition::AccessDefinition;
use crate::core::types::access_route::AccessRoute;
use crate::core::types::asset_identifier::AssetIdentifier;
use crate::service::asset_meta_repository::AssetMetaRepository;
use crate::service::deps_manager::DepsManager;
use crate::service::message_gathering_service::MessageGatheringService;
use crate::util::aliases::{AssetResult, EntryPointResponse};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::functions::filter_valid_access_routes;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};

#[derive(Clone, PartialEq)]
pub struct UpdateAccessRoutesV1 {
    pub identifier: AssetIdentifier,
    pub owner_address: String,
    pub access_routes: Vec<AccessRoute>,
}
impl UpdateAccessRoutesV1 {
    pub fn new<S: Into<String>>(
        identifier: AssetIdentifier,
        owner_address: S,
        access_routes: Vec<AccessRoute>,
    ) -> Self {
        Self {
            identifier,
            owner_address: owner_address.into(),
            access_routes,
        }
    }

    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<Self> {
        match msg {
            ExecuteMsg::UpdateAccessRoutes {
                identifier,
                owner_address,
                access_routes,
            } => Self::new(identifier, owner_address, access_routes).to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::UpdateAccessRoutes".to_string(),
            }
            .to_err(),
        }
    }
}

pub fn update_access_routes<'a, T>(
    repository: T,
    info: MessageInfo,
    msg: UpdateAccessRoutesV1,
) -> EntryPointResponse
where
    T: AssetMetaRepository + MessageGatheringService + DepsManager<'a>,
{
    // If the sender is not the specified owner address and the sender is not the admin, they are
    // not authorized to change access routes
    if info.sender.to_string() != msg.owner_address
        && info.sender
            != repository
                .use_deps(|deps| config_read_v2(deps.storage).load())?
                .admin
    {
        return ContractError::Unauthorized {
            explanation:
                "Only the admin or owner of the given access routes can make modifications to them"
                    .to_string(),
        }
        .to_err();
    }
    let mut access_routes = filter_valid_access_routes(msg.access_routes.clone());
    if access_routes.is_empty() {
        return ContractError::generic("No valid access routes were provided").to_err();
    }
    let scope_address = msg.identifier.get_scope_address()?;
    let mut scope_attribute = repository.get_asset(&scope_address)?;
    if let Some(mut target_access_definition) = scope_attribute
        .access_definitions
        .iter()
        .find(|def| &def.owner_address == &msg.owner_address)
        .map(|def| def.to_owned())
    {
        let mut new_access_definitions = scope_attribute
            .access_definitions
            .clone()
            .into_iter()
            .filter(|def| def != &target_access_definition)
            .collect::<Vec<AccessDefinition>>();
        target_access_definition.access_routes.clear();
        target_access_definition
            .access_routes
            .append(&mut access_routes);
        new_access_definitions.push(target_access_definition);
        scope_attribute.access_definitions = new_access_definitions;
        repository.update_attribute(&scope_attribute);
    } else {
        // If no access definitions are established for the given owner address, then the request is
        // invalid and should be rejected
        return ContractError::InvalidAddress {
            address: msg.owner_address,
            explanation: format!("Scope attribute for address [{scope_address}] does not have access definitions for specified owner"),
        }.to_err();
    }
    Response::new()
        .add_attributes(
            EventAttributes::new(EventType::UpdateAccessRoutes)
                .set_asset_type(&scope_attribute.asset_type)
                .set_scope_address(&scope_address)
                .set_new_value(access_routes.len()),
        )
        .add_messages(repository.get_messages())
        .to_ok()
}
