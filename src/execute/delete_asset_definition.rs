use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::delete_asset_definition_by_asset_type_v3;
use crate::util::aliases::{AssetResult, EntryPointResponse};
use crate::util::contract_helpers::{check_admin_only, check_funds_are_empty};
use crate::util::event_attributes::{EventAttributes, EventType};

use cosmwasm_std::{DepsMut, MessageInfo, Response};
use result_extensions::ResultExtensions;

/// A transformation of [ExecuteMsg::DeleteAssetDefinition](crate::core::msg::ExecuteMsg::DeleteAssetDefinition)
/// for ease of use in the underlying [delete_asset_definition](self::delete_asset_definition) function.
///
/// # Parameters
///
/// * `asset_type` The asset type to delete.
pub struct DeleteAssetDefinitionV1 {
    pub asset_type: String,
}
impl DeleteAssetDefinitionV1 {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `asset_type` The asset type to delete.
    pub fn new(asset_type: &str) -> Self {
        Self {
            asset_type: asset_type.to_string(),
        }
    }

    /// Attempts to create an instance of this struct from a provided execute msg.  If the provided
    /// value is not of the [DeleteAssetDefinition](crate::core::msg::ExecuteMsg::DeleteAssetDefinition)
    /// variant, then an [InvalidMessageType](crate::core::error::ContractError::InvalidMessageType)
    /// error will be returned.
    ///
    /// # Parameters
    ///
    /// * `msg` An execute msg provided by the contract's [execute](crate::contract::execute) function.
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<DeleteAssetDefinitionV1> {
        match msg {
            ExecuteMsg::DeleteAssetDefinition { asset_type } => {
                DeleteAssetDefinitionV1::new(&asset_type).to_ok()
            }
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::DeleteAssetDefinition".to_string(),
            }
            .to_err(),
        }
    }
}

/// Route implementation for [ExecuteMsg::DeleteAssetDefinition](crate::core::msg::ExecuteMsg::DeleteAssetDefinition).
/// This function allows for the admin address to completely remove an asset definition.  This is
/// dangerous, because existing assets in the onboarding process for an asset definition will start
/// emitting errors when being verified or retried.  This should only ever be used on a definition
/// that is guaranteed to be not in use and/or was erroneously added.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` An instance of the delete asset definition v1 struct, provided by conversion from an
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).
pub fn delete_asset_definition(
    deps: DepsMut,
    info: MessageInfo,
    msg: DeleteAssetDefinitionV1,
) -> EntryPointResponse {
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    let deleted_asset_type =
        delete_asset_definition_by_asset_type_v3(deps.storage, &msg.asset_type)?;
    Response::new()
        .add_attributes(
            EventAttributes::new(EventType::DeleteAssetDefinition)
                .set_asset_type(deleted_asset_type),
        )
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::contract::execute;
    use crate::core::error::ContractError;
    use crate::core::msg::ExecuteMsg;
    use crate::core::state::load_asset_definition_by_type_v3;
    use crate::execute::delete_asset_definition::{
        delete_asset_definition, DeleteAssetDefinitionV1,
    };
    use crate::testutil::test_constants::{DEFAULT_ADMIN_ADDRESS, DEFAULT_ASSET_TYPE};
    use crate::testutil::test_utilities::{
        empty_mock_info, mock_info_with_funds, single_attribute_for_key, test_instantiate_success,
        InstArgs,
    };
    use crate::util::constants::{ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY};
    use crate::util::event_attributes::EventType;
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::mock_env;
    use provwasm_mocks::mock_provenance_dependencies;

    #[test]
    fn test_delete_asset_definition_success_for_asset_type() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        load_asset_definition_by_type_v3(deps.as_ref().storage, DEFAULT_ASSET_TYPE).expect(
            "sanity check: expected the default asset type to be inserted after instantiation",
        );
        let response = delete_asset_definition(
            deps.as_mut(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            DeleteAssetDefinitionV1::new(DEFAULT_ASSET_TYPE),
        )
        .expect("expected deletion by asset type to succeed");
        assert!(
            response.messages.is_empty(),
            "the route should not emit messages",
        );
        assert_eq!(
            2,
            response.attributes.len(),
            "expected the correct number of attributes to be emitted",
        );
        assert_eq!(
            EventType::DeleteAssetDefinition.event_name(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "expected the event type attribute to be set correctly",
        );
        assert_eq!(
            DEFAULT_ASSET_TYPE,
            single_attribute_for_key(&response, ASSET_TYPE_KEY),
            "expected the asset type attribute to be set correctly",
        );
        let err = load_asset_definition_by_type_v3(deps.as_ref().storage, DEFAULT_ASSET_TYPE)
            .expect_err("expected an error to occur when loading the default asset definition");
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected the record not found error to occur when attempting to access the asset definition after deletion, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_delete_asset_definition_success_from_execute_route() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        execute(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ExecuteMsg::DeleteAssetDefinition {
                asset_type: DEFAULT_ASSET_TYPE.to_string(),
            },
        )
        .expect("expected the deletion to be successful");
        let err = load_asset_definition_by_type_v3(deps.as_ref().storage, DEFAULT_ASSET_TYPE)
            .expect_err("expected an error to occur when loading the default asset definition");
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected the record not found error to occur when attempting to access the asset definition after deletion, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_delete_asset_definition_failure_for_invalid_sender() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let err = delete_asset_definition(
            deps.as_mut(),
            empty_mock_info("bad-actor"),
            DeleteAssetDefinitionV1::new(DEFAULT_ASSET_TYPE),
        )
        .expect_err(
            "expected an error to occur when a non-admin user attempts to access the route",
        );
        assert!(
            matches!(err, ContractError::Unauthorized { .. }),
            "expected an unauthorized error to be emitted, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_delete_asset_definition_failure_for_provided_funds() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let err = delete_asset_definition(
            deps.as_mut(),
            mock_info_with_funds(DEFAULT_ADMIN_ADDRESS, &[coin(100, "coindollars")]),
            DeleteAssetDefinitionV1::new(DEFAULT_ASSET_TYPE),
        )
        .expect_err("expected an error to occur when funds are provided by the admin");
        assert!(
            matches!(err, ContractError::InvalidFunds(..)),
            "expected an invalid funds error to be emitted, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_delete_asset_definition_failure_for_missing_definition() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let err = delete_asset_definition(
            deps.as_mut(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            DeleteAssetDefinitionV1::new("not real asset type"),
        )
        .expect_err("expected an error to occur when an invalid asset type is provided");
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected a record not found error to be emitted when deleting a missing asset type, but got: {:?}",
            err,
        );
    }
}
