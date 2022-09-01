use cosmwasm_std::{MessageInfo, Response};

use crate::core::state::{load_asset_definition_by_type_v3, replace_asset_definition_v3};
use crate::{
    core::{error::ContractError, msg::ExecuteMsg},
    util::{
        aliases::{AssetResult, DepsMutC, EntryPointResponse},
        contract_helpers::{check_admin_only, check_funds_are_empty},
        event_attributes::{EventAttributes, EventType},
        traits::ResultExtensions,
    },
};

/// A transformation of [ExecuteMsg::ToggleAssetDefinition](crate::core::msg::ExecuteMsg::ToggleAssetDefinition)
/// for ease of use in the underlying [toggle_asset_definition](self::toggle_asset_definition) function.
///
/// # Parameters
///
/// * `asset_type` The unique identifier for the target [AssetDefinition](crate::core::types::asset_definition::AssetDefinitionV3),
/// keyed on its [asset_type](crate::core::types::asset_definition::AssetDefinitionV3::asset_type)
/// property.
/// * `expected_result` The value of [enabled](crate::core::types::asset_definition::AssetDefinitionV3::enabled)
/// after the toggle takes place.  This value is required to ensure that multiple toggles executed
/// in succession (either by accident or by various unrelated callers) will only be honored if
/// the asset definition is in the intended state during the execution of the route.
#[derive(Clone, PartialEq, Eq)]
pub struct ToggleAssetDefinitionV1 {
    pub asset_type: String,
    pub expected_result: bool,
}
impl ToggleAssetDefinitionV1 {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `asset_type` The unique identifier for the target [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3),
    /// keyed on its [asset_type](crate::core::types::asset_definition::AssetDefinitionV3::asset_type)
    /// property.
    /// * `expected_result` The value of [enabled](crate::core::types::asset_definition::AssetDefinitionV3::enabled)
    /// after the toggle takes place.  This value is required to ensure that multiple toggles executed
    /// in succession (either by accident or by various unrelated callers) will only be honored if
    /// the asset definition is in the intended state during the execution of the route.
    pub fn new<S: Into<String>>(asset_type: S, expected_result: bool) -> Self {
        ToggleAssetDefinitionV1 {
            asset_type: asset_type.into(),
            expected_result,
        }
    }

    /// Attempts to create an instance of this struct from a provided execute msg.  If the provided
    /// value is not of the [ToggleAssetDefinition](crate::core::msg::ExecuteMsg::ToggleAssetDefinition)
    /// variant, then an [InvalidMessageType](crate::core::error::ContractError::InvalidMessageType)
    /// error will be returned.
    ///
    /// # Parameters
    ///
    /// * `msg` An execute msg provided by the contract's [execute](crate::contract::execute) function.
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<ToggleAssetDefinitionV1> {
        match msg {
            ExecuteMsg::ToggleAssetDefinition {
                asset_type,
                expected_result,
            } => ToggleAssetDefinitionV1::new(asset_type, expected_result).to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::ToggleAssetDefinition".to_string(),
            }
            .to_err(),
        }
    }
}

