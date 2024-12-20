use crate::core::msg::InitMsg;
use crate::core::state::{insert_asset_definition_v3, StateV2, STATE_V2};
use crate::migrate::version_info::migrate_version_info;
use crate::util::aliases::EntryPointResponse;
use crate::util::contract_helpers::check_funds_are_empty;
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::functions::{generate_asset_attribute_name, msg_bind_name};

use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response};
use result_extensions::ResultExtensions;

/// The main functionality executed when the smart contract is first instantiated.   This creates
/// the internal contract [StateV2](crate::core::state::StateV2) value, as well as any provided
/// [AssetDefinitionsV3](crate::core::types::asset_definition::AssetDefinitionV3) provided in the init
/// msg.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `env` An environment object provided by the cosmwasm framework.  Describes the contract's
/// details, as well as blockchain information at the time of the transaction.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` A custom instantiation message defined by this contract for creating the initial
/// configuration used by the contract.
pub fn init_contract(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> EntryPointResponse {
    check_funds_are_empty(&info)?;
    let mut messages: Vec<CosmosMsg> = vec![];
    // If specified true, the contract needs to own its root name to be effective at preventing
    // asset classification "neighbors" that were never intended to be created from being reserved
    // by external callers
    if msg.bind_base_name {
        messages.push(CosmosMsg::from(msg_bind_name(
            &msg.base_contract_name,
            env.contract.address.clone(),
            true,
        )?));
    }
    // Note: This vector can remain empty on instantiation, and future executions by the admin can
    // append new definitions. When no definitions are supplied, this contract will not be able to
    // take execution input until they are
    for input in msg.asset_definitions.iter() {
        let asset_definition = input.as_asset_definition();
        // Create a new state storage for the provided asset definition
        insert_asset_definition_v3(deps.storage, &asset_definition)?;
        // Default to true for name bind if no value is specified.
        if input.bind_name.unwrap_or(true) {
            messages.push(CosmosMsg::from(msg_bind_name(
                generate_asset_attribute_name(
                    &asset_definition.asset_type,
                    &msg.base_contract_name,
                ),
                env.contract.address.clone(),
                true,
            )?));
        }
    }
    // Convert the init message into a state value that will drive the contract's future executions
    let state = StateV2::new(msg, info.sender);
    // Store the state by grabbing a mutable instance of the contract configuration
    STATE_V2.save(deps.storage, &state)?;
    // Set the version info to the default contract values on instantiation
    migrate_version_info(deps.storage)?;
    Response::new()
        .add_messages(messages)
        .add_attributes(EventAttributes::new(EventType::InstantiateContract))
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::contract::instantiate;
    use crate::core::error::ContractError;
    use crate::core::msg::InitMsg;
    use crate::core::state::{load_asset_definition_by_type_v3, STATE_V2};
    use crate::core::types::asset_definition::AssetDefinitionInputV3;
    use crate::core::types::fee_destination::FeeDestinationV2;
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::migrate::version_info::{get_version_info, CONTRACT_NAME, CONTRACT_VERSION};
    use crate::testutil::msg_utilities::{test_for_default_base_name, test_message_is_name_bind};
    use crate::testutil::test_constants::{
        DEFAULT_ADMIN_ADDRESS, DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME,
        DEFAULT_ONBOARDING_COST, DEFAULT_ONBOARDING_DENOM, DEFAULT_VERIFIER_ADDRESS,
    };
    use crate::testutil::test_utilities::{
        get_default_asset_definition, get_default_asset_definition_inputs,
        get_default_entity_detail, get_default_verifier_detail, single_attribute_for_key,
        test_instantiate, InstArgs,
    };
    use crate::util::constants::{ASSET_EVENT_TYPE_KEY, NHASH};
    use crate::util::event_attributes::EventType;
    use crate::util::traits::OptionExtensions;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{coin, Addr, Uint128};

    #[test]
    fn test_valid_default_init() {
        let mut deps = mock_dependencies();
        let response = test_instantiate(deps.as_mut(), &InstArgs::default())
            .expect("the default instantiate should produce a response without error");
        assert_eq!(
            1,
            response.attributes.len(),
            "a single attribute should be emitted"
        );
        assert_eq!(
            EventType::InstantiateContract.event_name().as_str(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "the proper event type should be emitted",
        );
        assert_eq!(
            2,
            response.messages.len(),
            "the correct number of messages should be emitted"
        );
        test_for_default_base_name(&response.messages);
        test_message_is_name_bind(&response.messages, DEFAULT_ASSET_TYPE);
        let asset_state =
            load_asset_definition_by_type_v3(deps.as_ref().storage, DEFAULT_ASSET_TYPE)
                .expect("expected the asset state data should be added to storage");
        assert_eq!(
            DEFAULT_ASSET_TYPE, asset_state.asset_type,
            "the asset type should be stored correctly",
        );
        assert_eq!(
            1,
            asset_state.verifiers.len(),
            "one verifier should be properly stored",
        );
        assert_eq!(
            asset_state,
            get_default_asset_definition(),
            "the returned value should directly match the default asset definition"
        );
        let version_info = get_version_info(deps.as_ref().storage)
            .expect("version info should successfully load after instantiation");
        assert_eq!(
            CONTRACT_NAME, version_info.contract,
            "the contract name should be properly stored after a successful instantiation",
        );
        assert_eq!(
            CONTRACT_VERSION, version_info.version,
            "the contract version should be properly stored after a successful instantiation",
        );
    }

    #[test]
    fn test_valid_init_with_multiple_asset_definitions() {
        let mut deps = mock_dependencies();
        let first_asset_def = AssetDefinitionInputV3::new(
            "heloc",
            "Home Equity Line of Street Cred".to_some(),
            vec![VerifierDetailV2::new(
                DEFAULT_VERIFIER_ADDRESS,
                DEFAULT_ONBOARDING_COST.into(),
                DEFAULT_ONBOARDING_DENOM,
                vec![FeeDestinationV2::new(
                    "tp18c94z83e6ng2sc3ylvutzytlx8zqggm554xp5a",
                    DEFAULT_ONBOARDING_COST / 4,
                )],
                get_default_entity_detail().to_some(),
                None,
                None,
            )],
            None,
            None,
        );
        let second_asset_def = AssetDefinitionInputV3::new(
            "mortgage",
            "DEATH PLEDGE".to_some(),
            vec![VerifierDetailV2::new(
                "tp1n6zl5u3x4k2uq29a5rxvh8g339wnk8j7v2sxdq",
                Uint128::new(300),
                NHASH,
                vec![
                    FeeDestinationV2::new("tp18c94z83e6ng2sc3ylvutzytlx8zqggm554xp5a", 75),
                    FeeDestinationV2::new("tp1haa4tyccy0278tt9lckvu42a2g6fzjlh4vuydn", 75),
                ],
                get_default_entity_detail().to_some(),
                None,
                None,
            )],
            None,
            None,
        );
        let response = test_instantiate(
            deps.as_mut(),
            &InstArgs {
                asset_definitions: vec![first_asset_def.clone(), second_asset_def.clone()],
                ..Default::default()
            },
        )
        .expect("instantiation should succeed with multiple asset definitions");
        assert_eq!(
            1,
            response.attributes.len(),
            "only one attribute should be emitted"
        );
        assert_eq!(
            3,
            response.messages.len(),
            "the correct number of messages should be emitted",
        );
        test_for_default_base_name(&response.messages);
        test_message_is_name_bind(&response.messages, "heloc");
        test_message_is_name_bind(&response.messages, "mortgage");
        let heloc_asset_state = load_asset_definition_by_type_v3(deps.as_ref().storage, "heloc")
            .expect("the heloc asset definition should be added to the state");
        assert_eq!(
            heloc_asset_state,
            first_asset_def.into_asset_definition(),
            "the heloc asset state should equate to the heloc input"
        );
        let mortgage_asset_state =
            load_asset_definition_by_type_v3(deps.as_ref().storage, "mortgage")
                .expect("the mortgage asset definition should be added to the state");
        assert_eq!(
            mortgage_asset_state,
            second_asset_def.into_asset_definition(),
            "the mortgage asset state should equate to the mortgage input"
        );
    }

    #[test]
    fn test_valid_init_bind_base_name_false_skips_base_bind() {
        let mut deps = mock_dependencies();
        let response = test_instantiate(
            deps.as_mut(),
            &InstArgs {
                bind_base_name: false,
                ..Default::default()
            },
        )
        .expect("instantiation with defaults and bind_base_name = false should succeed");
        assert_eq!(
            1,
            response.messages.len(),
            "the correct number of messages should be emitted"
        );
        // The only message emitted should be a name bind for the default asset type to the base name
        test_message_is_name_bind(&response.messages, DEFAULT_ASSET_TYPE);
    }

    #[test]
    fn test_valid_init_no_is_test_flag_supplied_defaults_to_false() {
        let mut deps = mock_dependencies();
        instantiate(
            deps.as_mut(),
            mock_env(),
            message_info(&Addr::unchecked(DEFAULT_ADMIN_ADDRESS), &[]),
            InitMsg {
                base_contract_name: DEFAULT_CONTRACT_BASE_NAME.to_string(),
                bind_base_name: true,
                asset_definitions: get_default_asset_definition_inputs(),
                is_test: None,
            },
        )
        .expect("instantiation should complete successfully");
        let state = STATE_V2
            .load(deps.as_ref().storage)
            .expect("state v2 should be created by instantiation");
        assert!(
            !state.is_test,
            "is_test should default to false when no value is provided by the caller",
        );
    }

    #[test]
    fn test_valid_init_with_false_name_bind_on_added_definition() {
        let mut deps = mock_dependencies();
        let response = instantiate(
            deps.as_mut(),
            mock_env(),
            message_info(&Addr::unchecked(DEFAULT_ADMIN_ADDRESS), &[]),
            InitMsg {
                base_contract_name: DEFAULT_CONTRACT_BASE_NAME.to_string(),
                bind_base_name: false,
                asset_definitions: vec![AssetDefinitionInputV3::new(
                    DEFAULT_ASSET_TYPE,
                    None::<String>,
                    vec![get_default_verifier_detail()],
                    true.to_some(),
                    // bind_name == false
                    false.to_some(),
                )],
                is_test: None,
            },
        )
        .expect("expected instantiation to succeed with no name binding on the added definition");
        assert!(
            response.messages.is_empty(),
            "no messages should be added when no bindings are added"
        );
        load_asset_definition_by_type_v3(deps.as_ref().storage, DEFAULT_ASSET_TYPE)
            .expect("the asset definition should be added, regardless of name binding");
    }

    #[test]
    fn test_invalid_init_contract_including_funds() {
        let mut deps = mock_dependencies();
        let error = test_instantiate(
            deps.as_mut(),
            &InstArgs {
                info: message_info(
                    &Addr::unchecked(DEFAULT_ADMIN_ADDRESS),
                    &[coin(100, "nhash")],
                ),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "the responding error should indicate invalid funds, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_init_fails_for_invalid_init_msg() {
        let args = InstArgs {
            asset_definitions: vec![AssetDefinitionInputV3::new(
                "",
                None::<String>,
                vec![],
                None,
                None,
            )],
            ..Default::default()
        };
        let error = instantiate(
            mock_dependencies().as_mut(),
            args.env,
            args.info,
            InitMsg {
                base_contract_name: DEFAULT_CONTRACT_BASE_NAME.to_string(),
                bind_base_name: true,
                asset_definitions: args.asset_definitions,
                is_test: None,
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "the responding error should indicate that the InitMsg was badly formatted, but got: {:?}",
            error,
        );
    }
}
