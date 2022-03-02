use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{asset_state, asset_state_read, config_read, AssetDefinition};
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::attribute_keys::ADD_ASSET_DEFINITION_KEY;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct AddAssetDefinitionV1 {
    pub asset_definition: AssetDefinition,
}
impl AddAssetDefinitionV1 {
    pub fn new(asset_definition: AssetDefinition) -> Self {
        AddAssetDefinitionV1 { asset_definition }
    }

    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<AddAssetDefinitionV1> {
        match msg {
            ExecuteMsg::AddAssetDefinition { asset_definition } => {
                AddAssetDefinitionV1 { asset_definition }.to_ok()
            }
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::AddAssetDefinition".to_string(),
            }
            .to_err(),
        }
    }
}
impl ResultExtensions for AddAssetDefinitionV1 {}

pub fn add_asset_definition(
    deps: DepsMutC,
    info: MessageInfo,
    msg: AddAssetDefinitionV1,
) -> ContractResponse {
    // Ensure only the admin is attempting to call this route
    let state = config_read(deps.storage).load()?;
    if info.sender != state.admin {
        return ContractError::Unauthorized {
            explanation: "admin required".to_string(),
        }
        .to_err();
    }
    // This function requires no funds to process - we don't want excess amounts left in the contract
    if !info.funds.is_empty() {
        return ContractError::InvalidFunds(
            "adding an asset definition does not require funds".to_string(),
        )
        .to_err();
    }
    // Ensure that this loan type has not ever yet been added
    if asset_state_read(deps.storage, &msg.asset_definition.asset_type)
        .may_load()?
        .is_some()
    {
        return ContractError::DuplicateAssetDefinitionProvided.to_err();
    }
    asset_state(deps.storage, &msg.asset_definition.asset_type).save(&msg.asset_definition)?;
    Response::new()
        .add_attribute(ADD_ASSET_DEFINITION_KEY, &msg.asset_definition.asset_type)
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::contract::execute;
    use crate::core::error::ContractError;
    use crate::core::msg::ExecuteMsg;
    use crate::core::state::{asset_state_read, AssetDefinition, FeeDestination, ValidatorDetail};
    use crate::execute::add_asset_definition::{add_asset_definition, AddAssetDefinitionV1};
    use crate::testutil::test_utilities::{test_instantiate_success, InstArgs, DEFAULT_INFO_NAME};
    use crate::util::aliases::DepsC;
    use crate::util::attribute_keys::ADD_ASSET_DEFINITION_KEY;
    use crate::validation::validate_init_msg::validate_asset_definition;
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
            1,
            response.attributes.len(),
            "adding an asset definition should append a single attribute",
        );
        let attribute = response.attributes.first().unwrap();
        assert_eq!(
            ADD_ASSET_DEFINITION_KEY,
            attribute.key.as_str(),
            "the add asset definition key should be the key on the only attribute",
        );
        assert_eq!(
            TEST_MOCK_LOAN_TYPE,
            attribute.value.as_str(),
            "the value on the attribute should be the loan type of the added definition",
        );
        test_asset_definition_was_added(&asset_definition, &deps.as_ref());
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
            asset_definition: AssetDefinition::new(String::new(), vec![]),
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

    fn test_asset_definition_was_added(asset_definition: &AssetDefinition, deps: &DepsC) {
        let state_def = asset_state_read(deps.storage, &asset_definition.asset_type)
            .load()
            .expect("expected the added asset type to be stored in the state");
        assert_eq!(
            asset_definition, &state_def,
            "the value in state should directly equate to the added value",
        );
    }

    fn get_valid_asset_definition() -> AssetDefinition {
        let def = AssetDefinition::new(
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
        );
        validate_asset_definition(&def, &mock_dependencies(&[]).as_ref())
            .expect("expected the asset definition to be valid");
        def
    }

    fn get_valid_add_asset_definition() -> AddAssetDefinitionV1 {
        AddAssetDefinitionV1::new(get_valid_asset_definition())
    }
}
