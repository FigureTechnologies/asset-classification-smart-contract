use crate::{
    core::msg::InitMsg,
    util::{aliases::ContractResult, traits::ResultExtensions},
};
use cosmwasm_std::{Addr, Storage};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use cw_storage_plus::{Index, IndexList, IndexedMap, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{asset::AssetDefinition, error::ContractError};

pub static STATE_KEY: &[u8] = b"state";
pub static ASSET_META_KEY: &[u8] = b"asset_meta";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub base_contract_name: String,
    pub admin: Addr,
}
impl State {
    pub fn new(msg: InitMsg, admin: Addr) -> State {
        State {
            base_contract_name: msg.base_contract_name,
            admin,
        }
    }
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, STATE_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, STATE_KEY)
}

/// Boilerplate implementation of indexes for an IndexMap around state.
/// This establishes a unique index on the scope spec address to ensure
/// that saves cannot include duplicate scope specs.
/// If it becomes a requirement in the future that we have duplicate scope specs,
/// we will need to swap to a MultiIndex, and a lot of the lookups in the contract
/// will fall apart
struct AssetDefinitionIndexes<'a> {
    scope_spec: UniqueIndex<'a, String, AssetDefinition>,
}
impl<'a> IndexList<AssetDefinition> for AssetDefinitionIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AssetDefinition>> + '_> {
        let v: Vec<&dyn Index<AssetDefinition>> = vec![&self.scope_spec];
        Box::new(v.into_iter())
    }
}

/// The main entrypoint access for AssetDefinition state.  Establishes an index map for all definitions,
/// allowing the standard save(), load() and iterator functionality. Private access to ensure only
/// helper functions below are used
fn asset_definitions<'a>() -> IndexedMap<'a, &'a [u8], AssetDefinition, AssetDefinitionIndexes<'a>>
{
    let indexes = AssetDefinitionIndexes {
        scope_spec: UniqueIndex::new(
            |d: &AssetDefinition| d.scope_spec_address.clone().to_lowercase(),
            "asset_definitions__scope_spec_address",
        ),
    };
    IndexedMap::new("asset_definitions", indexes)
}

/// Inserts a new asset definition into storage.  
/// If a value already exists, an error will be returned.
/// Note: Asset definitions must contain a unique asset_type value,
/// as well as a unique scope_spec_address.  Either unique constraint being
/// violated will return an error.
pub fn insert_asset_definition(
    storage: &mut dyn Storage,
    definition: &AssetDefinition,
) -> ContractResult<()> {
    let state = asset_definitions();
    let key = &definition.storage_key();
    if let Ok(existing_def) = state.load(storage, key) {
        ContractError::RecordAlreadyExists {
            explanation: format!(
                "unique constraints violated! record exists with asset type [{}] and scope spec address [{}]",
                existing_def.asset_type, existing_def.scope_spec_address
            ),
        }
        .to_err()
    } else {
        // At this point, we know there is no old data available, so we can safely call the replace function and
        // specify None for the old_data param.
        state
            .replace(storage, key, Some(definition), None)
            .map_err(ContractError::Std)
    }
}

/// Replaces an existing asset definition in state with the provided value.
/// If no value exists for the given definition, an error will be returned.
/// Note: IndexedMap (the type asset_definitions() function returns) provides
/// a really nice update() function that allows two branches (one for success and one for failure to find)
/// that seems ideal for this functionality, but it requires a non-reference version of the data to be used.
/// This requires that the provided definition must be cloned, which makes it vastly inefficient compared to
/// this implementation.
pub fn replace_asset_definition(
    storage: &mut dyn Storage,
    definition: &AssetDefinition,
) -> ContractResult<()> {
    let state = asset_definitions();
    let key = &definition.storage_key();
    if let Ok(existing_def) = state.load(storage, key) {
        // The documentation for the save() function in IndexedMap recommends calling replace() directly after
        // loading the data, because it's needed for an update and happens internally anyway
        state
            .replace(storage, key, Some(definition), Some(&existing_def))
            .map_err(ContractError::Std)
    } else {
        ContractError::RecordNotFound {
            explanation: format!(
                "no record exists to update for asset type [{}]",
                &definition.asset_type
            ),
        }
        .to_err()
    }
}

/// Finds an existing asset definition in state by checking against the provided asset type.
pub fn load_asset_definition_by_type<S: Into<String>>(
    storage: &dyn Storage,
    asset_type: S,
) -> ContractResult<AssetDefinition> {
    asset_definitions()
        // Coerce to lowercase to match how stored values are keyed
        .load(storage, asset_type.into().to_lowercase().as_bytes())
        .map_err(ContractError::Std)
}

/// Finds an existing asset definition in state by checking against the provided scope spec address.
pub fn load_asset_definition_by_scope_spec<S: Into<String>>(
    storage: &dyn Storage,
    scope_spec_address: S,
) -> ContractResult<AssetDefinition> {
    // Coerce to lowercase to match how stored values are keyed
    let spec_addr = scope_spec_address.into().to_lowercase();
    if let Some((_, asset_definition)) = asset_definitions()
        .idx
        .scope_spec
        .item(storage, spec_addr.clone())?
    {
        asset_definition.to_ok()
    } else {
        ContractError::RecordNotFound {
            explanation: format!(
                "no asset definition existed for scope spec address {}",
                spec_addr
            ),
        }
        .to_err()
    }
}

