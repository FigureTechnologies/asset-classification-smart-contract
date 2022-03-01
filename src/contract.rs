use crate::core::error::ContractError;
use crate::core::msg::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};
use crate::execute::onboard_asset::{onboard_asset, OnboardAssetV1};
use crate::execute::validate_asset::{validate_asset, ValidateAssetV1};
use crate::instantiate::init_contract::init_contract;
use crate::util::aliases::{ContractResponse, ContractResult, DepsC, DepsMutC};
use crate::util::traits::ResultExtensions;
use crate::validation::validate_execute_msg::validate_execute_msg;
use crate::validation::validate_init_msg::validate_init_msg;
use cosmwasm_std::{entry_point, Binary, Env, MessageInfo};

#[entry_point]
pub fn instantiate(deps: DepsMutC, env: Env, info: MessageInfo, msg: InitMsg) -> ContractResponse {
    // Ensure the init message is properly formatted before doing anything
    validate_init_msg(&msg, &deps.as_ref())?;
    // Execute the core instantiation code
    init_contract(deps, env, info, msg)
}

#[entry_point]
pub fn query(_deps: DepsC, _env: Env, _msg: QueryMsg) -> ContractResult<Binary> {
    ContractError::Unimplemented.to_err()
}

#[entry_point]
pub fn execute(deps: DepsMutC, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResponse {
    // Ensure the execute message is properly formatted before doing anything
    validate_execute_msg(&msg)?;
    match msg {
        ExecuteMsg::OnboardAsset { .. } => {
            onboard_asset(deps, env, info, OnboardAssetV1::from_execute_msg(msg)?)
        }
        ExecuteMsg::ValidateAsset { .. } => {
            validate_asset(deps, env, info, ValidateAssetV1::from_execute_msg(msg)?)
        }
    }
}

#[entry_point]
pub fn migrate(_deps: DepsMutC, _env: Env, _msg: MigrateMsg) -> ContractResponse {
    ContractError::Unimplemented.to_err()
}
