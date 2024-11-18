use crate::core::types::asset_definition::AssetDefinitionV3;
use crate::core::types::fee_payment_detail::FeePaymentDetail;
use crate::{core::msg::InitMsg, util::aliases::AssetResult};
use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::{Item, Map};
use result_extensions::ResultExtensions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::error::ContractError;

const STATE_V2_KEY: &str = "�state_v2";
pub const STATE_V2: Item<StateV2> = Item::new(STATE_V2_KEY);

const FEE_PAYMENT_DETAIL_NAMESPACE: &str = "fee_payment_detail";
const FEE_PAYMENT_DETAILS: Map<(Addr, String), FeePaymentDetail> =
    Map::new(FEE_PAYMENT_DETAIL_NAMESPACE);

/// Stores the main configurations for the contract internally.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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

/// Value is currently 'asset_definitions_v2' due to a structural change of data (removing an existing field, scope_spec_address) and switching from
/// and IndexedMap to a regular Map... so everything was changed to be called 'v3', but no migration was actually needed to transition all values to new
/// keys as the existing config was able to be read as a Map as-is.
const ASSET_DEFINITIONS_NAMESPACE: &str = "asset_definitions_v2";
/// The main entrypoint access for [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3) state.
/// Establishes an index map for all definitions, allowing the standard save(), load() and iterator
/// functionality. Private access to ensure only helper functions below are used.
const ASSET_DEFINITIONS_V3: Map<String, AssetDefinitionV3> = Map::new(ASSET_DEFINITIONS_NAMESPACE);

pub fn list_asset_definitions_v3(storage: &dyn Storage) -> Vec<AssetDefinitionV3> {
    ASSET_DEFINITIONS_V3
        .range(storage, None, None, cosmwasm_std::Order::Descending)
        .filter(|result| result.is_ok())
        .map(|result| result.unwrap().1)
        .collect::<Vec<AssetDefinitionV3>>()
}

/// Inserts a new asset definition into storage. If a value already exists, an error will be returned.
/// Note: Asset definitions must contain a unique [asset_type](super::types::asset_definition::AssetDefinitionV3::asset_type)
/// value. An error will be returned if this unique constraint is violated.
///
/// # Parameters
///
/// * `storage` A mutable reference to the contract's internal storage.
/// * `definition` The asset definition to insert into storage and derive the unique keys.
pub fn insert_asset_definition_v3(
    storage: &mut dyn Storage,
    definition: &AssetDefinitionV3,
) -> AssetResult<()> {
    let state = ASSET_DEFINITIONS_V3;
    let key = definition.storage_key();
    if let Ok(existing_def) = state.load(storage, key.clone()) {
        ContractError::RecordAlreadyExists {
            explanation: format!(
                "unique constraints violated! record exists with asset type [{}]",
                existing_def.asset_type
            ),
        }
        .to_err()
    } else {
        // At this point, we know there is no old data available, so we can safely call the replace function and
        // specify None for the old_data param.
        state
            .save(storage, key, definition)
            .map_err(ContractError::Std)
    }
}