#[cfg(test)]
mod tests {
    use provwasm_mocks::mock_dependencies;

    use crate::core::{
        asset::AssetDefinition,
        error::ContractError,
        state::{load_asset_definition_by_scope_spec, load_asset_definition_by_type},
    };

    use super::{insert_asset_definition, replace_asset_definition};

    #[test]
    fn test_insert_asset_definition() {
        let mut deps = mock_dependencies(&[]);
        let def = AssetDefinition::new("heloc", "heloc-scope-spec", vec![]);
        insert_asset_definition(deps.as_mut().storage, &def).expect("insert should work correctly");
        let error = insert_asset_definition(deps.as_mut().storage, &def).unwrap_err();
        match error {
            ContractError::RecordAlreadyExists { explanation } => {
                assert_eq!(
                    "unique constraints violated! record exists with asset type [heloc] and scope spec address [heloc-scope-spec]",
                    explanation,
                    "the proper record type should be returned in the resulting error"
                );
            }
            _ => panic!("unexpected error encountered: {:?}", error),
        }
        let def_with_same_scope_spec = AssetDefinition::new("mortgage", "heloc-scope-spec", vec![]);
        let scope_spec_key_violation_error =
            insert_asset_definition(deps.as_mut().storage, &def_with_same_scope_spec).unwrap_err();
        assert!(
            matches!(scope_spec_key_violation_error, ContractError::Std(_)),
            "violating the scope spec unique key should result in an error, but got incorrect error: {:?}",
            scope_spec_key_violation_error,
        );
        let loaded_asset_definition =
            load_asset_definition_by_type(deps.as_ref().storage, &def.asset_type)
                .expect("asset definition should load correctly");
        assert_eq!(
            loaded_asset_definition, def,
            "the asset definition should be inserted correctly"
        );
    }

    #[test]
    fn test_replace_asset_definition() {
        let mut deps = mock_dependencies(&[]);
        let mut def = AssetDefinition::new("heloc", "heloc-scope-spec", vec![]);
        let error = replace_asset_definition(deps.as_mut().storage, &def).unwrap_err();
        match error {
            ContractError::RecordNotFound { explanation } => {
                assert_eq!(
                    "no record exists to update for asset type [heloc]", explanation,
                    "the proper record type should be returned in the resulting error",
                );
            }
            _ => panic!("unexpected error encountered: {:?}", error),
        };
        insert_asset_definition(deps.as_mut().storage, &def).expect("insert should work correctly");
        def.scope_spec_address = "new-spec-address".to_string();
        replace_asset_definition(deps.as_mut().storage, &def)
            .expect("update should work correctly");
        let loaded_asset_definition =
            load_asset_definition_by_type(deps.as_ref().storage, &def.asset_type)
                .expect("asset definition should load correctly");
        assert_eq!(
            loaded_asset_definition, def,
            "the asset definition should be updated appropriately"
        );
    }

    #[test]
    fn test_load_asset_definition_by_type() {
        let mut deps = mock_dependencies(&[]);
        let heloc = AssetDefinition::new("heloc", "heloc-scope-spec", vec![]);
        let mortgage = AssetDefinition::new("mortgage", "mortgage-scope-spec", vec![]);
        insert_asset_definition(deps.as_mut().storage, &heloc)
            .expect("the heloc definition should insert appropriately");
        insert_asset_definition(deps.as_mut().storage, &mortgage)
            .expect("the mortgage definition should insert appropriately");
        let heloc_from_storage =
            load_asset_definition_by_type(deps.as_ref().storage, &heloc.asset_type)
                .expect("the heloc definition should load appropriately");
        let mortgage_from_storage =
            load_asset_definition_by_type(deps.as_ref().storage, &mortgage.asset_type)
                .expect("the mortgage definition should load appropriately");
        assert_eq!(
            heloc, heloc_from_storage,
            "the heloc definition should be the same after loading from storage"
        );
        assert_eq!(
            mortgage, mortgage_from_storage,
            "the mortgage definition should be the same after loading from storage"
        );
    }

    #[test]
    fn test_load_asset_definition_by_scope_spec() {
        let mut deps = mock_dependencies(&[]);
        let heloc = AssetDefinition::new("heloc", "heloc-scope-spec", vec![]);
        let mortgage = AssetDefinition::new("mortgage", "mortgage-scope-spec", vec![]);
        insert_asset_definition(deps.as_mut().storage, &heloc)
            .expect("the heloc definition should insert appropriately");
        insert_asset_definition(deps.as_mut().storage, &mortgage)
            .expect("the mortgage definition should insert appropriately");
        let heloc_from_storage =
            load_asset_definition_by_scope_spec(deps.as_ref().storage, &heloc.scope_spec_address)
                .expect("the heloc definition should load appropriately");
        let mortgage_from_storage = load_asset_definition_by_scope_spec(
            deps.as_ref().storage,
            &mortgage.scope_spec_address,
        )
        .expect("the mortgage definition should load appropriately");
        assert_eq!(
            heloc, heloc_from_storage,
            "the heloc definition should be the same after loading from storage"
        );
        assert_eq!(
            mortgage, mortgage_from_storage,
            "the mortgage definition should be the same after loading from storage"
        );
    }
}
