use crate::core::msg::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};
use crate::execute::add_asset_definition::{add_asset_definition, AddAssetDefinitionV1};
use crate::execute::add_asset_verifier::{add_asset_verifier, AddAssetVerifierV1};
use crate::execute::bind_contract_alias::{bind_contract_alias, BindContractAliasV1};
use crate::execute::onboard_asset::{onboard_asset, OnboardAssetV1};
use crate::execute::toggle_asset_definition::{toggle_asset_definition, ToggleAssetDefinitionV1};
use crate::execute::update_access_routes::{update_access_routes, UpdateAccessRoutesV1};
use crate::execute::update_asset_definition::{update_asset_definition, UpdateAssetDefinitionV1};
use crate::execute::update_asset_verifier::{update_asset_verifier, UpdateAssetVerifierV1};
use crate::execute::verify_asset::{verify_asset, VerifyAssetV1};
use crate::instantiate::init_contract::init_contract;
use crate::migrate::migrate_contract::migrate_contract;
use crate::query::query_asset_definition::query_asset_definition;
use crate::query::query_asset_definitions::query_asset_definitions;
use crate::query::query_asset_scope_attribute::query_asset_scope_attribute;
use crate::query::query_state::query_state;
use crate::query::query_version::query_version;
use crate::service::asset_meta_service::AssetMetaService;
use crate::util::aliases::{AssetResult, DepsC, DepsMutC, EntryPointResponse};
use crate::validation::validate_execute_msg::validate_execute_msg;
use crate::validation::validate_init_msg::validate_init_msg;
use cosmwasm_std::{entry_point, Binary, Env, MessageInfo};

#[entry_point]
pub fn instantiate(
    deps: DepsMutC,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> EntryPointResponse {
    // Ensure the init message is properly formatted before doing anything
    validate_init_msg(&msg)?;
    // Execute the core instantiation code
    init_contract(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: DepsC, _env: Env, msg: QueryMsg) -> AssetResult<Binary> {
    match msg {
        QueryMsg::QueryAssetDefinition { qualifier } => query_asset_definition(&deps, qualifier),
        QueryMsg::QueryAssetDefinitions {} => query_asset_definitions(&deps),
        QueryMsg::QueryAssetScopeAttribute { identifier } => {
            query_asset_scope_attribute(&deps, identifier)
        }
        QueryMsg::QueryState {} => query_state(&deps),
        QueryMsg::QueryVersion {} => query_version(&deps),
    }
}

#[entry_point]
pub fn execute(deps: DepsMutC, env: Env, info: MessageInfo, msg: ExecuteMsg) -> EntryPointResponse {
    // Ensure the execute message is properly formatted before doing anything
    validate_execute_msg(&msg)?;
    match msg {
        ExecuteMsg::OnboardAsset { .. } => onboard_asset(
            AssetMetaService::new(deps),
            info,
            OnboardAssetV1::from_execute_msg(msg)?,
        ),
        ExecuteMsg::VerifyAsset { .. } => verify_asset(
            AssetMetaService::new(deps),
            info,
            VerifyAssetV1::from_execute_msg(msg)?,
        ),
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
        ExecuteMsg::AddAssetVerifier { .. } => {
            add_asset_verifier(deps, info, AddAssetVerifierV1::from_execute_msg(msg)?)
        }
        ExecuteMsg::UpdateAssetVerifier { .. } => {
            update_asset_verifier(deps, info, UpdateAssetVerifierV1::from_execute_msg(msg)?)
        }
        ExecuteMsg::UpdateAccessRoutes { .. } => update_access_routes(
            AssetMetaService::new(deps),
            info,
            UpdateAccessRoutesV1::from_execute_msg(msg)?,
        ),
        ExecuteMsg::BindContractAlias { .. } => {
            bind_contract_alias(deps, env, info, BindContractAliasV1::from_execute_msg(msg)?)
        }
    }
}

#[entry_point]
pub fn migrate(deps: DepsMutC, _env: Env, msg: MigrateMsg) -> EntryPointResponse {
    match msg {
        MigrateMsg::ContractUpgrade {} => migrate_contract(deps),
    }
}
