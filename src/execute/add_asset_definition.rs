use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{config_read_v2, insert_asset_definition};
use crate::core::types::asset_definition::AssetDefinition;
use crate::util::aliases::{AssetResult, DepsMutC, EntryPointResponse};
use crate::util::contract_helpers::{check_admin_only, check_funds_are_empty};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::functions::generate_asset_attribute_name;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Env, MessageInfo, Response};
use provwasm_std::{bind_name, NameBinding};

#[derive(Clone, PartialEq)]
pub struct AddAssetDefinitionV1 {
    pub asset_definition: AssetDefinition,
}
impl AddAssetDefinitionV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<Self> {
        match msg {
            ExecuteMsg::AddAssetDefinition { asset_definition } => Self {
                asset_definition: asset_definition.into_asset_definition()?,
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::AddAssetDefinition".to_string(),
            }
            .to_err(),
        }
    }
}

pub fn add_asset_definition(
    deps: DepsMutC,
    env: Env,
    info: MessageInfo,
    msg: AddAssetDefinitionV1,
) -> EntryPointResponse {
    // Verify that the admin is making this call and no funds are provided
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    // The insert function includes its own checking to verify that the asset definition does not yet exist, and an error
    // will be returned if a duplicate is attempted
    insert_asset_definition(deps.storage, &msg.asset_definition)?;
    // Bind the new asset type's name the contract in order to be able to write new attributes for onboarded scopes
    let name_msg = bind_name(
        generate_asset_attribute_name(
            &msg.asset_definition.asset_type,
            config_read_v2(deps.storage).load()?.base_contract_name,
        ),
        env.contract.address,
        NameBinding::Restricted,
    )?;
    Response::new()
        .add_message(name_msg)
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
    use crate::core::msg::ExecuteMsg;
    use crate::core::state::load_asset_definition_by_type;
    use crate::core::types::asset_definition::{AssetDefinition, AssetDefinitionInput};
    use crate::core::types::fee_destination::FeeDestination;
    use crate::core::types::scope_spec_identifier::ScopeSpecIdentifier;
    use crate::core::types::verifier_detail::VerifierDetail;
    use crate::execute::add_asset_definition::{add_asset_definition, AddAssetDefinitionV1};
    use crate::testutil::msg_utilities::test_message_is_name_bind;
    use crate::testutil::test_constants::{
        DEFAULT_ADMIN_ADDRESS, DEFAULT_FEE_ADDRESS, DEFAULT_SCOPE_SPEC_ADDRESS,
        DEFAULT_VERIFIER_ADDRESS,
    };
    use crate::testutil::test_utilities::{
        empty_mock_info, get_default_entity_detail, single_attribute_for_key,
        test_instantiate_success, InstArgs,
    };
    use crate::util::aliases::DepsC;
    use crate::util::constants::{ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY, NHASH};
    use crate::util::event_attributes::EventType;
    use crate::util::traits::OptionExtensions;
    use crate::validation::validate_init_msg::validate_asset_definition_input;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;

    // These tests board a new asset type, so they need values other than the default to work with
    const TEST_ASSET_TYPE: &str = "add_asset_type";
    const TEST_SCOPE_SPEC_ADDRESS: &str = "scopespec1q3ptevdt2x5yg5ycflqjsky8rz5q47e34p";

    #[test]
    fn test_valid_add_asset_definition_via_execute() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let asset_definition = get_valid_asset_definition();
        let response = execute(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ExecuteMsg::AddAssetDefinition {
                asset_definition: asset_definition.clone(),
            },
        )
        .expect("expected the add asset checks to work correctly");
        assert_eq!(
            1,
            response.messages.len(),
            "the proper number of messages should be added",
        );
        test_message_is_name_bind(&response.messages, &asset_definition.asset_type);
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
            TEST_ASSET_TYPE,
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
        let messages = add_asset_definition(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            msg.clone(),
        )
        .expect("expected the add asset definition function to return properly")
        .messages;
        test_message_is_name_bind(&messages, &msg.asset_definition.asset_type);
        test_asset_definition_was_added(&msg.asset_definition, &deps.as_ref());
    }

    #[test]
    fn test_invalid_add_asset_definition_for_invalid_msg() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = ExecuteMsg::AddAssetDefinition {
            asset_definition: AssetDefinitionInput::new(
                "",
                ScopeSpecIdentifier::address(DEFAULT_SCOPE_SPEC_ADDRESS),
                vec![],
                true.to_some(),
            ),
        };
        let error = execute(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            msg,
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "expected an invalid asset definition to cause an InvalidMessageFields error, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_add_asset_definition_for_invalid_sender() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = add_asset_definition(
            deps.as_mut(),
            mock_env(),
            // Mock info defines the sender with this string - simply use something other than DEFAULT_INFO_NAME to cause the error
            mock_info("not-the-admin", &[]),
            get_valid_add_asset_definition(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::Unauthorized { .. }),
            "expected the unauthorized response to be returned when a different address than the admin is the sender, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_add_asset_definition_for_provided_funds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = add_asset_definition(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[coin(150, "nhash")]),
            get_valid_add_asset_definition(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "expected the invalid funds response to be returned when funds are provided to the function, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_add_asset_definition_for_duplicate_loan_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let mut add_asset = || {
            add_asset_definition(
                deps.as_mut(),
                mock_env(),
                empty_mock_info(DEFAULT_ADMIN_ADDRESS),
                get_valid_add_asset_definition(),
            )
        };
        add_asset().expect("expected the first asset definition to be added successfully");
        let error = add_asset().unwrap_err();
        assert!(
            matches!(error, ContractError::RecordAlreadyExists { .. }),
            "expected the duplicate asset definition response to be returned when the asset definition matches an existing loan type, but got: {:?}",
            error,
        );
    }

    fn test_asset_definition_was_added_for_input(input: &AssetDefinitionInput, deps: &DepsC) {
        test_asset_definition_was_added(
            &input
                .as_asset_definition()
                .expect("asset definition conversion should succeed"),
            deps,
        )
    }

    fn test_asset_definition_was_added(asset_definition: &AssetDefinition, deps: &DepsC) {
        let state_def = load_asset_definition_by_type(deps.storage, &asset_definition.asset_type)
            .expect("expected the added asset type to be stored in the state");
        assert_eq!(
            asset_definition, &state_def,
            "the value in state should directly equate to the added value",
        );
        assert_eq!(
            true, state_def.enabled,
            "the default value for added values should be enabled = true",
        );
    }

    fn get_valid_asset_definition() -> AssetDefinitionInput {
        let def = AssetDefinitionInput::new(
            TEST_ASSET_TYPE,
            ScopeSpecIdentifier::address(TEST_SCOPE_SPEC_ADDRESS),
            // Defining the verifier to be the same as the default values is fine, because
            // it is realistic that different asset types might use the same verifiers
            vec![VerifierDetail::new(
                DEFAULT_VERIFIER_ADDRESS,
                Uint128::new(1000),
                NHASH,
                Decimal::percent(50),
                vec![FeeDestination::new(
                    DEFAULT_FEE_ADDRESS,
                    Decimal::percent(100),
                )],
                get_default_entity_detail().to_some(),
            )],
            None,
        );
        validate_asset_definition_input(&def).expect("expected the asset definition to be valid");
        def
    }

    fn get_valid_add_asset_definition() -> AddAssetDefinitionV1 {
        AddAssetDefinitionV1 {
            asset_definition: get_valid_asset_definition()
                .into_asset_definition()
                .expect("asset definition conversion should succeed"),
        }
    }
}
