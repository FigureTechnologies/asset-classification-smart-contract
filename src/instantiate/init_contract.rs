use crate::core::error::ContractError;
use crate::core::msg::InitMsg;
use crate::core::state::{config, State};
use crate::util::aliases::{ContractResponse, DepsMutC};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Env, MessageInfo, Response};
use provwasm_std::{bind_name, NameBinding};

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
    // Convert the init message into a state value that will drive the contract's future executions
    let state = State::for_init_msg(msg);
    // Store the state by grabbing a mutable instance of the contract configuration
    config(deps.storage).save(&state)?;
    // Bind the request contract name to the contract's address, ensuring it has access to modify
    // its own attributes, and no other entities do
    let bind_name_msg = bind_name(
        &state.contract_name,
        env.contract.address,
        NameBinding::Restricted,
    )?;
    Ok(Response::new()
        .add_message(bind_name_msg)
        .add_attribute("action", "init"))
}
