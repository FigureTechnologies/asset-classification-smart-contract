use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{insert_asset_definition_v3, STATE_V2};
use crate::core::types::asset_definition::AssetDefinitionV3;
use crate::util::aliases::{AssetResult, EntryPointResponse};
use crate::util::contract_helpers::{check_admin_only, check_funds_are_empty};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::functions::{generate_asset_attribute_name, msg_bind_name};

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use result_extensions::ResultExtensions;

/// A transformation of [ExecuteMsg::AddAssetDefinition](crate::core::msg::ExecuteMsg::AddAssetDefinition)
/// for ease of use in the underlying [add_asset_definition](self::add_asset_definition) function.
///
/// # Parameters
///
/// * `asset_definition` The asset definition to add to the internal storage.  Must have a unique
/// [asset_type](crate::core::types::asset_definition::AssetDefinitionV3::asset_type).
/// * `bind_name` An optional parameter.  If omitted or provided as `true`, the contract will attempt
/// to bind a name branched off of its [base_contract_name](crate::core::state::StateV2::base_contract_name)
/// with the provided definition's [asset_type](crate::core::types::asset_definition::AssetDefinitionV3::asset_type).
#[derive(Clone, PartialEq, Eq)]
pub struct AddAssetDefinitionV1 {
    pub asset_definition: AssetDefinitionV3,
    pub bind_name: Option<bool>,
}
impl AddAssetDefinitionV1 {
    /// Attempts to create an instance of this struct from a provided execute msg.  If the provided
    /// value is not of the [AddAssetDefinition](crate::core::msg::ExecuteMsg::AddAssetDefinition)
    /// variant, then an [InvalidMessageType](crate::core::error::ContractError::InvalidMessageType)
    /// error will be returned.
    ///
    /// # Parameters
    ///
    /// * `msg` An execute msg provided by the contract's [execute](crate::contract::execute) function.
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<Self> {
        match msg {
            ExecuteMsg::AddAssetDefinition { asset_definition } => Self {
                bind_name: asset_definition.bind_name,
                asset_definition: asset_definition.into_asset_definition(),
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::AddAssetDefinition".to_string(),
            }
            .to_err(),
        }
    }
}

