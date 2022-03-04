use crate::core::error::ContractError;
use crate::core::msg::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};
use crate::execute::add_asset_definition::{add_asset_definition, AddAssetDefinitionV1};
use crate::execute::add_asset_validator::{add_asset_validator, AddAssetValidatorV1};
use crate::execute::onboard_asset::{onboard_asset, OnboardAssetV1};
use crate::execute::toggle_asset_definition::{toggle_asset_definition, ToggleAssetDefinitionV1};
use crate::execute::update_asset_definition::{update_asset_definition, UpdateAssetDefinitionV1};
use crate::execute::update_asset_validator::{update_asset_validator, UpdateAssetValidatorV1};
use crate::execute::validate_asset::{validate_asset, ValidateAssetV1};
use crate::instantiate::init_contract::init_contract;
use crate::query::query_asset_definition::query_asset_definition;
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
pub fn query(deps: DepsC, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::QueryAssetDefinition { asset_type } => query_asset_definition(&deps, asset_type),
    }
}

#[entry_point]
pub fn execute(deps: DepsMutC, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResponse {
    // Ensure the execute message is properly formatted before doing anything
    validate_execute_msg(&msg, &deps.as_ref())?;
    match msg {
        ExecuteMsg::OnboardAsset { .. } => {
            onboard_asset(deps, env, info, OnboardAssetV1::from_execute_msg(msg)?)
        }
        ExecuteMsg::ValidateAsset { .. } => {
            validate_asset(deps, env, info, ValidateAssetV1::from_execute_msg(msg)?)
        }
        ExecuteMsg::AddAssetDefinition { .. } => add_asset_definition(
            deps,
            env,
            info,
            AddAssetDefinitionV1::from_execute_msg(msg)?,
        ),
        ExecuteMsg::UpdateAssetDefinition { .. } => {
            update_asset_definition(deps, info, UpdateAssetDefinitionV1::from_execute_msg(msg)?)
        }
        ExecuteMsg::ToggleAssetDefinition { .. } => {
            toggle_asset_definition(deps, info, ToggleAssetDefinitionV1::from_execute_msg(msg)?)
        }
        ExecuteMsg::AddAssetValidator { .. } => {
            add_asset_validator(deps, info, AddAssetValidatorV1::from_execute_msg(msg)?)
        }
        ExecuteMsg::UpdateAssetValidator { .. } => {
            update_asset_validator(deps, info, UpdateAssetValidatorV1::from_execute_msg(msg)?)
        }
    }
}

#[entry_point]
pub fn migrate(_deps: DepsMutC, _env: Env, _msg: MigrateMsg) -> ContractResponse {
    ContractError::Unimplemented.to_err()
}