/// Replaces an existing asset definition in state with the provided value.  If no value exists for
/// the given definition, an error will be returned.  Note: Map (the internal storage type)
/// provides a really nice update() function that allows two branches (one for
/// success and one for failure to find) that seems ideal for this functionality, but it requires a
/// non-reference version of the data to be used. This requires that the provided definition must be
/// cloned, which makes it vastly inefficient compared to this implementation.
///
/// # Parameters
///
/// * `storage` A mutable reference to the contract's internal storage.
/// * `definition` The asset definition to replace by matching on its [asset_type](super::types::asset_definition::AssetDefinitionV3::asset_type)
/// property.
pub fn replace_asset_definition_v3(
    storage: &mut dyn Storage,
    definition: &AssetDefinitionV3,
) -> AssetResult<()> {
    let state = ASSET_DEFINITIONS_V3;
    let key = definition.storage_key();
    if let Ok(Some(_)) = state.may_load(storage, key.to_string()) {
        state
            .save(storage, key, definition)
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
/// * `asset_type` The unique name key [asset_type](super::types::asset_definition::AssetDefinitionV3::asset_type)
/// for the requested asset definition.
pub fn may_load_asset_definition_by_type_v3<S: Into<String>>(
    storage: &dyn Storage,
    asset_type: S,
) -> AssetResult<Option<AssetDefinitionV3>> {
    ASSET_DEFINITIONS_V3
        // Coerce to lowercase to match how stored values are keyed
        .may_load(storage, asset_type.into().to_lowercase())
        .map_err(ContractError::Std)
}

/// Finds an existing asset definition by asset type, or returns an error if no definition is found.
///
/// # Parameters
///
/// * `storage` A reference to the contract's internal storage.
/// * `asset_type` The unique name key [asset_type](super::types::asset_definition::AssetDefinitionV3::asset_type)
/// for the requested asset definition.
pub fn load_asset_definition_by_type_v3<S: Into<String>>(
    storage: &dyn Storage,
    asset_type: S,
) -> AssetResult<AssetDefinitionV3> {
    let asset_type = asset_type.into();
    if let Some(asset_definition) = may_load_asset_definition_by_type_v3(storage, &asset_type)? {
        asset_definition.to_ok()
    } else {
        ContractError::RecordNotFound {
            explanation: format!("no asset definition existed for asset type {}", asset_type,),
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
/// * `asset_type` The asset type to delete.
pub fn delete_asset_definition_by_asset_type_v3(
    storage: &mut dyn Storage,
    asset_type: &str,
) -> AssetResult<String> {
    let existing_asset_type = load_asset_definition_by_type_v3(storage, asset_type)?.asset_type;
    ASSET_DEFINITIONS_V3.remove(storage, existing_asset_type.to_lowercase());
    Ok(existing_asset_type)
}

/// Inserts a new payment detail into storage.  If a value already exists, an error will be returned.
/// Note: Each payment detail must contain a unique [scope_address](super::types::fee_payment_detail::FeePaymentDetail::scope_address)
/// value, or the insert will be rejected with an error.
///
/// # Parameters
///
/// * `storage` A mutable reference to the contract's internal storage.
/// * `fee_payment_detail` The detail to insert into storage and derive the unique scope address key.
pub fn insert_fee_payment_detail(
    storage: &mut dyn Storage,
    fee_payment_detail: &FeePaymentDetail,
    asset_type: &str,
) -> AssetResult<()> {
    if load_fee_payment_detail(storage, &fee_payment_detail.scope_address, asset_type).is_ok() {
        return ContractError::RecordAlreadyExists {
            explanation: format!(
                "cannot insert payment detail for scope [{}] and asset type [{}] because a record already exists with that address and asset type",
                &fee_payment_detail.scope_address,
                &asset_type
            )
        }.to_err();
    }
    FEE_PAYMENT_DETAILS
        .save(
            storage,
            (
                Addr::unchecked(&fee_payment_detail.scope_address),
                asset_type.into(),
            ),
            fee_payment_detail,
        )?
        .to_ok()
}

/// Finds an existing fee payment detail by scope address, or returns an error if no detail is found.
///
/// # Parameters
///
/// * `storage` A reference to the contract's internal storage.
/// * `scope_address` The unique key [scope_address](super::types::fee_payment_detail::FeePaymentDetail::scope_address)
/// for the requested payment detail.
pub fn load_fee_payment_detail<S1: Into<String>, S2: Into<String>>(
    storage: &dyn Storage,
    scope_address: S1,
    asset_type: S2,
) -> AssetResult<FeePaymentDetail> {
    FEE_PAYMENT_DETAILS
        .load(storage, (Addr::unchecked(scope_address), asset_type.into()))?
        .to_ok()
}

/// Attempts to find an existing fee payment detail by scope address, or returns a None variant if
/// an error occurs or no detail is found.
///
/// # Parameters
///
/// * `storage` A reference to the contract's internal storage.
/// * `scope_address` The unique key [scope_address](super::types::fee_payment_detail::FeePaymentDetail::scope_address)
/// for the requested payment detail.
pub fn may_load_fee_payment_detail<S1: Into<String>, S2: Into<String>>(
    storage: &dyn Storage,
    scope_address: S1,
    asset_type: S2,
) -> Option<FeePaymentDetail> {
    FEE_PAYMENT_DETAILS
        .may_load(storage, (Addr::unchecked(scope_address), asset_type.into()))
        .unwrap_or(None)
}

/// Attempts to delete an existing payment detail by scope address.  Returns an error if the detail
/// does not exist or if deletion fails.
///
/// # Parameters
///
/// * `storage` A mutable reference to the contract's internal storage.
/// * `scope_address` The unique key [scope_address](super::types::fee_payment_detail::FeePaymentDetail::scope_address)
/// of the detail to delete.
pub fn delete_fee_payment_detail<S1: Into<String>, S2: Into<String>>(
    storage: &mut dyn Storage,
    scope_address: S1,
    asset_type: S2,
) -> AssetResult<()> {
    let scope_address = scope_address.into();
    let asset_type = asset_type.into();
    // Verify the detail exists before allowing its deletion.  The standard "remove" function will
    // not produce an error if no target value exists, but it is very informative of bad code to
    // reveal when unnecessary operations occur.
    load_fee_payment_detail(storage, &scope_address, &asset_type)?;
    FEE_PAYMENT_DETAILS.remove(
        storage,
        (
            Addr::unchecked(scope_address.to_owned()),
            asset_type.to_owned(),
        ),
    );
    ().to_ok()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::StdError;
    use provwasm_mocks::mock_provenance_dependencies;

    use crate::core::error::ContractError;
    use crate::core::state::{
        delete_asset_definition_by_asset_type_v3, delete_fee_payment_detail,
        insert_asset_definition_v3, insert_fee_payment_detail, load_asset_definition_by_type_v3,
        load_fee_payment_detail, may_load_asset_definition_by_type_v3, may_load_fee_payment_detail,
        replace_asset_definition_v3,
    };
    use crate::core::types::asset_definition::AssetDefinitionV3;
    use crate::testutil::test_constants::{DEFAULT_ASSET_TYPE, DEFAULT_SCOPE_ADDRESS};
    use crate::testutil::test_utilities::get_duped_fee_payment_detail;
    use crate::util::traits::OptionExtensions;

    #[test]
    fn test_insert_asset_definition() {
        let mut deps = mock_provenance_dependencies();
        let def = AssetDefinitionV3::new("heloc", "Home Equity Line of Credit".to_some(), vec![]);
        insert_asset_definition_v3(deps.as_mut().storage, &def)
            .expect("insert should work correctly");
        let error = insert_asset_definition_v3(deps.as_mut().storage, &def).unwrap_err();
        match error {
            ContractError::RecordAlreadyExists { explanation } => {
                assert_eq!(
                    "unique constraints violated! record exists with asset type [heloc]",
                    explanation,
                    "the proper record type should be returned in the resulting error"
                );
            }
            _ => panic!("unexpected error encountered: {:?}", error),
        }
        let loaded_asset_definition =
            load_asset_definition_by_type_v3(deps.as_ref().storage, &def.asset_type)
                .expect("asset definition should load without error");
        assert_eq!(
            loaded_asset_definition, def,
            "the asset definition should be inserted correctly"
        );
    }

    #[test]
    fn test_replace_asset_definition() {
        let mut deps = mock_provenance_dependencies();
        let mut def =
            AssetDefinitionV3::new("heloc", "Home Equity Line of Credit".to_some(), vec![]);
        let error = replace_asset_definition_v3(deps.as_mut().storage, &def).unwrap_err();
        match error {
            ContractError::RecordNotFound { explanation } => {
                assert_eq!(
                    "no record exists to update for asset type [heloc]", explanation,
                    "the proper record type should be returned in the resulting error",
                );
            }
            _ => panic!("unexpected error encountered: {:?}", error),
        };
        insert_asset_definition_v3(deps.as_mut().storage, &def)
            .expect("insert should work correctly");
        def.enabled = !def.enabled;
        replace_asset_definition_v3(deps.as_mut().storage, &def)
            .expect("update should work correctly");
        let loaded_asset_definition =
            load_asset_definition_by_type_v3(deps.as_ref().storage, &def.asset_type)
                .expect("asset definition should load without error");
        assert_eq!(
            loaded_asset_definition, def,
            "the asset definition should be updated appropriately"
        );
    }

    #[test]
    fn test_may_load_asset_definition_by_type() {
        let mut deps = mock_provenance_dependencies();
        let heloc = AssetDefinitionV3::new("heloc", "Home Equity Line of Credit".to_some(), vec![]);
        insert_asset_definition_v3(deps.as_mut().storage, &heloc)
            .expect("the heloc definition should insert without error");
        assert!(
            may_load_asset_definition_by_type_v3(deps.as_ref().storage, "not-heloc")
                .expect("may load asset definition by type should execute without error")
                .is_none(),
            "expected the missing asset definition to return an empty Option",
        );
        assert_eq!(
            may_load_asset_definition_by_type_v3(deps.as_ref().storage, &heloc.asset_type)
            .expect("may load asset definition by type should execute without error")
            .expect("expected the asset definition loaded by a populated type to be present"),
            heloc,
            "expected the loaded asset definition to equate to the original value that was inserted",
        );
    }

    #[test]
    fn test_load_asset_definition_by_type() {
        let mut deps = mock_provenance_dependencies();
        let heloc = AssetDefinitionV3::new("heloc", "Home Equity Line of Credit".to_some(), vec![]);
        let mortgage = AssetDefinitionV3::new("mortgage", "DEATH PLEDGE".to_some(), vec![]);
        insert_asset_definition_v3(deps.as_mut().storage, &heloc)
            .expect("the heloc definition should insert appropriately");
        insert_asset_definition_v3(deps.as_mut().storage, &mortgage)
            .expect("the mortgage definition should insert appropriately");
        let heloc_from_storage =
            load_asset_definition_by_type_v3(deps.as_ref().storage, &heloc.asset_type)
                .expect("the heloc definition should load without error");
        let mortgage_from_storage =
            load_asset_definition_by_type_v3(deps.as_ref().storage, &mortgage.asset_type)
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
        let mut deps = mock_provenance_dependencies();
        let def = AssetDefinitionV3::new("heloc", "Home Equity Line of Credit".to_some(), vec![]);
        insert_asset_definition_v3(deps.as_mut().storage, &def)
            .expect("expected the asset definition to be stored without error");
        assert_eq!(
            load_asset_definition_by_type_v3(deps.as_ref().storage, &def.asset_type)
                .expect("expected the load to succeed"),
            def,
            "sanity check: asset definition should be accessible by asset type",
        );
        delete_asset_definition_by_asset_type_v3(deps.as_mut().storage, &def.asset_type)
            .expect("expected the deletion to succeed");
        let err = load_asset_definition_by_type_v3(deps.as_ref().storage, &def.asset_type)
            .expect_err(
                "expected an error to occur when attempting to load the deleted definition",
            );
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected the record not found error to occur when attempting to load by asset type, but got: {:?}",
            err,
        );
        insert_asset_definition_v3(deps.as_mut().storage, &def)
            .expect("expected the asset definition to be stored again without error");
        assert_eq!(
            load_asset_definition_by_type_v3(deps.as_ref().storage, &def.asset_type)
                .expect("expected the load to succeed"),
            def,
            "the definition should be once again successfully attainable by asset type",
        );
    }

    #[test]
    fn test_delete_nonexistent_asset_definition_by_type_failure() {
        let mut deps = mock_provenance_dependencies();
        let err = delete_asset_definition_by_asset_type_v3(deps.as_mut().storage, "fake-type")
            .expect_err(
                "expected an error to occur when attempting to delete a missing asset type",
            );
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected a record not found error to be emitted when the definition does not exist, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_insert_and_load_fee_payment_detail() {
        let mut deps = mock_provenance_dependencies();
        let err = load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect_err(
            "an error should occur when trying to load a payment detail that does not exist",
        );
        assert!(
            matches!(err, ContractError::Std(StdError::NotFound { .. })),
            "a not found error should occur when the payment detail is not found, but got: {:?}",
            err,
        );
        let payment_detail = get_duped_fee_payment_detail(DEFAULT_SCOPE_ADDRESS);
        insert_fee_payment_detail(deps.as_mut().storage, &payment_detail, DEFAULT_ASSET_TYPE)
            .expect("inserting a new fee payment detail should succeed");
        let loaded_payment_detail = load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect("loading the stored payment detail should succeed");
        assert_eq!(
            payment_detail, loaded_payment_detail,
            "the loaded payment detail should equate to the stored value",
        );
        let err = insert_fee_payment_detail(
            deps.as_mut().storage,
            &loaded_payment_detail,
            DEFAULT_ASSET_TYPE,
        )
        .expect_err("an error should occur when attempting to insert a duplicate payment detail");
        assert!(
            matches!(err, ContractError::RecordAlreadyExists { .. }),
            "a record already exists error should occur, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_may_load_fee_payment_detail() {
        let mut deps = mock_provenance_dependencies();
        assert!(
            may_load_fee_payment_detail(
                deps.as_ref().storage,
                DEFAULT_SCOPE_ADDRESS,
                DEFAULT_ASSET_TYPE
            )
            .is_none(),
            "attempting to load a detail that does not exist should produce a None variant",
        );
        let payment_detail = get_duped_fee_payment_detail(DEFAULT_SCOPE_ADDRESS);
        insert_fee_payment_detail(deps.as_mut().storage, &payment_detail, DEFAULT_ASSET_TYPE)
            .expect("inserting a new fee payment detail should succeed");
        let loaded_payment_detail = may_load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect("the fee payment detail should load successfully");
        assert_eq!(
            payment_detail, loaded_payment_detail,
            "the loaded payment detail should equate to the inserted value",
        );
    }

    #[test]
    fn test_delete_fee_payment_detail() {
        let mut deps = mock_provenance_dependencies();
        let err = delete_fee_payment_detail(deps.as_mut().storage, DEFAULT_SCOPE_ADDRESS, DEFAULT_ASSET_TYPE).expect_err(
            "an error should occur when attempting to delete a fee payment detail that does not exist"
        );
        assert!(
            matches!(err, ContractError::Std(StdError::NotFound { .. })),
            "a not found error should occur when the payment detail is not found, but got: {:?}",
            err,
        );
        let payment_detail = get_duped_fee_payment_detail(DEFAULT_SCOPE_ADDRESS);
        insert_fee_payment_detail(deps.as_mut().storage, &payment_detail, DEFAULT_ASSET_TYPE)
            .expect("inserting a payment detail should succeed");
        assert!(
            load_fee_payment_detail(
                deps.as_ref().storage,
                DEFAULT_SCOPE_ADDRESS,
                DEFAULT_ASSET_TYPE
            )
            .is_ok(),
            "sanity check: fee payment detail should be available after insert",
        );
        delete_fee_payment_detail(
            deps.as_mut().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect("deleting a payment detail should succeed");
        let err = load_fee_payment_detail(
            deps.as_ref().storage,
            DEFAULT_SCOPE_ADDRESS,
            DEFAULT_ASSET_TYPE,
        )
        .expect_err(
            "an error should occur when trying to load a payment detail after it has been deleted",
        );
        assert!(
            matches!(err, ContractError::Std(StdError::NotFound { .. })),
            "a not found error should occur when the payment detail is loaded after deletion, but got: {:?}",
            err,
        );
    }
}
