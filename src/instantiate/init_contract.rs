use crate::core::msg::InitMsg;
use crate::core::state::{asset_state, config, AssetDefinition, State};
use crate::migrate::version_info::migrate_version_info;
use crate::util::aliases::{ContractResponse, DepsMutC};
use crate::util::contract_helpers::check_funds_are_empty;
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::functions::generate_asset_attribute_name;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{CosmosMsg, Env, MessageInfo, Response};
use provwasm_std::{bind_name, NameBinding, ProvenanceMsg};

pub fn init_contract(
    deps: DepsMutC,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> ContractResponse {
    check_funds_are_empty(&info)?;
    // The contract needs to own its root name to be effective at preventing asset classification "neighbors"
    // that were never intended to be created from being reserved by external callers
    let mut messages: Vec<CosmosMsg<ProvenanceMsg>> = vec![bind_name(
        &msg.base_contract_name,
        env.contract.address.clone(),
        NameBinding::Restricted,
    )?];
    // Note: This vector can remain empty on instantiation, and future executions by the admin can
    // append new definitions. When no definitions are supplied, this contract will not be able to
    // take execution input until they are
    for input in msg.asset_definitions.iter() {
        let asset_definition: AssetDefinition = input.into();
        // Create a new state storage for the provided asset definition
        asset_state(deps.storage, &asset_definition.asset_type).save(&asset_definition)?;
        messages.push(bind_name(
            generate_asset_attribute_name(&asset_definition.asset_type, &msg.base_contract_name),
            env.contract.address.clone(),
            NameBinding::Restricted,
        )?);
    }
    // Convert the init message into a state value that will drive the contract's future executions
    let state = State::new(msg, info.sender);
    // Store the state by grabbing a mutable instance of the contract configuration
    config(deps.storage).save(&state)?;
    // Set the version info to the default contract values on instantiation
    migrate_version_info(deps.storage)?;
    Response::new()
        .add_messages(messages)
        .add_attributes(EventAttributes::new(EventType::InstantiateContract))
        .to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::contract::instantiate;
    use crate::core::error::ContractError;
    use crate::core::msg::{AssetDefinitionInput, InitMsg};
    use crate::core::state::{asset_state_read, FeeDestination, ValidatorDetail};
    use crate::migrate::version_info::{get_version_info, CONTRACT_NAME, CONTRACT_VERSION};
    use crate::testutil::msg_utilities::{test_for_default_base_name, test_message_is_name_bind};
    use crate::testutil::test_utilities::{
        get_default_asset_definition_inputs, single_attribute_for_key, test_instantiate, InstArgs,
        DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME, DEFAULT_INFO_NAME, DEFAULT_ONBOARDING_COST,
        DEFAULT_VALIDATOR_ADDRESS,
    };
    use crate::util::constants::ASSET_EVENT_TYPE_KEY;
    use crate::util::event_attributes::EventType;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::{coin, Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_valid_default_init() {
        let mut deps = mock_dependencies(&[]);
        let response = test_instantiate(deps.as_mut(), InstArgs::default())
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
        let asset_state = asset_state_read(deps.as_ref().storage, DEFAULT_ASSET_TYPE)
            .load()
            .expect("expected the asset state data should be added to storage");
        assert_eq!(
            DEFAULT_ASSET_TYPE, asset_state.asset_type,
            "the asset type should be stored correctly",
        );
        assert_eq!(
            1,
            asset_state.validators.len(),
            "one validator should be properly stored",
        );
        assert_eq!(
            asset_state,
            get_default_asset_definition_inputs()
                .first()
                .unwrap()
                .to_owned()
                .into(),
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
        let mut deps = mock_dependencies(&[]);
        let first_asset_def = AssetDefinitionInput::new(
            "heloc".to_string(),
            vec![ValidatorDetail::new(
                DEFAULT_VALIDATOR_ADDRESS.into(),
                DEFAULT_ONBOARDING_COST.into(),
                Decimal::percent(50),
                vec![FeeDestination::new(
                    "first".to_string(),
                    Decimal::percent(100),
                )],
            )],
            None,
        );
        let second_asset_def = AssetDefinitionInput::new(
            "mortgage".to_string(),
            vec![ValidatorDetail::new(
                "other-address".to_string(),
                Uint128::new(150),
                Decimal::percent(100),
                vec![
                    FeeDestination::new("first".to_string(), Decimal::percent(50)),
                    FeeDestination::new("second".to_string(), Decimal::percent(50)),
                ],
            )],
            None,
        );
        let response = test_instantiate(
            deps.as_mut(),
            InstArgs {
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
        let heloc_asset_state = asset_state_read(deps.as_ref().storage, "heloc")
            .load()
            .expect("the heloc asset definition should be added to the state");
        assert_eq!(
            heloc_asset_state,
            first_asset_def.into(),
            "the heloc asset state should equate to the heloc input"
        );
        let mortgage_asset_state = asset_state_read(deps.as_ref().storage, "mortgage")
            .load()
            .expect("the mortgage asset definition should be added to the state");
        assert_eq!(
            mortgage_asset_state,
            second_asset_def.into(),
            "the mortgage asset state should equate to the mortgage input"
        );
    }

    #[test]
    fn test_invalid_init_contract_including_funds() {
        let mut deps = mock_dependencies(&[]);
        let error = test_instantiate(
            deps.as_mut(),
            InstArgs {
                info: mock_info(DEFAULT_INFO_NAME, &[coin(100, "nhash")]),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "the responding error should indicate invalid funds",
        );
    }

    #[test]
    fn test_invalid_init_fails_for_invalid_init_msg() {
        let args = InstArgs {
            asset_definitions: vec![AssetDefinitionInput::new(String::new(), vec![], None)],
            ..Default::default()
        };
        let error = instantiate(
            mock_dependencies(&[]).as_mut(),
            args.env,
            args.info,
            InitMsg {
                base_contract_name: DEFAULT_CONTRACT_BASE_NAME.to_string(),
                asset_definitions: args.asset_definitions,
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "the responding error should indicate that the InitMsg was badly formatted",
        );
    }
}
