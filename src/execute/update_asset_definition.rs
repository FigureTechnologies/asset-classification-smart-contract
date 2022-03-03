use crate::core::error::ContractError;
use crate::core::msg::{AssetDefinitionInput, ExecuteMsg};
use crate::core::state::{asset_state, asset_state_read, AssetDefinition};
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::contract_helpers::{check_admin_only, check_funds_are_empty};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct UpdateAssetDefinitionV1 {
    pub asset_definition: AssetDefinition,
}
impl UpdateAssetDefinitionV1 {
    pub fn new(asset_definition: AssetDefinition) -> Self {
        UpdateAssetDefinitionV1 { asset_definition }
    }

    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<UpdateAssetDefinitionV1> {
        match msg {
            ExecuteMsg::UpdateAssetDefinition { asset_definition } => Ok(asset_definition.into()),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::UpdateAssetDefinition".to_string(),
            }
            .to_err(),
        }
    }
}
impl ResultExtensions for UpdateAssetDefinitionV1 {}
impl From<AssetDefinitionInput> for UpdateAssetDefinitionV1 {
    fn from(input: AssetDefinitionInput) -> Self {
        UpdateAssetDefinitionV1 {
            asset_definition: input.into(),
        }
    }
}
impl From<AssetDefinition> for UpdateAssetDefinitionV1 {
    fn from(asset_definition: AssetDefinition) -> Self {
        UpdateAssetDefinitionV1 { asset_definition }
    }
}

pub fn update_asset_definition(
    deps: DepsMutC,
    info: MessageInfo,
    msg: UpdateAssetDefinitionV1,
) -> ContractResponse {
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    // If the asset definition does not exist within the state, there is nothing to update
    if asset_state_read(deps.storage, &msg.asset_definition.asset_type)
        .may_load()?
        .is_none()
    {
        return ContractError::NotFound {
            explanation: format!(
                "no record exists for asset type {}. please use add asset definition instead",
                msg.asset_definition.asset_type
            ),
        }
        .to_err();
    }
    // Overwrite the existing asset definition with the new one
    asset_state(deps.storage, &msg.asset_definition.asset_type).save(&msg.asset_definition)?;
    Response::new()
        .add_attributes(
            EventAttributes::new(EventType::UpdateAssetDefinition)
                .set_asset_type(&msg.asset_definition.asset_type),
        )
        .to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::contract::execute;
    use crate::core::error::ContractError;
    use crate::core::msg::{AssetDefinitionInput, ExecuteMsg};
    use crate::core::state::{asset_state_read, AssetDefinition, FeeDestination, ValidatorDetail};
    use crate::execute::update_asset_definition::{
        update_asset_definition, UpdateAssetDefinitionV1,
    };
    use crate::testutil::test_utilities::{
        empty_mock_info, single_attribute_for_key, test_instantiate_success, InstArgs,
        DEFAULT_ASSET_TYPE, DEFAULT_INFO_NAME,
    };
    use crate::util::aliases::DepsC;
    use crate::util::constants::{ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY};
    use crate::util::event_attributes::EventType;
    use crate::validation::validate_init_msg::validate_asset_definition_input;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_valid_update_asset_definition_via_execute() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let asset_definition = get_update_asset_definition();
        let response = execute(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(),
            ExecuteMsg::UpdateAssetDefinition {
                asset_definition: asset_definition.clone(),
            },
        )
        .expect("expected the update asset checks to work correctly");
        assert!(
            response.messages.is_empty(),
            "updating an asset definition should not require messages",
        );
        assert_eq!(
            2,
            response.attributes.len(),
            "updating an asset definition should produce the correct number of attributes",
        );
        assert_eq!(
            EventType::UpdateAssetDefinition.event_name().as_str(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "the correct event type should be emitted",
        );
        assert_eq!(
            DEFAULT_ASSET_TYPE,
            single_attribute_for_key(&response, ASSET_TYPE_KEY),
            "the asset type attribute should be added correctly",
        );
        test_asset_definition_was_updated_for_input(&asset_definition, &deps.as_ref());
    }

    #[test]
    fn test_valid_update_asset_definition_via_internal() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = get_valid_update_asset_definition();
        update_asset_definition(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            msg.clone(),
        )
        .expect("expected the update asset definition function to return properly");
        test_asset_definition_was_updated(&msg.asset_definition, &deps.as_ref());
    }

    #[test]
    fn test_invalid_update_asset_definition_for_invalid_msg() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = ExecuteMsg::UpdateAssetDefinition {
            asset_definition: AssetDefinitionInput::new(
                DEFAULT_ASSET_TYPE.to_string(),
                vec![],
                None,
            ),
        };
        let error = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            msg,
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "expected an invalid asset definition to cause an InvalidMessageFields error",
        );
    }

    #[test]
    fn test_invalid_update_asset_definition_for_invalid_sender() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = update_asset_definition(
            deps.as_mut(),
            mock_info("not-the-admin", &[]),
            get_valid_update_asset_definition(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::Unauthorized { .. }),
            "expected the unauthorized response to be returned when a different address than the admin is the sender",
        );
    }

    #[test]
    fn test_invalid_update_asset_definition_for_provided_funds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = update_asset_definition(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[coin(420, "usdf")]),
            get_valid_update_asset_definition(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "expected the invalid funds response to be returned when funds are provided to the function",
        );
    }

    #[test]
    fn test_invalid_update_asset_definition_for_missing_loan_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let missing_asset_definition = AssetDefinition::new(
            "nonexistent-type".to_string(),
            vec![ValidatorDetail::new(
                "validator".to_string(),
                Uint128::new(100),
                Decimal::percent(25),
                vec![FeeDestination::new(
                    "fee-guy".to_string(),
                    Decimal::percent(100),
                )],
            )],
        );
        let error = update_asset_definition(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            UpdateAssetDefinitionV1::new(missing_asset_definition),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::NotFound { .. }),
            "expected the not found response to be returned when an update is attempted for a definition that does not exist",
        );
    }

    fn test_asset_definition_was_updated_for_input(input: &AssetDefinitionInput, deps: &DepsC) {
        test_asset_definition_was_updated(&AssetDefinition::from(input), deps)
    }

    fn test_asset_definition_was_updated(asset_definition: &AssetDefinition, deps: &DepsC) {
        let state_def = asset_state_read(deps.storage, &asset_definition.asset_type)
            .load()
            .expect("expected the updated asset definition to be stored in the state");
        assert_eq!(
            asset_definition, &state_def,
            "the value in state should directly equate to the added value",
        );
    }

    // This builds off of the existing default asset definition in test_utilities and adds/tweaks
    // details
    fn get_update_asset_definition() -> AssetDefinitionInput {
        let def = AssetDefinitionInput::new(
            DEFAULT_ASSET_TYPE.into(),
            vec![ValidatorDetail {
                address: "different-validator-address".to_string(),
                onboarding_cost: Uint128::new(1500000),
                fee_percent: Decimal::percent(50),
                fee_destinations: vec![
                    FeeDestination::new("first".to_string(), Decimal::percent(70)),
                    FeeDestination::new("second".to_string(), Decimal::percent(30)),
                ],
            }],
            None,
        );
        validate_asset_definition_input(&def, &mock_dependencies(&[]).as_ref())
            .expect("expected the asset definition to be valid");
        def
    }

    fn get_valid_update_asset_definition() -> UpdateAssetDefinitionV1 {
        get_update_asset_definition().into()
    }
}
