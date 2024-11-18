use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{load_asset_definition_by_type_v3, replace_asset_definition_v3, STATE_V2};
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::util::aliases::{AssetResult, EntryPointResponse};
use crate::util::contract_helpers::check_funds_are_empty;
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::functions::replace_single_matching_vec_element;

use cosmwasm_std::{DepsMut, MessageInfo, Response};
use result_extensions::ResultExtensions;

/// A transformation of [ExecuteMsg::UpdateAssetVerifier](crate::core::msg::ExecuteMsg::UpdateAssetVerifier)
/// for ease of use in the underlying [update_asset_verifier](self::update_asset_verifier) function.
///
/// # Parameters
///
/// * `asset_type` The unique identifier for the target [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3),
/// keyed on its [asset_type](crate::core::types::asset_definition::AssetDefinitionV3::asset_type)
/// property that the target verifier detail lives in, in its [verifiers](crate::core::types::asset_definition::AssetDefinitionV3::verifiers)
/// property.
/// * `verifier` The verifier detail that will be updated.  All values within this provided struct
/// will replace the existing detail on the target [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3).
#[derive(Clone, PartialEq, Eq)]
pub struct UpdateAssetVerifierV1 {
    pub asset_type: String,
    pub verifier: VerifierDetailV2,
}
impl UpdateAssetVerifierV1 {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `asset_type` The unique identifier for the target [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3),
    /// keyed on its [asset_type](crate::core::types::asset_definition::AssetDefinitionV3::asset_type)
    /// property that the target verifier detail lives in, in its [verifiers](crate::core::types::asset_definition::AssetDefinitionV3::verifiers)
    /// property.
    /// * `verifier` The verifier detail that will be updated.  All values within this provided struct
    /// will replace the existing detail on the target [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3).
    pub fn new<S: Into<String>>(asset_type: S, verifier: VerifierDetailV2) -> Self {
        UpdateAssetVerifierV1 {
            asset_type: asset_type.into(),
            verifier,
        }
    }

    /// Attempts to create an instance of this struct from a provided execute msg.  If the provided
    /// value is not of the [UpdateAssetVerifier](crate::core::msg::ExecuteMsg::UpdateAssetVerifier)
    /// variant, then an [InvalidMessageType](crate::core::error::ContractError::InvalidMessageType)
    /// error will be returned.
    ///
    /// # Parameters
    ///
    /// * `msg` An execute msg provided by the contract's [execute](crate::contract::execute) function.
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<UpdateAssetVerifierV1> {
        match msg {
            ExecuteMsg::UpdateAssetVerifier {
                asset_type,
                verifier,
            } => UpdateAssetVerifierV1::new(asset_type, verifier).to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::UpdateAssetVerifier".to_string(),
            }
            .to_err(),
        }
    }
}

