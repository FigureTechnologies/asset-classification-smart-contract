use crate::core::types::asset_definition::AssetDefinitionV2;
use crate::core::types::asset_qualifier::AssetQualifier;
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::{
    core::msg::InitMsg,
    util::{
        aliases::AssetResult,
        traits::{OptionExtensions, ResultExtensions},
    },
};
use cosmwasm_std::{Addr, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use cw_storage_plus::{Index, IndexList, IndexedMap, UniqueIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::error::ContractError;

pub static STATE_V2_KEY: &[u8] = b"state_v2";
pub static ASSET_META_KEY: &[u8] = b"asset_meta";
pub static LATEST_VERIFIER_DETAIL_KEY: &[u8] = b"latest_verifier_detail";

/// Stores the main configurations for the contract internally.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateV2 {
    /// The root name from which all asset names branch.  All sub-names specified in the [AssetDefinitions](super::types::access_definition::AccessDefinition)
    /// will use this value as their parent name.
    pub base_contract_name: String,
    /// The Provenance Blockchain bech32 address that maintains primary control over the contract.
    /// This address is derived from the sender of the initial contract instantiation, and is the
    /// only address that can access administrative execution routes in the contract.  It can be
    /// changed during migrations.
    pub admin: Addr,
    /// A boolean value allowing for less restrictions to be placed on certain functionalities
    /// across the contract's execution processes.  Notably, this disables a check during the
    /// onboarding process to determine if onboarded scopes include underlying record values.  This
    /// should never be set to true in a mainnet environment.
    pub is_test: bool,
}
impl StateV2 {
    /// Constructs a new instance of this struct for the instantiation process.
    ///
    /// # Parameters
    ///
    /// * `msg` The message submitted by the instantiating account.
    /// * `admin` The Provenance Blockchain bech32 address of the administrator account for the contract.
    /// The sender's address is automatically used for this, and they alone will have access to
    /// change the admin address to a different one via migrations.
    pub fn new(msg: InitMsg, admin: Addr) -> StateV2 {
        StateV2 {
            base_contract_name: msg.base_contract_name,
            admin,
            is_test: msg.is_test.unwrap_or(false),
        }
    }
}

/// Fetches a mutable reference to the storage from a [DepsMutC](crate::util::aliases::DepsMutC).
///
/// # Parameters
///
/// * `storage` A mutable reference to the contract's internal storage.
pub fn config_v2(storage: &mut dyn Storage) -> Singleton<StateV2> {
    singleton(storage, STATE_V2_KEY)
}

/// Fetches a read-only cosmwasm storage singleton instance for loading the contract's state.
///
/// # Parameters
///
/// * `storage` A reference to the storage from a [DepsC](crate::util::aliases::DepsC).
pub fn config_read_v2(storage: &dyn Storage) -> ReadonlySingleton<StateV2> {
    singleton_read(storage, STATE_V2_KEY)
}

/// Boilerplate implementation of indexes for an IndexMap around state.
/// This establishes a unique index on the scope spec address to ensure
/// that saves cannot include duplicate scope specs.
/// If it becomes a requirement in the future that we have duplicate scope specs,
/// we will need to swap to a MultiIndex, and a lot of the lookups in the contract
/// will fall apart.
pub struct AssetDefinitionIndexesV2<'a> {
    scope_spec: UniqueIndex<'a, String, AssetDefinitionV2>,
}
impl<'a> IndexList<AssetDefinitionV2> for AssetDefinitionIndexesV2<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AssetDefinitionV2>> + '_> {
        let v: Vec<&dyn Index<AssetDefinitionV2>> = vec![&self.scope_spec];
        Box::new(v.into_iter())
    }
}

/// The main entrypoint access for [AssetDefinitionV2](super::types::asset_definition::AssetDefinitionV2) state.
/// Establishes an index map for all definitions, allowing the standard save(), load() and iterator
/// functionality. Private access to ensure only helper functions below are used.
pub fn asset_definitions_v2<'a>(
) -> IndexedMap<'a, &'a [u8], AssetDefinitionV2, AssetDefinitionIndexesV2<'a>> {
    let indexes = AssetDefinitionIndexesV2 {
        scope_spec: UniqueIndex::new(
            |d: &AssetDefinitionV2| d.scope_spec_address.clone().to_lowercase(),
            "asset_definitions_v2__scope_spec_address",
        ),
    };
    IndexedMap::new("asset_definitions_v2", indexes)
}

