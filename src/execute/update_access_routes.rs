use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::config_read_v2;
use crate::core::types::access_definition::AccessDefinition;
use crate::core::types::access_route::AccessRoute;
use crate::core::types::asset_identifier::AssetIdentifier;
use crate::service::asset_meta_repository::AssetMetaRepository;
use crate::service::deps_manager::DepsManager;
use crate::service::message_gathering_service::MessageGatheringService;
use crate::util::aliases::{AssetResult, EntryPointResponse};
use crate::util::contract_helpers::check_funds_are_empty;
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::functions::filter_valid_access_routes;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};

/// A transformation of [ExecuteMsg::UpdateAccessRoutes](crate::core::msg::ExecuteMsg::UpdateAccessRoutes)
/// for ease of use in the underlying [update_access_routes](self::update_access_routes) function.
///
/// # Parameters
///
/// * `identifier` An instance of the asset identifier enum that helps the contract identify which
/// [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute) that the
/// sender is referring to in the request.
/// * `owner_address` The Provenance Blockchain bech32 address that owns the scope referred to by
/// the [identifier](self::UpdateAccessRoutesV1::identifier).  This must either match the sender, or
/// the sender must be the [contract admin](crate::core::state::StateV2::admin).
/// * `access_routes` A vector of [AccessRoute](crate::core::types::access_route::AccessRoute) to be used
/// instead of the existing routes.  If other existing routes need to be maintained and the updated
/// is intended to simply add a new route, then the existing routes need to be included in the
/// request alongside the new route(s).
#[derive(Clone, PartialEq)]
pub struct UpdateAccessRoutesV1 {
    pub identifier: AssetIdentifier,
    pub owner_address: String,
    pub access_routes: Vec<AccessRoute>,
}
impl UpdateAccessRoutesV1 {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `identifier` An instance of the asset identifier enum that helps the contract identify which
    /// [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute) that the
    /// sender is referring to in the request.
    /// * `owner_address` The Provenance Blockchain bech32 address that owns the scope referred to by
    /// the [identifier](self::UpdateAccessRoutesV1::identifier).  This must either match the sender, or
    /// the sender must be the [contract admin](crate::core::state::StateV2::admin).
    /// * `access_routes` A vector of [AccessRoute](crate::core::types::access_route::AccessRoute) to be used
    /// instead of the existing routes.  If other existing routes need to be maintained and the updated
    /// is intended to simply add a new route, then the existing routes need to be included in the
    /// request alongside the new route(s).
    pub fn new<S: Into<String>>(
        identifier: AssetIdentifier,
        owner_address: S,
        access_routes: Vec<AccessRoute>,
    ) -> Self {
        Self {
            identifier,
            owner_address: owner_address.into(),
            access_routes,
        }
    }

    /// Attempts to create an instance of this struct from a provided execute msg.  If the provided
    /// value is not of the [UpdateAccessRoutes](crate::core::msg::ExecuteMsg::UpdateAccessRoutes)
    /// variant, then an [InvalidMessageType](crate::core::error::ContractError::InvalidMessageType)
    /// error will be returned.
    ///
    /// # Parameters
    ///
    /// * `msg` An execute msg provided by the contract's [execute](crate::contract::execute) function.
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<Self> {
        match msg {
            ExecuteMsg::UpdateAccessRoutes {
                identifier,
                owner_address,
                access_routes,
            } => Self::new(
                identifier.to_asset_identifier()?,
                owner_address,
                access_routes,
            )
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::UpdateAccessRoutes".to_string(),
            }
            .to_err(),
        }
    }
}

