use crate::core::error::ContractError;
use crate::core::msg::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};
use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};
use crate::instantiate::init_contract::init_contract;
use crate::util::traits::ResultExtensions;
use crate::validation::init_msg::validate_init_msg;

#[entry_point]
pub fn instantiate(
    deps: DepsMut<ProvenanceQuery>,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    // Ensure the init message is properly formatted before doing anything
    validate_init_msg(&msg, &deps.as_ref())?;
    // Execute the core instantiation code
    init_contract(deps, env, info, msg)
}

#[entry_point]
pub fn query(
    _deps: Deps<ProvenanceQuery>,
    _env: Env,
    _msg: QueryMsg,
) -> Result<Binary, ContractError> {
    ContractError::Unimplemented.to_err()
}

#[entry_point]
pub fn execute(
    _deps: DepsMut<ProvenanceQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    ContractError::Unimplemented.to_err()
}

#[entry_point]
pub fn migrate(
    _deps: DepsMut<ProvenanceQuery>,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    ContractError::Unimplemented.to_err()
}