/// Inserts a new asset definition into storage. If a value already exists, an error will be returned.
/// Note: Asset definitions must contain a unique [asset_type](super::types::asset_definition::AssetDefinitionV2::asset_type)
/// value, as well as a unique [scope_spec_address](super::types::asset_definition::AssetDefinitionV2::scope_spec_address).
/// Either unique constraint being violated will return an error.
///
/// # Parameters
///
/// * `storage` A mutable reference to the contract's internal storage.
pub fn insert_asset_definition_v2(
    storage: &mut dyn Storage,
    definition: &AssetDefinitionV2,
) -> AssetResult<()> {
    let state = asset_definitions_v2();
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
            .replace(storage, key, definition.to_some(), None)
            .map_err(ContractError::Std)
    }
}

/// Replaces an existing asset definition in state with the provided value.  If no value exists for
/// the given definition, an error will be returned.  Note: IndexedMap (the type [asset_definitions_v2](self::asset_definitions_v2)
/// function returns) provides a really nice update() function that allows two branches (one for
/// success and one for failure to find) that seems ideal for this functionality, but it requires a
/// non-reference version of the data to be used. This requires that the provided definition must be
/// cloned, which makes it vastly inefficient compared to this implementation.
///
/// # Parameters
///
/// * `storage` A mutable reference to the contract's internal storage.
/// * `definition` The asset definition to replace by matching on its [asset_type](super::types::asset_definition::AssetDefinitionV2::asset_type)
/// property.
pub fn replace_asset_definition_v2(
    storage: &mut dyn Storage,
    definition: &AssetDefinitionV2,
) -> AssetResult<()> {
    let state = asset_definitions_v2();
    let key = &definition.storage_key();
    if let Ok(existing_def) = state.load(storage, key) {
        // The documentation for the save() function in IndexedMap recommends calling replace() directly after
        // loading the data, because it's needed for an update and happens internally anyway
        state
            .replace(
                storage,
                key,
                definition.to_some(),
                (&existing_def).to_some(),
            )
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

/// Finds an existing asset definition in state by checking against the provided asset type,
/// returning an Option that reflects whether or not the definition exists.
///
/// # Parameters
///
/// * `storage` A reference to the contract's internal storage.
/// * `asset_type` The unique name key [asset_type](super::types::asset_definition::AssetDefinitionV2::asset_type)
/// for the requested asset definition.
pub fn may_load_asset_definition_v2_by_type<S: Into<String>>(
    storage: &dyn Storage,
    asset_type: S,
) -> AssetResult<Option<AssetDefinitionV2>> {
    asset_definitions_v2()
        // Coerce to lowercase to match how stored values are keyed
        .may_load(storage, asset_type.into().to_lowercase().as_bytes())
        .map_err(ContractError::Std)
}

/// Finds an existing asset definition by asset type, or returns an error if no definition is found.
///
/// # Parameters
///
/// * `storage` A reference to the contract's internal storage.
/// * `asset_type` The unique name key [asset_type](super::types::asset_definition::AssetDefinitionV2::asset_type)
/// for the requested asset definition.
pub fn load_asset_definition_v2_by_type<S: Into<String>>(
    storage: &dyn Storage,
    asset_type: S,
) -> AssetResult<AssetDefinitionV2> {
    let asset_type = asset_type.into();
    if let Some(asset_definition) = may_load_asset_definition_v2_by_type(storage, &asset_type)? {
        asset_definition.to_ok()
    } else {
        ContractError::RecordNotFound {
            explanation: format!("no asset definition existed for asset type {}", asset_type,),
        }
        .to_err()
    }
}

/// Finds an existing asset definition in state by checking against the provided scope spec address,
/// returning an Option that reflects whether or not the definition exists.
///
/// # Parameters
///
/// * `storage` A reference to the contract's internal storage.
/// * `scope_spec_address` The unique address key [scope_spec_address](super::types::asset_definition::AssetDefinitionV2::scope_spec_address)
/// for the requested asset definition.
pub fn may_load_asset_definition_v2_by_scope_spec<S: Into<String>>(
    storage: &dyn Storage,
    scope_spec_address: S,
) -> AssetResult<Option<AssetDefinitionV2>> {
    // Coerce to lowercase to match how stored values are keyed
    let spec_addr = scope_spec_address.into().to_lowercase();
    asset_definitions_v2()
        .idx
        .scope_spec
        .item(storage, spec_addr)
        .map(|option| option.map(|(_, def)| def))
        .map_err(ContractError::Std)
}

/// Finds an existing asset definition by scope spec address, or returns an error if no definition is
/// found.
///
/// # Parameters
///
/// * `storage` A reference to the contract's internal storage.
/// * `scope_spec_address` The unique address key [scope_spec_address](super::types::asset_definition::AssetDefinitionV2::scope_spec_address)
/// for the requested asset definition.
pub fn load_asset_definition_v2_by_scope_spec<S: Into<String>>(
    storage: &dyn Storage,
    scope_spec_address: S,
) -> AssetResult<AssetDefinitionV2> {
    let scope_spec_address = scope_spec_address.into();
    if let Some(asset_definition) =
        may_load_asset_definition_v2_by_scope_spec(storage, &scope_spec_address)?
    {
        asset_definition.to_ok()
    } else {
        ContractError::RecordNotFound {
            explanation: format!(
                "no asset definition existed for scope spec address {}",
                scope_spec_address
            ),
        }
        .to_err()
    }
}

/// Attempts to delete an existing asset definition by asset type.  Returns an error if the
/// definition does not exist or if the deletion fails.  Returns the asset type of the deleted
/// definition on a successful deletion.
///
/// # Parameters
///
/// * `storage` A mutable reference to the contract's internal storage.
/// * `qualifier` An asset qualifier that can identify the [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
/// to delete.
pub fn delete_asset_definition_v2_by_qualifier(
    storage: &mut dyn Storage,
    qualifier: &AssetQualifier,
) -> AssetResult<String> {
    let existing_asset_type = match qualifier {
        AssetQualifier::AssetType(asset_type) => {
            load_asset_definition_v2_by_type(storage, asset_type)
        }
        AssetQualifier::ScopeSpecAddress(scope_spec_address) => {
            load_asset_definition_v2_by_scope_spec(storage, scope_spec_address)
        }
    }?
    .asset_type;
    asset_definitions_v2().remove(storage, existing_asset_type.to_lowercase().as_bytes())?;
    Ok(existing_asset_type)
}

/// Fetches a mutable reference to the latest verifier detail storage from a [DepsMutC](crate::util::aliases::DepsMutC).
/// This storage exists for the purpose of maintaining an accurate record of the [VerifierDetailV2](crate::core::types::verifier_detail::VerifierDetailV2)
/// each scope has when it is onboarded.  As those values can technically change between the time at
/// which an asset is onboarded versus when it is verified, it is important that the values are
/// stored until verification is complete in order to generate the correct fees to match the
/// onboarding cost that is held in contract escrow during onboarding.
///
/// The other reason for this value being stored in contract storage versus on the scope attribute
/// itself is because attributes written to the Provenance Blockchain have a set size limit.  Storing
/// the entire [VerifierDetailV2](crate::core::types::verifier_detail::VerifierDetailV2) on the
/// attribute is a massive bloat to its size and can cause rejected attribute writes in certain
/// circumstances.
///
/// # Parameters
///
/// * `storage` A mutable reference to the contract's internal storage.
pub fn latest_verifier_detail_store(storage: &mut dyn Storage) -> Bucket<VerifierDetailV2> {
    bucket(storage, LATEST_VERIFIER_DETAIL_KEY)
}

/// Fetches a read-only cosmwasm storage bucket instance for loading verifier details for scope
/// attributes.  This storage exists for the purpose of maintaining
/// an accurate record of the VerifierDetailV2 each scope has when it is onboarded.  As those values
/// can technically change between the time at which an asset is onboarded versus when it is verified,
/// it is important that the values are stored until verification is complete in order to generate
/// the correct fees to match the onboarding cost that is held in contract escrow during onboarding.
///
/// The other reason for this value being stored in contract storage versus on the scope attribute
/// itself is because attributes written to the Provenance Blockchain have a set size limit.  Storing
/// the entire [VerifierDetailV2](crate::core::types::verifier_detail::VerifierDetailV2) on the
/// attribute is a massive bloat to its size and can cause rejected attribute writes in certain
/// circumstances.
///
/// # Parameters
///
/// * `storage` A reference to the storage from a [DepsC](crate::util::aliases::DepsC) or
/// [DepsMutC](crate::util::aliases::DepsMutC).
pub fn latest_verifier_detail_store_ro(storage: &dyn Storage) -> ReadonlyBucket<VerifierDetailV2> {
    bucket_read(storage, LATEST_VERIFIER_DETAIL_KEY)
}

/// Inserts a verifier detail into the contract's storage, leveraging the
/// [latest_verifier_detail_store](self::latest_verifier_detail_store) function.
///
/// # Parameters
///
/// * `storage` A mutable reference to the contract's internal storage.
/// * `scope_address` The scope address of the asset for which to store the detail.  This serves
/// as the primary key for this bucket.
/// * `verifier_detail` The verifier detail to store.  This represents the latest version of the
/// verifier detail that was used when the asset was onboarded into the contract for verification.
pub fn insert_latest_verifier_detail<S: Into<String>>(
    storage: &mut dyn Storage,
    scope_address: S,
    verifier_detail: &VerifierDetailV2,
) -> AssetResult<()> {
    latest_verifier_detail_store(storage)
        .save(scope_address.into().as_bytes(), verifier_detail)
        .map_err(|err| ContractError::Std(err))
}

/// Deletes an existing [VerifierDetailV2](crate::core::types::verifier_detail::VerifierDetailV2)
/// from the contract's internal latest verifier detail storage.  This leverages the
/// [latest_verifier_detail_store](self::latest_verifier_detail_store) function.  This function is
/// important, as contract storage should not retain any bloat.  All verifier details should be
/// deleted after a verification has been completed successfully, as they are a historical record
/// that is not relevant.
///
/// # Parameters
///
/// * `storage` A mutable reference to the contract's internal storage.
/// * `scope_address` The scope address of the asset for which to store the detail.  This serves
/// as the primary key for this bucket.
pub fn delete_latest_verifier_detail<S: Into<String>>(
    storage: &mut dyn Storage,
    scope_address: S,
) -> AssetResult<()> {
    latest_verifier_detail_store(storage)
        .remove(scope_address.into().as_bytes())
        .to_ok()
}

#[cfg(test)]
mod tests {
    use provwasm_mocks::mock_dependencies;

    use crate::core::error::ContractError;
    use crate::core::state::{
        delete_asset_definition_v2_by_qualifier, delete_latest_verifier_detail,
        insert_asset_definition_v2, insert_latest_verifier_detail, latest_verifier_detail_store_ro,
        load_asset_definition_v2_by_scope_spec, load_asset_definition_v2_by_type,
        may_load_asset_definition_v2_by_scope_spec, may_load_asset_definition_v2_by_type,
        replace_asset_definition_v2,
    };
    use crate::core::types::asset_definition::AssetDefinitionV2;
    use crate::core::types::asset_qualifier::AssetQualifier;
    use crate::testutil::test_utilities::get_default_verifier_detail;

    #[test]
    fn test_insert_asset_definition() {
        let mut deps = mock_dependencies(&[]);
        let def = AssetDefinitionV2::new("heloc", "heloc-scope-spec", vec![]);
        insert_asset_definition_v2(deps.as_mut().storage, &def)
            .expect("insert should work correctly");
        let error = insert_asset_definition_v2(deps.as_mut().storage, &def).unwrap_err();
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
        let def_with_same_scope_spec =
            AssetDefinitionV2::new("mortgage", "heloc-scope-spec", vec![]);
        let scope_spec_key_violation_error =
            insert_asset_definition_v2(deps.as_mut().storage, &def_with_same_scope_spec)
                .unwrap_err();
        assert!(
            matches!(scope_spec_key_violation_error, ContractError::Std(_)),
            "violating the scope spec unique key should result in an error, but got incorrect error: {:?}",
            scope_spec_key_violation_error,
        );
        let loaded_asset_definition =
            load_asset_definition_v2_by_type(deps.as_ref().storage, &def.asset_type)
                .expect("asset definition should load without error");
        assert_eq!(
            loaded_asset_definition, def,
            "the asset definition should be inserted correctly"
        );
    }

    #[test]
    fn test_replace_asset_definition() {
        let mut deps = mock_dependencies(&[]);
        let mut def = AssetDefinitionV2::new("heloc", "heloc-scope-spec", vec![]);
        let error = replace_asset_definition_v2(deps.as_mut().storage, &def).unwrap_err();
        match error {
            ContractError::RecordNotFound { explanation } => {
                assert_eq!(
                    "no record exists to update for asset type [heloc]", explanation,
                    "the proper record type should be returned in the resulting error",
                );
            }
            _ => panic!("unexpected error encountered: {:?}", error),
        };
        insert_asset_definition_v2(deps.as_mut().storage, &def)
            .expect("insert should work correctly");
        def.scope_spec_address = "new-spec-address".to_string();
        replace_asset_definition_v2(deps.as_mut().storage, &def)
            .expect("update should work correctly");
        let loaded_asset_definition =
            load_asset_definition_v2_by_type(deps.as_ref().storage, &def.asset_type)
                .expect("asset definition should load without error");
        assert_eq!(
            loaded_asset_definition, def,
            "the asset definition should be updated appropriately"
        );
    }

    #[test]
    fn test_may_load_asset_definition_by_type() {
        let mut deps = mock_dependencies(&[]);
        let heloc = AssetDefinitionV2::new("heloc", "heloc-scope-spec", vec![]);
        insert_asset_definition_v2(deps.as_mut().storage, &heloc)
            .expect("the heloc definition should insert without error");
        assert!(
            may_load_asset_definition_v2_by_type(deps.as_ref().storage, "not-heloc")
                .expect("may load asset definition by type should execute without error")
                .is_none(),
            "expected the missing asset definition to return an empty Option",
        );
        assert_eq!(
            may_load_asset_definition_v2_by_type(deps.as_ref().storage, &heloc.asset_type)
            .expect("may load asset definition by type should execute without error")
            .expect("expected the asset definition loaded by a populated type to be present"),
            heloc,
            "expected the loaded asset definition to equate to the original value that was inserted",
        );
    }

    #[test]
    fn test_load_asset_definition_by_type() {
        let mut deps = mock_dependencies(&[]);
        let heloc = AssetDefinitionV2::new("heloc", "heloc-scope-spec", vec![]);
        let mortgage = AssetDefinitionV2::new("mortgage", "mortgage-scope-spec", vec![]);
        insert_asset_definition_v2(deps.as_mut().storage, &heloc)
            .expect("the heloc definition should insert appropriately");
        insert_asset_definition_v2(deps.as_mut().storage, &mortgage)
            .expect("the mortgage definition should insert appropriately");
        let heloc_from_storage =
            load_asset_definition_v2_by_type(deps.as_ref().storage, &heloc.asset_type)
                .expect("the heloc definition should load without error");
        let mortgage_from_storage =
            load_asset_definition_v2_by_type(deps.as_ref().storage, &mortgage.asset_type)
                .expect("the mortgage definition should load without error");
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
    fn test_may_load_asset_definition_by_scope_spec_address() {
        let mut deps = mock_dependencies(&[]);
        let heloc = AssetDefinitionV2::new("heloc", "heloc-scope-spec", vec![]);
        insert_asset_definition_v2(deps.as_mut().storage, &heloc)
            .expect("the heloc definition should insert without error");
        assert!(
            may_load_asset_definition_v2_by_scope_spec(
                deps.as_ref().storage,
                "not-heloc-scope-spec"
            )
            .expect("may load asset definition by scope spec should execute without error")
            .is_none(),
            "expected the missing asset definition to return an empty Option",
        );
        assert_eq!(
            may_load_asset_definition_v2_by_scope_spec(deps.as_ref().storage, &heloc.scope_spec_address)
            .expect("may load asset definition by scope spec should execute without error")
            .expect("expected the asset definition loaded by a populated scope spec address to be present"),
            heloc,
            "expected the loaded asset definition to equate to the original value that was inserted",
        );
    }

    #[test]
    fn test_load_asset_definition_by_scope_spec() {
        let mut deps = mock_dependencies(&[]);
        let heloc = AssetDefinitionV2::new("heloc", "heloc-scope-spec", vec![]);
        let mortgage = AssetDefinitionV2::new("mortgage", "mortgage-scope-spec", vec![]);
        insert_asset_definition_v2(deps.as_mut().storage, &heloc)
            .expect("the heloc definition should insert appropriately");
        insert_asset_definition_v2(deps.as_mut().storage, &mortgage)
            .expect("the mortgage definition should insert appropriately");
        let heloc_from_storage = load_asset_definition_v2_by_scope_spec(
            deps.as_ref().storage,
            &heloc.scope_spec_address,
        )
        .expect("the heloc definition should load without error");
        let mortgage_from_storage = load_asset_definition_v2_by_scope_spec(
            deps.as_ref().storage,
            &mortgage.scope_spec_address,
        )
        .expect("the mortgage definition should load without error");
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
    fn test_delete_asset_definition_by_type() {
        let mut deps = mock_dependencies(&[]);
        let def = AssetDefinitionV2::new("heloc", "heloc-scope-spec", vec![]);
        insert_asset_definition_v2(deps.as_mut().storage, &def)
            .expect("expected the asset definition to be stored without error");
        assert_eq!(
            load_asset_definition_v2_by_type(deps.as_ref().storage, &def.asset_type)
                .expect("expected the load to succeed"),
            def,
            "sanity check: asset definition should be accessible by asset type",
        );
        assert_eq!(
            load_asset_definition_v2_by_scope_spec(deps.as_ref().storage, &def.scope_spec_address)
                .expect("expected the load to succeed"),
            def,
            "sanity check: asset definition should be accessible by scope spec address",
        );
        delete_asset_definition_v2_by_qualifier(
            deps.as_mut().storage,
            &AssetQualifier::asset_type(&def.asset_type),
        )
        .expect("expected the deletion to succeed");
        let err = load_asset_definition_v2_by_type(deps.as_ref().storage, &def.asset_type)
            .expect_err(
                "expected an error to occur when attempting to load the deleted definition",
            );
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected the record not found error to occur when attempting to load by asset type, but got: {:?}",
            err,
        );
        let err =
            load_asset_definition_v2_by_scope_spec(deps.as_ref().storage, &def.scope_spec_address)
                .expect_err(
                    "expected an error to occur when attempting to load the deleted definition",
                );
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected the record not found error to occur when attempting to load by scope spec address, but got: {:?}",
            err,
        );
        insert_asset_definition_v2(deps.as_mut().storage, &def)
            .expect("expected the asset definition to be stored again without error");
        assert_eq!(
            load_asset_definition_v2_by_type(deps.as_ref().storage, &def.asset_type)
                .expect("expected the load to succeed"),
            def,
            "the definition should be once again successfully attainable by asset type",
        );
        assert_eq!(
            load_asset_definition_v2_by_scope_spec(deps.as_ref().storage, &def.scope_spec_address)
                .expect("expected the load to succeed"),
            def,
            "the definition should be once again successfully attainable by scope spec address",
        );
    }

    #[test]
    fn test_delete_nonexistent_asset_definition_by_type_failure() {
        let mut deps = mock_dependencies(&[]);
        let err = delete_asset_definition_v2_by_qualifier(
            deps.as_mut().storage,
            &AssetQualifier::asset_type("fake-type"),
        )
        .expect_err("expected an error to occur when attempting to delete a missing asset type");
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected a record not found error to be emitted when the definition does not exist, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_delete_asset_definition_by_scope_spec_address() {
        let mut deps = mock_dependencies(&[]);
        let def = AssetDefinitionV2::new("heloc", "heloc-scope-spec", vec![]);
        insert_asset_definition_v2(deps.as_mut().storage, &def)
            .expect("expected the asset definition to be stored without error");
        assert_eq!(
            load_asset_definition_v2_by_type(deps.as_ref().storage, &def.asset_type)
                .expect("expected the load to succeed"),
            def,
            "sanity check: asset definition should be accessible by asset type",
        );
        assert_eq!(
            load_asset_definition_v2_by_scope_spec(deps.as_ref().storage, &def.scope_spec_address)
                .expect("expected the load to succeed"),
            def,
            "sanity check: asset definition should be accessible by scope spec address",
        );
        delete_asset_definition_v2_by_qualifier(
            deps.as_mut().storage,
            &AssetQualifier::scope_spec_address(&def.scope_spec_address),
        )
        .expect("expected the deletion to succeed");
        let err = load_asset_definition_v2_by_type(deps.as_ref().storage, &def.asset_type)
            .expect_err(
                "expected an error to occur when attempting to load the deleted definition",
            );
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected the record not found error to occur when attempting to load by asset type, but got: {:?}",
            err,
        );
        let err =
            load_asset_definition_v2_by_scope_spec(deps.as_ref().storage, &def.scope_spec_address)
                .expect_err(
                    "expected an error to occur when attempting to load the deleted definition",
                );
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected the record not found error to occur when attempting to load by scope spec address, but got: {:?}",
            err,
        );
        insert_asset_definition_v2(deps.as_mut().storage, &def)
            .expect("expected the asset definition to be stored again without error");
        assert_eq!(
            load_asset_definition_v2_by_type(deps.as_ref().storage, &def.asset_type)
                .expect("expected the load to succeed"),
            def,
            "the definition should be once again successfully attainable by asset type",
        );
        assert_eq!(
            load_asset_definition_v2_by_scope_spec(deps.as_ref().storage, &def.scope_spec_address)
                .expect("expected the load to succeed"),
            def,
            "the definition should be once again successfully attainable by scope spec address",
        );
    }

    #[test]
    fn test_delete_nonexistent_asset_definition_by_scope_spec_address_failure() {
        let mut deps = mock_dependencies(&[]);
        let err = delete_asset_definition_v2_by_qualifier(
            deps.as_mut().storage,
            &AssetQualifier::scope_spec_address("fake-scope-spec-address"),
        )
        .expect_err(
            "expected an error to occur when attempting to delete by a missing scope spec address",
        );
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected a record not found error to be emitted when the definition does not exist, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_insert_latest_verifier_detail() {
        let mut deps = mock_dependencies(&[]);
        let mut verifier_detail = get_default_verifier_detail();
        let scope_address = "test-address";
        insert_latest_verifier_detail(deps.as_mut().storage, scope_address, &verifier_detail)
            .expect("expected inserting the verifier detail to succeed");
        let mut detail_from_storage = latest_verifier_detail_store_ro(deps.as_ref().storage)
            .load(scope_address.as_bytes())
            .expect("expected the verifier detail to be fetched successfully from storage");
        assert_eq!(
            verifier_detail, detail_from_storage,
            "expected the verifier detail from storage to match the inserted value",
        );
        verifier_detail.address = "different address".to_string();
        insert_latest_verifier_detail(deps.as_mut().storage, scope_address, &verifier_detail)
            .expect("expected the modified verifier detail to be stored to overwrite the original");
        detail_from_storage = latest_verifier_detail_store_ro(deps.as_ref().storage)
            .load(scope_address.as_bytes())
            .expect(
                "expected the modified verifier detail to be fetched successfully from storage",
            );
        assert_eq!(
            verifier_detail, detail_from_storage,
            "expected the modified verifier detail to match its stored counterpart",
        );
    }

    #[test]
    fn test_delete_latest_verifier_detail() {
        let mut deps = mock_dependencies(&[]);
        let verifier_detail = get_default_verifier_detail();
        let scope_address = "test-address";
        insert_latest_verifier_detail(deps.as_mut().storage, scope_address, &verifier_detail)
            .expect("expected inserting the verifier detail to succeed");
        latest_verifier_detail_store_ro(deps.as_ref().storage)
            .load(scope_address.as_bytes())
            .expect("expected loading the verifier detail from storage after an insert to succeed");
        delete_latest_verifier_detail(deps.as_mut().storage, scope_address)
            .expect("expected deleting an existing verifier detail from storage to succeed");
        assert!(
            latest_verifier_detail_store_ro(deps.as_ref().storage).may_load(scope_address.as_bytes())
                .expect("expected the may_load call on the storage to succeed")
                .is_none(),
            "expected an attempt to load a verifier detail after it is deleted to provide a None option",
        );
    }
}