/// The function used by [execute](crate::contract::execute) when an [ExecuteMsg::AddAssetDefinition](crate::core::msg::ExecuteMsg::AddAssetDefinition)
/// message is provided.  Attempts to add a new [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3)
/// to the contract's internal storage.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `env` An environment object provided by the cosmwasm framework.  Describes the contract's
/// details, as well as blockchain information at the time of the transaction.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` An instance of the add asset definition v1 struct, provided by conversion from an
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).
pub fn add_asset_definition(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: AddAssetDefinitionV1,
) -> EntryPointResponse {
    // Verify that the admin is making this call and no funds are provided
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    // The insert function includes its own checking to verify that the asset definition does not yet exist, and an error
    // will be returned if a duplicate is attempted
    insert_asset_definition_v3(deps.storage, &msg.asset_definition)?;
    let mut messages = vec![];
    // If requested, or the bind_name param is omitted, bind the new asset type's name the contract in order to be able
    // to write new attributes for onboarded scopes
    if msg.bind_name.unwrap_or(true) {
        messages.push(msg_bind_name(
            generate_asset_attribute_name(
                &msg.asset_definition.asset_type,
                STATE_V2.load(deps.storage)?.base_contract_name,
            ),
            env.contract.address,
            true,
        )?);
    }

    Response::new()
        .add_messages(messages)
        .add_attributes(
            EventAttributes::new(EventType::AddAssetDefinition)
                .set_asset_type(&msg.asset_definition.asset_type),
        )
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::contract::execute;
    use crate::core::error::ContractError;
    use crate::core::msg::ExecuteMsg;
    use crate::core::state::load_asset_definition_by_type_v3;
    use crate::core::types::asset_definition::{AssetDefinitionInputV3, AssetDefinitionV3};
    use crate::core::types::fee_destination::FeeDestinationV2;
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::execute::add_asset_definition::{add_asset_definition, AddAssetDefinitionV1};
    use crate::testutil::msg_utilities::test_message_is_name_bind;
    use crate::testutil::test_constants::{
        DEFAULT_ADMIN_ADDRESS, DEFAULT_FEE_ADDRESS, DEFAULT_VERIFIER_ADDRESS,
    };
    use crate::testutil::test_utilities::{
        empty_mock_info, get_default_entity_detail, single_attribute_for_key,
        test_instantiate_success, InstArgs,
    };
    use crate::util::constants::{ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY, NHASH};
    use crate::util::event_attributes::EventType;
    use crate::util::traits::OptionExtensions;
    use crate::validation::validate_init_msg::validate_asset_definition_input;
    use cosmwasm_std::testing::{message_info, mock_env};
    use cosmwasm_std::{coin, Addr, Deps, Uint128};
    use provwasm_mocks::mock_provenance_dependencies;

    // These tests board a new asset type, so they need values other than the default to work with
    const TEST_ASSET_TYPE: &str = "add_asset_type";

    #[test]
    fn test_valid_add_asset_definition_via_execute() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
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
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let msg = get_valid_add_asset_definition(true);
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
    fn test_valid_add_asset_definition_skip_name_binding_from_execute() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let mut asset_definition = get_valid_asset_definition();
        asset_definition.bind_name = false.to_some();
        let response = execute(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ExecuteMsg::AddAssetDefinition {
                asset_definition: asset_definition.clone(),
            },
        )
        .expect("expected the add asset definition function to return properly");
        assert!(
            response.messages.is_empty(),
            "when no name binding is requested, no messages should be emitted"
        );
        test_asset_definition_was_added(&asset_definition.into_asset_definition(), &deps.as_ref());
    }

    #[test]
    fn test_valid_add_asset_definition_skip_name_binding_from_direct() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let msg = get_valid_add_asset_definition(false);
        let messages = add_asset_definition(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            msg.clone(),
        )
        .expect("expected the add asset definition function to return properly")
        .messages;
        assert!(
            messages.is_empty(),
            "when no name binding is requested, no messages should be emitted"
        );
        test_asset_definition_was_added(&msg.asset_definition, &deps.as_ref());
    }

    #[test]
    fn test_invalid_add_asset_definition_for_invalid_msg() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let msg = ExecuteMsg::AddAssetDefinition {
            asset_definition: AssetDefinitionInputV3::new(
                "",
                None::<String>,
                vec![],
                true.to_some(),
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
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let error = add_asset_definition(
            deps.as_mut(),
            mock_env(),
            // Mock info defines the sender with this string - simply use something other than DEFAULT_INFO_NAME to cause the error
            message_info(&Addr::unchecked("not-the-admin"), &[]),
            get_valid_add_asset_definition(true),
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
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let error = add_asset_definition(
            deps.as_mut(),
            mock_env(),
            message_info(
                &Addr::unchecked(DEFAULT_ADMIN_ADDRESS),
                &[coin(150, "nhash")],
            ),
            get_valid_add_asset_definition(true),
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
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let mut add_asset = || {
            add_asset_definition(
                deps.as_mut(),
                mock_env(),
                empty_mock_info(DEFAULT_ADMIN_ADDRESS),
                get_valid_add_asset_definition(true),
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

    fn test_asset_definition_was_added_for_input(input: &AssetDefinitionInputV3, deps: &Deps) {
        test_asset_definition_was_added(&input.as_asset_definition(), deps)
    }

    fn test_asset_definition_was_added(asset_definition: &AssetDefinitionV3, deps: &Deps) {
        let state_def =
            load_asset_definition_by_type_v3(deps.storage, &asset_definition.asset_type)
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

    fn get_valid_asset_definition() -> AssetDefinitionInputV3 {
        let def = AssetDefinitionInputV3::new(
            TEST_ASSET_TYPE,
            Some("TEST YO'SELF"),
            // Defining the verifier to be the same as the default values is fine, because
            // it is realistic that different asset types might use the same verifiers
            vec![VerifierDetailV2::new(
                DEFAULT_VERIFIER_ADDRESS,
                Uint128::new(1000),
                NHASH,
                vec![FeeDestinationV2::new(DEFAULT_FEE_ADDRESS, 500)],
                get_default_entity_detail().to_some(),
                None,
                None,
            )],
            None,
            None,
        );
        validate_asset_definition_input(&def).expect("expected the asset definition to be valid");
        def
    }

    fn get_valid_add_asset_definition(bind_name: bool) -> AddAssetDefinitionV1 {
        AddAssetDefinitionV1 {
            asset_definition: get_valid_asset_definition().into_asset_definition(),
            bind_name: bind_name.to_some(),
        }
    }
}
