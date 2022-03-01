use crate::core::error::ContractError;
use crate::core::msg::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};
use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};
use crate::execute::onboard_asset::{onboard_asset, OnboardAssetV1};
use crate::instantiate::init_contract::init_contract;
use crate::util::traits::ResultExtensions;
use crate::util::aliases::{ContractResponse, DepsC, DepsMutC};
use crate::validation::execute_msg::validate_execute_msg;
use crate::validation::init_msg::validate_init_msg;

#[entry_point]
pub fn instantiate(deps: DepsMutC, env: Env, info: MessageInfo, msg: InitMsg) -> ContractResponse {
    // Ensure the init message is properly formatted before doing anything
    validate_init_msg(&msg, &deps.as_ref())?;
    // Execute the core instantiation code
    init_contract(deps, env, info, msg)
}

#[entry_point]
pub fn query(_deps: DepsC, _env: Env, _msg: QueryMsg) -> ContractResponse {
    ContractError::Unimplemented.to_err()
}

#[entry_point]
pub fn execute(deps: DepsMutC, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResponse {
    // Ensure the execute message is properly formatted before doing anything
    validate_execute_msg(&msg)?;
    match msg {
        ExecuteMsg::OnboardAsset { .. } => onboard_asset(
            deps,
            env,
            info,
            OnboardAssetV1::from_execute_msg(msg)?,
        )
    }
}

#[entry_point]
pub fn migrate(_deps: DepsMutC, _env: Env, _msg: MigrateMsg) -> ContractResponse {
    ContractError::Unimplemented.to_err()
}
