use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::delete_asset_definition_v2_by_qualifier;
use crate::core::types::asset_qualifier::AssetQualifier;
use crate::util::aliases::{AssetResult, DepsMutC, EntryPointResponse};
use crate::util::contract_helpers::{check_admin_only, check_funds_are_empty};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};

/// A transformation of [ExecuteMsg::DeleteAssetDefinition](crate::core::msg::ExecuteMsg::DeleteAssetDefinition)
/// for ease of use in the underlying [delete_asset_definition](self::delete_asset_definition) function.
///
/// # Parameters
///
/// * `qualifier` An asset qualifier that can identify the [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
/// to delete.
pub struct DeleteAssetDefinitionV1 {
    pub qualifier: AssetQualifier,
}
impl DeleteAssetDefinitionV1 {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `qualifier` An asset qualifier that can identify the [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2)
    /// to delete.
    pub fn new(qualifier: AssetQualifier) -> Self {
        Self { qualifier }
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
            ExecuteMsg::DeleteAssetDefinition { qualifier } => {
                DeleteAssetDefinitionV1::new(qualifier.to_asset_qualifier()?).to_ok()
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
    deps: DepsMutC,
    info: MessageInfo,
    msg: DeleteAssetDefinitionV1,
) -> EntryPointResponse {
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    let deleted_asset_type = delete_asset_definition_v2_by_qualifier(deps.storage, &msg.qualifier)?;
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
    use crate::core::state::load_asset_definition_v2_by_type;
    use crate::core::types::asset_qualifier::AssetQualifier;
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
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_delete_asset_definition_success_for_asset_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        load_asset_definition_v2_by_type(deps.as_ref().storage, DEFAULT_ASSET_TYPE).expect(
            "sanity check: expected the default asset type to be inserted after instantiation",
        );
        let response = delete_asset_definition(
            deps.as_mut(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            DeleteAssetDefinitionV1::new(AssetQualifier::asset_type(DEFAULT_ASSET_TYPE)),
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
        let err = load_asset_definition_v2_by_type(deps.as_ref().storage, DEFAULT_ASSET_TYPE)
            .expect_err("expected an error to occur when loading the default asset definition");
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected the record not found error to occur when attempting to access the asset definition after deletion, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_delete_asset_definition_success_from_execute_route() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        execute(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ExecuteMsg::DeleteAssetDefinition {
                qualifier: AssetQualifier::asset_type(DEFAULT_ASSET_TYPE).to_serialized_enum(),
            },
        )
        .expect("expected the deletion to be successful");
        let err = load_asset_definition_v2_by_type(deps.as_ref().storage, DEFAULT_ASSET_TYPE)
            .expect_err("expected an error to occur when loading the default asset definition");
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected the record not found error to occur when attempting to access the asset definition after deletion, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_delete_asset_definition_failure_for_invalid_sender() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let err = delete_asset_definition(
            deps.as_mut(),
            empty_mock_info("bad-actor"),
            DeleteAssetDefinitionV1::new(AssetQualifier::asset_type(DEFAULT_ASSET_TYPE)),
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
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let err = delete_asset_definition(
            deps.as_mut(),
            mock_info_with_funds(DEFAULT_ADMIN_ADDRESS, &[coin(100, "coindollars")]),
            DeleteAssetDefinitionV1::new(AssetQualifier::asset_type(DEFAULT_ADMIN_ADDRESS)),
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
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let err = delete_asset_definition(
            deps.as_mut(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            DeleteAssetDefinitionV1::new(AssetQualifier::asset_type("not real asset type")),
        )
        .expect_err("expected an error to occur when an invalid asset type is provided");
        assert!(
            matches!(err, ContractError::RecordNotFound { .. }),
            "expected a record not found error to be emitted when deleting a missing asset type, but got: {:?}",
            err,
        );
    }
}