/// The function used by [execute](crate::contract::execute) when an [ExecuteMsg::UpdateAccessRoutes](crate::core::msg::ExecuteMsg::UpdateAccessRoutes)
/// message is provided.  Attempts to change the [AccessRoutes](crate::core::types::access_route::AccessRoute)
/// for an [AccessDefinition](crate::core::types::access_definition::AccessDefinition) on a target
/// [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute).
///
/// # Parameters
///
/// * `repository` A helper collection of traits that allows complex lookups of scope values and
/// emits messages to construct the process of updating access routes as a collection of messages
/// to produce in the function's result.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` An instance of the update access routes v1 struct, provided by conversion from an
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).
pub fn update_access_routes<'a, T>(
    repository: T,
    info: MessageInfo,
    msg: UpdateAccessRoutesV1,
) -> EntryPointResponse
where
    T: AssetMetaRepository + MessageGatheringService + DepsManager<'a>,
{
    check_funds_are_empty(&info)?;
    // If the sender is not the specified owner address and the sender is not the admin, they are
    // not authorized to change access routes
    if info.sender != msg.owner_address
        && info.sender
            != repository
                .use_deps(|deps| config_read_v2(deps.storage).load())?
                .admin
    {
        return ContractError::Unauthorized {
            explanation:
                "only the admin or owner of the given access routes can make modifications to them"
                    .to_string(),
        }
        .to_err();
    }
    let mut access_routes = filter_valid_access_routes(msg.access_routes.clone());
    if msg.access_routes.len() != access_routes.len() {
        // The filtration function will trim duplicate routes, as well as invalid routes
        return ContractError::generic("invalid or duplicate access routes were provided").to_err();
    }
    let scope_address = msg.identifier.get_scope_address()?;
    let mut scope_attribute = repository.get_asset(&scope_address)?;
    if let Some(mut target_access_definition) = scope_attribute
        .access_definitions
        .iter()
        .find(|def| def.owner_address == msg.owner_address)
        .map(|def| def.to_owned())
    {
        // Filter the access definition to be changed from the attribute's vector
        scope_attribute.access_definitions = scope_attribute
            .access_definitions
            .into_iter()
            .filter(|def| def != &target_access_definition)
            .collect::<Vec<AccessDefinition>>();
        // Remove all existing access routes on the target definition to change
        target_access_definition.access_routes.clear();
        // Add all access routes from the request into the definition
        target_access_definition
            .access_routes
            .append(&mut access_routes);
        // Append the altered definition to the scope attribute, effectively "replacing" the original record
        scope_attribute
            .access_definitions
            .push(target_access_definition);
        repository.update_attribute(&scope_attribute)?;
    } else {
        // If no access definitions are established for the given owner address, then the request is
        // invalid and should be rejected
        return ContractError::InvalidAddress {
            address: msg.owner_address,
            explanation: format!("scope attribute for address [{scope_address}] does not have access definitions for specified owner"),
        }.to_err();
    }
    Response::new()
        .add_attributes(
            EventAttributes::new(EventType::UpdateAccessRoutes)
                .set_asset_type(&scope_attribute.asset_type)
                .set_scope_address(&scope_address),
        )
        .add_messages(repository.get_messages())
        .to_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::execute;
    use crate::service::asset_meta_service::AssetMetaService;
    use crate::testutil::onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset};
    use crate::testutil::test_constants::{
        DEFAULT_ADMIN_ADDRESS, DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME,
        DEFAULT_SCOPE_ADDRESS, DEFAULT_SENDER_ADDRESS, DEFAULT_VERIFIER_ADDRESS,
    };
    use crate::testutil::test_utilities::{
        assert_single_item, empty_mock_info, setup_test_suite, single_attribute_for_key, InstArgs,
    };
    use crate::testutil::update_access_routes_helpers::{
        test_update_access_routes, TestUpdateAccessRoutes,
    };
    use crate::testutil::verify_asset_helpers::{test_verify_asset, TestVerifyAsset};
    use crate::util::constants::{ASSET_EVENT_TYPE_KEY, ASSET_SCOPE_ADDRESS_KEY, ASSET_TYPE_KEY};
    use crate::util::functions::generate_asset_attribute_name;
    use crate::util::traits::OptionExtensions;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, from_binary, CosmosMsg};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{
        AttributeMsgParams, AttributeValueType, ProvenanceMsg, ProvenanceMsgParams,
    };

    #[test]
    fn test_error_for_provided_funds() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("expected the default asset onboarding to succeed");
        let err = update_access_routes(
            AssetMetaService::new(deps.as_mut()),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[coin(111, "coindollars")]),
            get_valid_update_routes_v1(),
        )
        .expect_err("expected a ContractError to be emitted when funds are provided");
        match err {
            ContractError::InvalidFunds(message) => {
                assert_eq!(
                    "route requires no funds be present", message,
                    "unexpected InvalidFunds message encountered",
                );
            }
            _ => panic!("unexpected error encountered: {:?}", err),
        }
    }

    #[test]
    fn test_error_for_invalid_sender() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("expected the default asset onboarding to succeed");
        let err = update_access_routes(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info("wrong-sender"),
            get_valid_update_routes_v1(),
        )
        .expect_err("expected a ContractError to be emitted when an invalid sender is provided");
        match err {
            ContractError::Unauthorized { explanation } => {
                assert_eq!(
                    "only the admin or owner of the given access routes can make modifications to them",
                    explanation,
                    "unexpected Unauthorized error message encountered",
                );
            }
            _ => panic!("unexpected error encountered: {:?}", err),
        }
    }

    // Doing an update with NO access routes is completely valid, but providing any invalid routes
    // gets rejected to ensure that user error does not result in incorrect output
    #[test]
    fn test_error_for_no_valid_access_routes() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("expected the default asset onboarding to succeed");
        let err = update_access_routes(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            UpdateAccessRoutesV1::new(
                AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                DEFAULT_SENDER_ADDRESS,
                vec![AccessRoute::new("", "".to_some())],
            ),
        )
        .expect_err(
            "expected a ContractError to be emitted when an invalid AccessRoutes are provided",
        );
        match err {
            ContractError::GenericError { msg } => {
                assert_eq!(
                    "invalid or duplicate access routes were provided", msg,
                    "unexpected generic error message countered"
                );
            }
            _ => panic!("unexpected error encountered: {:?}", err),
        }
    }

    #[test]
    fn test_error_for_no_access_definitions_for_owner() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("expected the default asset onboarding to succeed");
        let err = update_access_routes(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            UpdateAccessRoutesV1::new(
                AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                "some random person",
                vec![AccessRoute::new("fakeroute", "something-idk".to_some())],
            )
        ).expect_err(
            "expected a ContractError to be emitted when the specified owner does not have an access definition on the scope",
        );
        match err {
            ContractError::InvalidAddress {
                address,
                explanation,
            } => {
                assert_eq!(
                    "some random person", address,
                    "expected the input address to be used in the error message",
                );
                assert_eq!(
                    format!("scope attribute for address [{DEFAULT_SCOPE_ADDRESS}] does not have access definitions for specified owner"),
                    explanation,
                    "unexpected InvalidAddress explanation encountered",
                );
            }
            _ => panic!("unexpected error encountered: {:?}", err),
        }
    }

    #[test]
    fn test_successful_update_access_routes_by_route_owner() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("expected the default asset onboarding to succeed");
        let attribute_before_update = AssetMetaService::new(deps.as_mut()).get_asset(DEFAULT_SCOPE_ADDRESS).expect(
            "expected a scope attribute to be available for the default address after onboarding",
        );
        assert!(
            attribute_before_update
                .access_definitions
                .iter()
                .any(|def| def.owner_address == DEFAULT_SENDER_ADDRESS),
            "expected an access definition to exist for the sender address",
        );
        let access_route_before_update = assert_single_item(
            &assert_single_item(
                &attribute_before_update.access_definitions,
                "an onboard should leave only a single access definition on the scope attribute",
            )
            .access_routes,
            "only a single access route should be added during onboarding",
        );
        // Use test_update_access_routes to ensure the AssetScopeAttribute changes get recorded and
        // are available after execution
        let response = test_update_access_routes(
            &mut deps,
            TestUpdateAccessRoutes {
                info: empty_mock_info(DEFAULT_SENDER_ADDRESS),
                update_access_routes: get_valid_update_routes_v1(),
            },
        )
        .expect("expected the update to complete successfully");
        assert_eq!(
            1,
            response.messages.len(),
            "expected the update to emit the correct number of messages"
        );
        let expected_attribute_name =
            generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME);
        let attribute_after_update = AssetMetaService::new(deps.as_mut())
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("expected to retrieve the attribute successfully after the access route update is completed");
        response.messages.iter().for_each(|msg| match &msg.msg {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::Attribute(AttributeMsgParams::UpdateAttribute {
                        address,
                        name,
                        original_value,
                        original_value_type,
                        update_value,
                        update_value_type,
                    }),
                ..
            }) => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS,
                    address.to_string(),
                    "the UpdateAttribute should target the scope's address",
                );
                assert_eq!(
                    &expected_attribute_name, name,
                    "the UpdateAttribute should target the default attribute name",
                );
                assert_eq!(
                    attribute_before_update,
                    from_binary(original_value).expect("original value deserialization failure"),
                    "the attribute before the update should be equivalent to the serialized original_value",
                );
                assert_eq!(
                    &AttributeValueType::Json,
                    original_value_type,
                    "the original_value_type should always be json",
                );
                assert_eq!(
                    attribute_after_update,
                    from_binary(update_value).expect("update value deserialization failure"),
                    "the attribute after the update should be equivalent to the serialized update_value",
                );
                assert_eq!(
                    &AttributeValueType::Json,
                    update_value_type,
                    "the update_value_type should always be json",
                );
            }
            _ => panic!(
                "unexpected message emitted during update access routes: {:?}",
                &msg.msg
            ),
        });
        assert_eq!(
            3,
            response.attributes.len(),
            "expected the correct number of attributes to be emitted"
        );
        assert_eq!(
            EventType::UpdateAccessRoutes.event_name(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "expected the correct event type to be emitted",
        );
        assert_eq!(
            DEFAULT_ASSET_TYPE,
            single_attribute_for_key(&response, ASSET_TYPE_KEY),
            "expected the correct asset type to be emitted",
        );
        assert_eq!(
            DEFAULT_SCOPE_ADDRESS,
            single_attribute_for_key(&response, ASSET_SCOPE_ADDRESS_KEY),
            "expected the correct scope address to be emitted",
        );
        assert_eq!(
            attribute_before_update.access_definitions.len(),
            attribute_after_update.access_definitions.len(),
            "the update process should not cause the number of access definitions to change",
        );
        let access_route_after_update = assert_single_item(
            &assert_single_item(
                &attribute_after_update.access_definitions,
                "only a single access definition should exist after the update occurs",
            )
            .access_routes,
            "only a single access route should remain after the update",
        );
        assert_ne!(
            access_route_before_update.route, access_route_after_update.route,
            "the route should be altered after the update",
        );
        let name_after_update = access_route_after_update.name.unwrap();
        assert_ne!(
            &access_route_before_update.name.unwrap(),
            &name_after_update,
            "the name should be altered after the update",
        );
        assert_eq!(
            "grpcs://fake.route:1234", access_route_after_update.route,
            "the route should reflect the value provided during the update",
        );
        assert_eq!(
            "fake_name", name_after_update,
            "the name should reflect the value provided during the update",
        );
    }

    #[test]
    fn test_successful_update_access_routes_by_admin() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("expected the default asset onboarding to succeed");
        test_update_access_routes(
            &mut deps,
            TestUpdateAccessRoutes {
                info: empty_mock_info(DEFAULT_ADMIN_ADDRESS),
                update_access_routes: get_valid_update_routes_v1(),
            },
        )
        .expect("expected the update to complete successfully");
    }

    #[test]
    fn test_successful_update_to_remove_access_routes() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("expected the default asset onboarding to succeed");
        test_update_access_routes(
            &mut deps,
            TestUpdateAccessRoutes {
                info: empty_mock_info(DEFAULT_SENDER_ADDRESS),
                update_access_routes: UpdateAccessRoutesV1::new(
                    AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                    DEFAULT_SENDER_ADDRESS,
                    vec![],
                ),
            },
        )
        .expect("expected the update to complete successfully");
        let scope_attribute = AssetMetaService::new(deps.as_mut())
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("expected the scope attribute to be available after an update");
        let access_definition = assert_single_item(
            &scope_attribute.access_definitions,
            "only one access definition should be available after an onboard and update",
        );
        assert!(
            access_definition.access_routes.is_empty(),
            "expected the access routes to be empty after the update succeeds",
        );
    }

    #[test]
    fn test_successful_update_after_verification_retains_other_access_definitions() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("expected the default asset onboarding to succeed");
        test_verify_asset(&mut deps, TestVerifyAsset::default())
            .expect("expected the default asset verification to succeed");
        let attribute_before_update = AssetMetaService::new(deps.as_mut()).get_asset(DEFAULT_SCOPE_ADDRESS).expect(
            "expected a scope attribute to be available for the default address after onboarding",
        );
        let verifier_definition_before_update = attribute_before_update
            .access_definitions
            .iter()
            .find(|def| def.owner_address == DEFAULT_VERIFIER_ADDRESS)
            .expect("expected an access definition for the verifier to exist");
        let sender_definition_before_update = attribute_before_update
            .access_definitions
            .iter()
            .find(|def| def.owner_address == DEFAULT_SENDER_ADDRESS)
            .expect("expected an access definition for the sender to exist");
        assert_eq!(
            2,
            attribute_before_update.access_definitions.len(),
            "expected the attribute to contain two different access definitions before the update",
        );
        test_update_access_routes(
            &mut deps,
            TestUpdateAccessRoutes {
                info: empty_mock_info(DEFAULT_SENDER_ADDRESS),
                update_access_routes: get_valid_update_routes_v1(),
            },
        )
        .expect("expected the update to complete successfully");
        let attribute_after_update = AssetMetaService::new(deps.as_mut())
            .get_asset(DEFAULT_SCOPE_ADDRESS)
            .expect("expected the scope attribute to be available after the update");
        assert_eq!(
            2,
            attribute_after_update.access_definitions.len(),
            "expected the attribute to still contain both access definitions after the update",
        );
        let verifier_definition_after_update = attribute_after_update
            .access_definitions
            .iter()
            .find(|def| def.owner_address == DEFAULT_VERIFIER_ADDRESS)
            .expect("expected an access definition for the verifier to exist after the update");
        assert_eq!(
            verifier_definition_before_update, verifier_definition_after_update,
            "expected the verifier access definition to be completely unmodified after the update",
        );
        let sender_definition_after_update = attribute_after_update
            .access_definitions
            .iter()
            .find(|def| def.owner_address == DEFAULT_SENDER_ADDRESS)
            .expect("expected an access definition for the sender to exist after the update");
        assert_ne!(
            sender_definition_before_update, sender_definition_after_update,
            "expected the sender's access definition to be changed after the update",
        );
    }

    #[test]
    fn test_successful_update_through_execute_function() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default())
            .expect("expected the default asset onboarding to succeed");
        execute(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            ExecuteMsg::UpdateAccessRoutes {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS)
                    .to_serialized_enum(),
                owner_address: DEFAULT_SENDER_ADDRESS.to_string(),
                access_routes: vec![AccessRoute::new("grpcs://no.u:4433", "some_name".to_some())],
            },
        )
        .expect("expected an update through the execute function to complete successfully");
    }

    fn get_valid_update_routes_v1() -> UpdateAccessRoutesV1 {
        UpdateAccessRoutesV1::new(
            AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
            DEFAULT_SENDER_ADDRESS,
            vec![AccessRoute::new(
                "grpcs://fake.route:1234",
                "fake_name".to_some(),
            )],
        )
    }
}