/// The function used by [execute](crate::contract::execute) when an [ExecuteMsg::ToggleAssetDefinition](crate::core::msg::ExecuteMsg::ToggleAssetDefinition)
/// message is provided.  Attempts to swap the [enabled](crate::core::types::asset_definition::AssetDefinitionV3::enabled)
/// property from true to false, or false to true.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` An instance of the toggle asset definition v1 struct, provided by conversion from an
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).
pub fn toggle_asset_definition(
    deps: DepsMutC,
    info: MessageInfo,
    msg: ToggleAssetDefinitionV1,
) -> EntryPointResponse {
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    let mut asset_definition = load_asset_definition_by_type_v3(deps.storage, &msg.asset_type)?;
    // Never toggle the state if the caller didn't expect the target result
    // If current state == expected result, then the requestor wants to change TO the current state. So this is a no-op.
    if asset_definition.enabled == msg.expected_result {
        return ContractError::UnexpectedState {
            explanation: format!(
                "expected to toggle to [enabled = {}], but toggle would set value to [enabled = {}]",
                msg.expected_result, !asset_definition.enabled
            ),
        }
        .to_err();
    }
    // Simply negate the current value in state to swap it
    asset_definition.enabled = !asset_definition.enabled;
    replace_asset_definition_v3(deps.storage, &asset_definition)?;
    Response::new()
        .add_attributes(
            EventAttributes::new(EventType::ToggleAssetDefinition)
                .set_asset_type(&msg.asset_type)
                .set_new_value(&asset_definition.enabled),
        )
        .to_ok()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_env, mock_info};
    use provwasm_mocks::mock_dependencies;

    use crate::core::state::load_asset_definition_by_type_v3;
    use crate::{
        contract::execute,
        core::{error::ContractError, msg::ExecuteMsg},
        testutil::{
            test_constants::{DEFAULT_ADMIN_ADDRESS, DEFAULT_ASSET_TYPE},
            test_utilities::{
                empty_mock_info, mock_info_with_nhash, single_attribute_for_key,
                test_instantiate_success, InstArgs,
            },
        },
        util::{
            aliases::{DepsC, DepsMutC},
            constants::{ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY, NEW_VALUE_KEY},
            event_attributes::EventType,
        },
    };

    use super::{toggle_asset_definition, ToggleAssetDefinitionV1};

    #[test]
    fn test_valid_toggle_asset_definition_via_execute() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let response = execute(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ExecuteMsg::ToggleAssetDefinition {
                asset_type: DEFAULT_ASSET_TYPE.to_string(),
                expected_result: false,
            },
        )
        .expect("the toggle should work correctly");
        assert!(
            response.messages.is_empty(),
            "toggling an asset definition should not require messages",
        );
        assert_eq!(
            3,
            response.attributes.len(),
            "toggling an asset definition should produce the correct number of attributes",
        );
        assert_eq!(
            EventType::ToggleAssetDefinition.event_name().as_str(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "the proper event type should be emitted",
        );
        assert_eq!(
            DEFAULT_ASSET_TYPE,
            single_attribute_for_key(&response, ASSET_TYPE_KEY),
            "the proper asset type should be emitted",
        );
        assert_eq!(
            "false",
            single_attribute_for_key(&response, NEW_VALUE_KEY),
            "the new value key should indicate that the asset definition has been set to enabled = false",
        );
        test_toggle_has_successfully_occurred(&deps.as_ref(), false);
    }

    #[test]
    fn test_valid_toggle_asset_definition_via_internal() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        toggle_default_asset_definition(deps.as_mut(), false);
        test_toggle_has_successfully_occurred(&deps.as_ref(), false);
    }

    #[test]
    fn test_toggle_off_and_back_on() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        // First toggle should disable the automagically enabled default asset type
        toggle_default_asset_definition(deps.as_mut(), false);
        test_toggle_has_successfully_occurred(&deps.as_ref(), false);
        // Second toggle should re-enable it
        toggle_default_asset_definition(deps.as_mut(), true);
        test_toggle_has_successfully_occurred(&deps.as_ref(), true);
    }

    #[test]
    fn test_invalid_toggle_asset_definition_for_invalid_msg() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = execute(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ExecuteMsg::ToggleAssetDefinition {
                asset_type: String::new(),
                expected_result: false,
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "expected the invalid message fields error to be returned when the message is malformatted, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_toggle_asset_definition_for_invalid_sender() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = toggle_asset_definition(
            deps.as_mut(),
            mock_info("not-the-admin", &[]),
            ToggleAssetDefinitionV1::new(DEFAULT_ASSET_TYPE, false),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::Unauthorized { .. }),
            "expected the unauthorized error to be returned when the sender is not the admin, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_toggle_asset_definition_for_provided_funds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = toggle_asset_definition(
            deps.as_mut(),
            mock_info_with_nhash(DEFAULT_ADMIN_ADDRESS, 150),
            ToggleAssetDefinitionV1::new(DEFAULT_ASSET_TYPE, false),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "expected the invalid funds error to be returned when the sender provides funds, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_toggle_asset_definition_for_invalid_asset_target() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = toggle_asset_definition(
            deps.as_mut(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ToggleAssetDefinitionV1::new("no-u", false),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::RecordNotFound { .. }),
            "expected the not found error to be returned, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_toggle_to_incorrect_expected_state_fails() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        // The asset type should be enabled by default, so trying to toggle it to enabled again should fail
        let enable_error = toggle_asset_definition(
            deps.as_mut(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ToggleAssetDefinitionV1::new(DEFAULT_ASSET_TYPE, true),
        )
        .unwrap_err();
        match enable_error {
            ContractError::UnexpectedState { explanation } => {
                assert_eq!(
                    "expected to toggle to [enabled = true], but toggle would set value to [enabled = false]",
                    explanation.as_str(),
                    "incorrect error message encountered on invalid toggle false -> true",
                );
            }
            _ => panic!(
                "unexpected error encountered on invalid toggle false -> true: {:?}",
                enable_error
            ),
        };
        // Toggle off successfully to ensure the opposite attempt cannot be made either
        toggle_default_asset_definition(deps.as_mut(), false);
        let disable_error = toggle_asset_definition(
            deps.as_mut(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ToggleAssetDefinitionV1::new(DEFAULT_ASSET_TYPE, false),
        )
        .unwrap_err();
        match disable_error {
            ContractError::UnexpectedState { explanation } => {
                assert_eq!(
                    "expected to toggle to [enabled = false], but toggle would set value to [enabled = true]",
                    explanation.as_str(),
                    "incorrect error message encountered on invalid toggle true -> false",
                );
            }
            _ => panic!(
                "unexpected error encountered on invalid toggle true -> false: {:?}",
                disable_error
            ),
        }
    }

    fn test_toggle_has_successfully_occurred(deps: &DepsC, expected_enabled_value: bool) {
        let asset_def = load_asset_definition_by_type_v3(deps.storage, DEFAULT_ASSET_TYPE)
            .expect("the default asset definition should exist in storage");
        assert_eq!(
            expected_enabled_value, asset_def.enabled,
            "the asset definition enabled value was not toggled to the expected value",
        );
    }

    fn toggle_default_asset_definition(deps: DepsMutC, expected_result: bool) {
        toggle_asset_definition(
            deps,
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ToggleAssetDefinitionV1::new(DEFAULT_ASSET_TYPE, expected_result),
        )
        .expect("toggle should execute without fail");
    }
}
