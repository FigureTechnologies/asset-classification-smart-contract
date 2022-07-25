use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::util::aliases::{AssetResult, DepsMutC, EntryPointResponse};
use crate::util::contract_helpers::{check_admin_only, check_funds_are_empty};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Env, MessageInfo, Response};
use provwasm_std::{bind_name, NameBinding};

/// A transformation of [ExecuteMsg::BindContractAlias](crate::core::msg::ExecuteMsg::BindContractAlias)
/// for ease of use in the underlying [bind_contract_alias](self::bind_contract_alias) function.
///
/// # Parameters
///
/// * `alias_name` The Provenance Name Module fully-qualified name to have the contract bind to
/// itself in the [bind_contract_alias](self::bind_contract_alias) function.
#[derive(Clone, PartialEq)]
pub struct BindContractAliasV1 {
    pub alias_name: String,
}
impl BindContractAliasV1 {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `alias_name` The Provenance Name Module fully-qualified name to have the contract bind to
    /// itself in the [bind_contract_alias](self::bind_contract_alias) function.
    pub fn new<S: Into<String>>(alias_name: S) -> Self {
        Self {
            alias_name: alias_name.into(),
        }
    }

    /// Attempts to create an instance of this struct from a provided execute msg.  If the provided
    /// value is not of the [BindContractAlias](crate::core::msg::ExecuteMsg::BindContractAlias)
    /// variant, then an [InvalidMessageType](crate::core::error::ContractError::InvalidMessageType)
    /// error will be returned.
    ///
    /// # Parameters
    ///
    /// * `msg` An execute msg provided by the contract's [execute](crate::contract::execute) function.
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<BindContractAliasV1> {
        match msg {
            ExecuteMsg::BindContractAlias { alias_name } => {
                BindContractAliasV1::new(alias_name).to_ok()
            }
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::BindContractAlias".to_string(),
            }
            .to_err(),
        }
    }
}

/// Route implementation for [ExecuteMsg::BindContractAlias](crate::core::msg::ExecuteMsg::BindContractAlias).
/// This function allows the contract to bind a name to itself using the admin address.
/// Note: Due to the way Provenance names work, this route will only work when attempting to self-bind
/// to an unrestricted parent name.  Binding an alias to a restricted parent name will still require
/// that the address that owns the parent name signs the name binding message.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `env` An environment object provided by the cosmwasm framework.  Describes the contract's
/// details, as well as blockchain information at the time of the transaction.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` An instance of the bind contract alias v1 struct, provided by conversion from an
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).
pub fn bind_contract_alias(
    deps: DepsMutC,
    env: Env,
    info: MessageInfo,
    msg: BindContractAliasV1,
) -> EntryPointResponse {
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    Response::new()
        .add_attributes(
            EventAttributes::new(EventType::BindContractAlias).set_new_value(&msg.alias_name),
        )
        .add_message(bind_name(
            &msg.alias_name,
            env.contract.address,
            NameBinding::Restricted,
        )?)
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::contract::execute;
    use crate::core::error::ContractError;
    use crate::core::msg::ExecuteMsg;
    use crate::execute::bind_contract_alias::{bind_contract_alias, BindContractAliasV1};
    use crate::testutil::test_constants::DEFAULT_ADMIN_ADDRESS;
    use crate::testutil::test_utilities::{
        empty_mock_info, mock_info_with_funds, single_attribute_for_key, test_instantiate_success,
        InstArgs,
    };
    use crate::util::aliases::EntryPointResponse;
    use crate::util::constants::{ASSET_EVENT_TYPE_KEY, NEW_VALUE_KEY};
    use crate::util::event_attributes::EventType;
    use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, CosmosMsg};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{NameMsgParams, ProvenanceMsg, ProvenanceMsgParams};

    #[test]
    fn test_valid_bind_contract_alias_via_execute() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        test_valid_response(
            execute(
                deps.as_mut(),
                mock_env(),
                empty_mock_info(DEFAULT_ADMIN_ADDRESS),
                ExecuteMsg::BindContractAlias {
                    alias_name: "somealias.pb".to_string(),
                },
            ),
            "somealias.pb",
        );
    }

    #[test]
    fn test_valid_bind_contract_alias_via_direct() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        test_valid_response(
            bind_contract_alias(
                deps.as_mut(),
                mock_env(),
                empty_mock_info(DEFAULT_ADMIN_ADDRESS),
                BindContractAliasV1::new("anotheralias.pio"),
            ),
            "anotheralias.pio",
        );
    }

    #[test]
    fn test_invalid_bind_contract_alias_for_non_admin_sender() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let err = bind_contract_alias(
            deps.as_mut(),
            mock_env(),
            empty_mock_info("some-other-guy"),
            BindContractAliasV1::new("aliasnamething.test"),
        )
        .expect_err(
            "expected an error to occur when attempting to bind an alias using a non-admin account",
        );
        assert!(
            matches!(err, ContractError::Unauthorized { .. }),
            "unexpected error emitted when using a non-admin account to bind an alias: {:?}",
            err,
        );
    }

    #[test]
    fn test_invalid_bind_contract_alias_for_funds_provided() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let err = bind_contract_alias(
            deps.as_mut(),
            mock_env(),
            mock_info_with_funds(DEFAULT_ADMIN_ADDRESS, &[coin(150, "nhash")]),
            BindContractAliasV1::new("google.com"),
        )
        .expect_err(
            "expected an error to occur when attempting to bind an alias and sending funds",
        );
        assert!(
            matches!(err, ContractError::InvalidFunds(..)),
            "unexpected error emitted when sending funds to bind an alias: {:?}",
            err,
        );
    }

    fn test_valid_response<S: Into<String>>(response: EntryPointResponse, expected_bound_name: S) {
        let response = response.expect("expected binding the alias to succeed");
        let expected_alias: String = expected_bound_name.into();
        assert_eq!(
            1,
            response.messages.len(),
            "expected the correct amount of messages to be added",
        );
        match &response.messages.first().unwrap().msg {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Name(NameMsgParams::BindName {
                        name,
                        address,
                        restrict,
                    }),
                ..
            }) => {
                assert_eq!(
                    &expected_alias, name,
                    "expected the correct name to be bound",
                );
                assert_eq!(
                    MOCK_CONTRACT_ADDR,
                    address.as_str(),
                    "expected the contract's address to be bound to the name",
                );
                assert!(restrict, "expected the alias address to be restricted",);
            }
            msg => panic!("unexpected message encountered after bind alias: {:?}", msg),
        }
        assert_eq!(
            2,
            response.attributes.len(),
            "expected the correct number of attributes to be added",
        );
        assert_eq!(
            EventType::BindContractAlias.event_name(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "expected the correct event type attribute to be added",
        );
        assert_eq!(
            expected_alias,
            single_attribute_for_key(&response, NEW_VALUE_KEY),
            "expected the correct new value attribute to be added",
        );
    }
}
