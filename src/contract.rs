use crate::core::error::ContractError;
use crate::core::msg::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};
use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

#[entry_point]
pub fn instantiate(
    _deps: DepsMut<ProvenanceQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: InitMsg,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    ContractError::Unimplemented.to_result()
}

#[entry_point]
pub fn query(
    _deps: Deps<ProvenanceQuery>,
    _env: Env,
    _msg: QueryMsg,
) -> Result<Binary, ContractError> {
    ContractError::Unimplemented.to_result()
}

#[entry_point]
pub fn execute(
    _deps: DepsMut<ProvenanceQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    ContractError::Unimplemented.to_result()
}

#[entry_point]
pub fn migrate(
    _deps: DepsMut<ProvenanceQuery>,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    ContractError::Unimplemented.to_result()
}