/// The function used by [execute](crate::contract::execute) when an [ExecuteMsg::UpdateAssetVerifier](crate::core::msg::ExecuteMsg::UpdateAssetVerifier)
/// message is provided.  Replaces an existing [VerifierDetailV2](crate::core::types::verifier_detail::VerifierDetailV2)
/// on an existing [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3).
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` An instance of the update asset verifier v1 struct, provided by conversion from an
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).
pub fn update_asset_verifier(
    deps: DepsMut,
    info: MessageInfo,
    msg: UpdateAssetVerifierV1,
) -> EntryPointResponse {
    check_funds_are_empty(&info)?;
    let mut asset_definition = load_asset_definition_by_type_v3(deps.storage, &msg.asset_type)?;
    let state = STATE_V2.load(deps.storage)?;
    if info.sender != state.admin && info.sender.as_str() != msg.verifier.address.as_str() {
        return ContractError::Unauthorized {
            explanation: "admin or verifier required".to_string(),
        }
        .to_err();
    }
    let verifier_address = msg.verifier.address.clone();
    // If a single verifier for the given address cannot be found, data is either corrupt, or the
    // verifier does not exist.  Given validation upfront prevents multiple verifiers with the
    // same address from existing on an asset definition, this generally will indicate that the
    // verifier is outright missing
    if !asset_definition
        .verifiers
        .iter()
        .any(|v| v.address == verifier_address)
    {
        return ContractError::NotFound {
            explanation: format!(
                "verifier with address {} not found for asset definition for type {}. Trying adding this verifier instead",
                msg.verifier.address, asset_definition.asset_type
            ),
        }
        .to_err();
    }
    // Declare the attributes up-front before values are moved
    let attributes = EventAttributes::new(EventType::UpdateAssetVerifier)
        .set_asset_type(&asset_definition.asset_type)
        .set_verifier(&msg.verifier.address);
    // Replace the existing verifier and save the result to the state
    asset_definition.verifiers =
        replace_single_matching_vec_element(asset_definition.verifiers, msg.verifier, |v| {
            v.address == verifier_address
        })?;
    replace_asset_definition_v3(deps.storage, &asset_definition)?;
    // Respond with emitted attributes
    Response::new().add_attributes(attributes).to_ok()
}

#[cfg(test)]
mod tests {
    use crate::contract::execute;
    use crate::core::error::ContractError;
    use crate::core::msg::ExecuteMsg;
    use crate::core::state::load_asset_definition_by_type_v3;
    use crate::core::types::fee_destination::FeeDestinationV2;
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::execute::update_asset_verifier::{update_asset_verifier, UpdateAssetVerifierV1};
    use crate::testutil::test_constants::{
        DEFAULT_ADMIN_ADDRESS, DEFAULT_ASSET_TYPE, DEFAULT_VERIFIER_ADDRESS,
    };
    use crate::testutil::test_utilities::{
        empty_mock_info, get_default_entity_detail, single_attribute_for_key,
        test_instantiate_success, InstArgs,
    };
    use crate::util::constants::{
        ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY, NHASH, VERIFIER_ADDRESS_KEY,
    };
    use crate::util::event_attributes::EventType;
    use crate::util::traits::OptionExtensions;
    use crate::validation::validate_init_msg::validate_verifier;
    use cosmwasm_std::testing::{message_info, mock_env};
    use cosmwasm_std::{coin, Addr, Deps, Uint128};
    use provwasm_mocks::mock_provenance_dependencies;

    #[test]
    fn test_valid_update_asset_verifier_via_execute() {
        // Test that both the admin and the verifier can make the update without being rejected
        for sender_address in vec![DEFAULT_ADMIN_ADDRESS, DEFAULT_VERIFIER_ADDRESS] {
            let mut deps = mock_provenance_dependencies();
            test_instantiate_success(deps.as_mut(), &InstArgs::default());
            let verifier = get_valid_update_verifier();
            let response = execute(
                deps.as_mut(),
                mock_env(),
                empty_mock_info(sender_address),
                ExecuteMsg::UpdateAssetVerifier {
                    asset_type: DEFAULT_ASSET_TYPE.to_string(),
                    verifier: verifier.clone(),
                },
            )
            .expect("expected the update verifier checks to work correctly");
            assert!(
                response.messages.is_empty(),
                "updating an asset verifier should not require messages",
            );
            assert_eq!(
                3,
                response.attributes.len(),
                "the correct number of attributes should be produced",
            );
            assert_eq!(
                EventType::UpdateAssetVerifier.event_name().as_str(),
                single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
                "expected the proper event type to be emitted",
            );
            assert_eq!(
                DEFAULT_ASSET_TYPE,
                single_attribute_for_key(&response, ASSET_TYPE_KEY),
                "expected the update asset verifier main key to include the asset type",
            );
            assert_eq!(
                &verifier.address,
                single_attribute_for_key(&response, VERIFIER_ADDRESS_KEY),
                "expected the verifier's address to be the value for the address key",
            );
            test_default_verifier_was_updated(&verifier, &deps.as_ref());
        }
    }

    #[test]
    fn test_valid_update_asset_verifier_via_internal() {
        // Test that both the admin and the verifier can make the update without being rejected
        for sender_address in vec![DEFAULT_ADMIN_ADDRESS, DEFAULT_VERIFIER_ADDRESS] {
            let mut deps = mock_provenance_dependencies();
            test_instantiate_success(deps.as_mut(), &InstArgs::default());
            let msg = get_valid_update_verifier_msg();
            update_asset_verifier(deps.as_mut(), empty_mock_info(sender_address), msg.clone())
                .expect("expected the update verifier function to return properly");
            test_default_verifier_was_updated(&msg.verifier, &deps.as_ref());
        }
    }

    #[test]
    fn test_invalid_update_asset_verifier_for_invalid_asset_type() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let error = execute(
            deps.as_mut(),
            mock_env(),
            message_info(&Addr::unchecked(DEFAULT_ADMIN_ADDRESS), &[]),
            ExecuteMsg::UpdateAssetVerifier {
                // Invalid because the asset type is missing
                asset_type: String::new(),
                verifier: get_valid_update_verifier(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "when an invalid asset type is provided to execute, the invalid message fields error should be returned, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_update_asset_verifier_for_invalid_msg() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let error = execute(
            deps.as_mut(),
            mock_env(),
            message_info(&Addr::unchecked(DEFAULT_ADMIN_ADDRESS), &[]),
            ExecuteMsg::UpdateAssetVerifier {
                asset_type: DEFAULT_ASSET_TYPE.to_string(),
                verifier: VerifierDetailV2::new(
                    // Invalid because the address is blank
                    "",
                    Uint128::zero(),
                    NHASH,
                    vec![],
                    None,
                    None,
                    None,
                ),
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "when an invalid verifier is provided to execute, the invalid message fields error should be returned, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_update_asset_verifier_for_invalid_sender() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let error = update_asset_verifier(
            deps.as_mut(),
            message_info(&Addr::unchecked("bad-guy"), &[]),
            get_valid_update_verifier_msg(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::Unauthorized { .. }),
            "expected the unauthorized response to be returned when a different address than the admin is the sender, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_update_asset_verifier_for_provided_funds() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let error = update_asset_verifier(
            deps.as_mut(),
            message_info(
                &Addr::unchecked(DEFAULT_ADMIN_ADDRESS),
                &[coin(93849382, "dopehash")],
            ),
            get_valid_update_verifier_msg(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "expected the invalid funds response to be returned when funds are provided to the function, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_update_asset_verifier_for_missing_verifier() {
        let mut deps = mock_provenance_dependencies();
        test_instantiate_success(deps.as_mut(), &InstArgs::default());
        let error = update_asset_verifier(
            deps.as_mut(),
            message_info(&Addr::unchecked(DEFAULT_ADMIN_ADDRESS), &[]),
            UpdateAssetVerifierV1::new(
                DEFAULT_ASSET_TYPE,
                VerifierDetailV2::new(
                    "unknown-address-guy",
                    Uint128::zero(),
                    NHASH,
                    vec![],
                    None,
                    None,
                    None,
                ),
            ),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::NotFound { .. }),
            "the not found error should be returned when the provided update verifier cannot be located in the asset definition, but got: {:?}",
            error,
        );
    }

    fn test_default_verifier_was_updated(verifier: &VerifierDetailV2, deps: &Deps) {
        let state_def = load_asset_definition_by_type_v3(deps.storage, DEFAULT_ASSET_TYPE)
            .expect("expected the default asset type to be stored in the state");
        let target_verifier = state_def.verifiers.into_iter().find(|v| v.address == verifier.address)
            .expect("expected a single verifier to be produced when searching for the updated verifier's address");
        assert_eq!(
            verifier, &target_verifier,
            "expected the verifier stored in state to equate to the updated verifier",
        );
    }

    // This builds off of the existing default asset verifier in test_utilities and adds/tweaks
    // details.  The fee addresses are randomly-generated bech32 provenance testnet addresses
    fn get_valid_update_verifier() -> VerifierDetailV2 {
        let verifier = VerifierDetailV2::new(
            DEFAULT_VERIFIER_ADDRESS,
            Uint128::new(420),
            NHASH,
            vec![
                FeeDestinationV2::new("tp1av6u8yp70mf4f62vx6mzf68pkhut4ets5k4sgx", 105),
                FeeDestinationV2::new("tp169qp36ax8gvtrzszfevqcwhe4hn2g02g35lne8", 105),
            ],
            get_default_entity_detail().to_some(),
            None,
            None,
        );
        validate_verifier(&verifier).expect("expected the verifier to pass validation");
        verifier
    }

    fn get_valid_update_verifier_msg() -> UpdateAssetVerifierV1 {
        UpdateAssetVerifierV1::new(DEFAULT_ASSET_TYPE, get_valid_update_verifier())
    }
}
