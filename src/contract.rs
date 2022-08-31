use crate::core::msg::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};
use crate::execute::add_asset_definition::{add_asset_definition, AddAssetDefinitionV1};
use crate::execute::add_asset_verifier::{add_asset_verifier, AddAssetVerifierV1};
use crate::execute::delete_asset_definition::{delete_asset_definition, DeleteAssetDefinitionV1};
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
use crate::query::query_asset_scope_attribute_by_asset_type::query_asset_scope_attribute_by_asset_type;
use crate::query::query_fee_payments::query_fee_payments;
use crate::query::query_state::query_state;
use crate::query::query_version::query_version;
use crate::service::asset_meta_service::AssetMetaService;
use crate::util::aliases::{AssetResult, DepsC, DepsMutC, EntryPointResponse};
use crate::validation::validate_execute_msg::validate_execute_msg;
use crate::validation::validate_init_msg::validate_init_msg;
use cosmwasm_std::{entry_point, Binary, Env, MessageInfo};

/// The entry point used when an external address instantiates a stored code wasm payload of this
/// contract on the Provenance Blockchain.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `env` An environment object provided by the cosmwasm framework.  Describes the contract's
/// details, as well as blockchain information at the time of the transaction.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` A custom instantiation message defined by this contract for creating the initial
/// configuration used by the contract.
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

/// The entry point used when an external address desires to retrieve information from the contract.
/// Allows access to the internal storage information, as well as scope attributes emitted by the
/// onboarding process.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `_env` An environment object provided by the cosmwasm framework.  Describes the contract's
/// details, as well as blockchain information at the time of the transaction.  Unused by this
/// function, but required by cosmwasm for successful query entrypoint.
/// * `msg` A custom query message enum defined by this contract to allow multiple different results
/// to be determined for this route.
#[entry_point]
pub fn query(deps: DepsC, _env: Env, msg: QueryMsg) -> AssetResult<Binary> {
    match msg {
        QueryMsg::QueryAssetDefinition { asset_type } => query_asset_definition(&deps, &asset_type),
        QueryMsg::QueryAssetDefinitions {} => query_asset_definitions(&deps),
        QueryMsg::QueryAssetScopeAttributes { identifier } => {
            query_asset_scope_attribute(&deps, identifier.to_asset_identifier()?)
        }
        QueryMsg::QueryAssetScopeAttributeForAssetType {
            identifier,
            asset_type,
        } => query_asset_scope_attribute_by_asset_type(
            &deps,
            identifier.to_asset_identifier()?,
            asset_type,
        ),
        QueryMsg::QueryFeePayments {
            identifier,
            asset_type,
        } => query_fee_payments(&deps, identifier.to_asset_identifier()?, &asset_type),
        QueryMsg::QueryState {} => query_state(&deps),
        QueryMsg::QueryVersion {} => query_version(&deps),
    }
}

/// The entry point used when an external address desires to initiate a process defined in the
/// contract.  This defines the primary purposes of this contract, like the onboarding and
/// verification processes, as well as allowing the administrator address to make changes to the
/// contract's internal configuration.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `env` An environment object provided by the cosmwasm framework.  Describes the contract's
/// details, as well as blockchain information at the time of the transaction.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` A custom execution message enum defined by this contract to allow multiple different
/// processes to be defined for the singular execution route entry point allowed by the
/// cosmwasm framework.
#[entry_point]
pub fn execute(deps: DepsMutC, env: Env, info: MessageInfo, msg: ExecuteMsg) -> EntryPointResponse {
    // Ensure the execute message is properly formatted before doing anything
    validate_execute_msg(&msg)?;
    match msg {
        ExecuteMsg::OnboardAsset { .. } => onboard_asset(
            AssetMetaService::new(deps),
            env,
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
        ExecuteMsg::DeleteAssetDefinition { .. } => {
            delete_asset_definition(deps, info, DeleteAssetDefinitionV1::from_execute_msg(msg)?)
        }
    }
}

/// The entry point used when migrating a live contract instance to a new code instance, or to
/// refresh the contract with an existing matching codebase for the purpose of running migration
/// options.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `_env` An environment object provided by the cosmwasm framework.  Describes the contract's
/// details, as well as blockchain information at the time of the transaction.  Unused by this
/// function, but required by cosmwasm for successful migration entrypoint.
/// * msg` A custom migrate message enum defined by this contract to allow multiple different
/// results of invoking the migrate endpoint.
#[entry_point]
pub fn migrate(deps: DepsMutC, _env: Env, msg: MigrateMsg) -> EntryPointResponse {
    match msg {
        MigrateMsg::ContractUpgrade { options } => migrate_contract(deps, options),
    }
}
