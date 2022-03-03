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
pub struct AddAssetDefinitionV1 {
    pub asset_definition: AssetDefinition,
}
impl AddAssetDefinitionV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<Self> {
        match msg {
            ExecuteMsg::AddAssetDefinition { asset_definition } => Ok(asset_definition.into()),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::AddAssetDefinition".to_string(),
            }
            .to_err(),
        }
    }
}
impl ResultExtensions for AddAssetDefinitionV1 {}
impl From<AssetDefinitionInput> for AddAssetDefinitionV1 {
    fn from(input: AssetDefinitionInput) -> Self {
        AddAssetDefinitionV1 {
            asset_definition: input.into(),
        }
    }
}
impl From<AssetDefinition> for AddAssetDefinitionV1 {
    fn from(asset_definition: AssetDefinition) -> Self {
        AddAssetDefinitionV1 { asset_definition }
    }
}

pub fn add_asset_definition(
    deps: DepsMutC,
    info: MessageInfo,
    msg: AddAssetDefinitionV1,
) -> ContractResponse {
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    // Ensure that this loan type has not ever yet been added
    if asset_state_read(deps.storage, &msg.asset_definition.asset_type)
        .may_load()?
        .is_some()
    {
        return ContractError::DuplicateAssetDefinitionProvided.to_err();
    }
    asset_state(deps.storage, &msg.asset_definition.asset_type).save(&msg.asset_definition)?;
    Response::new()
        .add_attributes(
            EventAttributes::new(EventType::AddAssetDefinition)
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
    use crate::execute::add_asset_definition::{add_asset_definition, AddAssetDefinitionV1};
    use crate::testutil::test_utilities::{
        single_attribute_for_key, test_instantiate_success, InstArgs, DEFAULT_INFO_NAME,
    };
    use crate::util::aliases::DepsC;
    use crate::util::constants::{ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY};
    use crate::util::event_attributes::EventType;
    use crate::validation::validate_init_msg::validate_asset_definition_input;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;

    const TEST_MOCK_LOAN_TYPE: &str = "fakeloantype";

    #[test]
    fn test_valid_add_asset_definition_via_execute() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let asset_definition = get_valid_asset_definition();
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            ExecuteMsg::AddAssetDefinition {
                asset_definition: asset_definition.clone(),
            },
        )
        .expect("expected the add asset checks to work correctly");
        assert!(
            response.messages.is_empty(),
            "adding an asset definition should not require messages",
        );
        assert_eq!(
            2,
            response.attributes.len(),
            "adding an asset definition should produce the correct number of attributes",
        );
        assert_eq!(
            EventType::AddAssetDefinition.event_name().as_str(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "the proper event type should be emitted",
        );
        assert_eq!(
            TEST_MOCK_LOAN_TYPE,
            single_attribute_for_key(&response, ASSET_TYPE_KEY),
            "the value on the attribute should be the loan type of the added definition",
        );
        test_asset_definition_was_added_for_input(&asset_definition, &deps.as_ref());
    }

    #[test]
    fn test_valid_add_asset_definition_via_internal() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = get_valid_add_asset_definition();
        add_asset_definition(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            msg.clone(),
        )
        .expect("expected the add asset definition function to return properly");
        test_asset_definition_was_added(&msg.asset_definition, &deps.as_ref());
    }

    #[test]
    fn test_invalid_add_asset_definition_for_invalid_msg() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = ExecuteMsg::AddAssetDefinition {
            asset_definition: AssetDefinition::new(String::new(), vec![]).into(),
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
    fn test_invalid_add_asset_definition_for_invalid_sender() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = add_asset_definition(
            deps.as_mut(),
            // Mock info defines the sender with this string - simply use something other than DEFAULT_INFO_NAME to cause the error
            mock_info("not-the-admin", &[]),
            get_valid_add_asset_definition(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::Unauthorized { .. }),
            "expected the unauthorized response to be returned when a different address than the admin is the sender",
        );
    }

    #[test]
    fn test_invalid_add_asset_definition_for_provided_funds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = add_asset_definition(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[coin(150, "nhash")]),
            get_valid_add_asset_definition(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "expected the invalid funds response to be returned when funds are provided to the function",
        );
    }

    #[test]
    fn test_invalid_add_asset_definition_for_duplicate_loan_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let mut add_asset = || {
            add_asset_definition(
                deps.as_mut(),
                mock_info(DEFAULT_INFO_NAME, &[]),
                get_valid_add_asset_definition(),
            )
        };
        add_asset().expect("expected the first asset definition to be added successfully");
        let error = add_asset().unwrap_err();
        assert!(
            matches!(error, ContractError::DuplicateAssetDefinitionProvided),
            "expected the duplicate asset definition response to be returned when the asset definition matches an existing loan type",
        );
    }

    fn test_asset_definition_was_added_for_input(input: &AssetDefinitionInput, deps: &DepsC) {
        test_asset_definition_was_added(&AssetDefinition::from(input), deps)
    }

    fn test_asset_definition_was_added(asset_definition: &AssetDefinition, deps: &DepsC) {
        let state_def = asset_state_read(deps.storage, &asset_definition.asset_type)
            .load()
            .expect("expected the added asset type to be stored in the state");
        assert_eq!(
            asset_definition, &state_def,
            "the value in state should directly equate to the added value",
        );
    }

    fn get_valid_asset_definition() -> AssetDefinitionInput {
        let def = AssetDefinitionInput::new(
            TEST_MOCK_LOAN_TYPE.to_string(),
            vec![ValidatorDetail::new(
                "validator-address".to_string(),
                Uint128::new(1000),
                Decimal::percent(50),
                vec![FeeDestination::new(
                    "fee-address".to_string(),
                    Decimal::percent(100),
                )],
            )],
            None,
        );
        validate_asset_definition_input(&def, &mock_dependencies(&[]).as_ref())
            .expect("expected the asset definition to be valid");
        def
    }

    fn get_valid_add_asset_definition() -> AddAssetDefinitionV1 {
        get_valid_asset_definition().into()
    }
}
