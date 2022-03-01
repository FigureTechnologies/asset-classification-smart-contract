use crate::core::error::ContractError;
use crate::core::msg::InitMsg;
use crate::core::state::{asset_state, config, State};
use crate::util::aliases::{ContractResponse, DepsMutC};
use crate::util::functions::generate_asset_attribute_name;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{CosmosMsg, Env, MessageInfo, Response};
use provwasm_std::{bind_name, NameBinding, ProvenanceMsg};

pub fn init_contract(
    deps: DepsMutC,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> ContractResponse {
    // Ensure that funds were not improperly provided
    if !info.funds.is_empty() {
        return ContractError::InvalidFunds(
            "Funds should not be provided for contract instantiation".to_string(),
        )
        .to_err();
    }
    let mut messages: Vec<CosmosMsg<ProvenanceMsg>> = vec![];
    // Note: This vector can remain empty on instantiation, and future executions by the admin can
    // append new definitions. When no definitions are supplied, this contract will not be able to
    // take execution input until they are
    for asset_definition in msg.asset_definitions.iter() {
        // Create a new state storage for the provided asset definition
        asset_state(deps.storage, &asset_definition.asset_type).save(asset_definition)?;
        messages.push(bind_name(
            generate_asset_attribute_name(&asset_definition.asset_type, &msg.base_contract_name),
            env.contract.address.clone(),
            NameBinding::Restricted,
        )?);
    }
    // Convert the init message into a state value that will drive the contract's future executions
    let state = State::new(msg, info.sender);
    // Store the state by grabbing a mutable instance of the contract configuration
    config(deps.storage).save(&state)?;
    Ok(Response::new()
        .add_messages(messages)
        //.add_message(bind_name_msg)
        .add_attribute("action", "init"))
}
