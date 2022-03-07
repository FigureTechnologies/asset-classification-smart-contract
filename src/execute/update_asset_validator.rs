use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{asset_state, ValidatorDetail};
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::contract_helpers::{check_admin_only, check_funds_are_empty};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::functions::replace_single_matching_vec_element;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};

#[derive(Clone, PartialEq)]
pub struct UpdateAssetValidatorV1 {
    pub asset_type: String,
    pub validator: ValidatorDetail,
}
impl UpdateAssetValidatorV1 {
    pub fn new<S: Into<String>>(asset_type: S, validator: ValidatorDetail) -> Self {
        UpdateAssetValidatorV1 {
            asset_type: asset_type.into(),
            validator,
        }
    }

    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<UpdateAssetValidatorV1> {
        match msg {
            ExecuteMsg::UpdateAssetValidator {
                asset_type,
                validator,
            } => UpdateAssetValidatorV1::new(asset_type, validator).to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::UpdateAssetValidator".to_string(),
            }
            .to_err(),
        }
    }
}
impl ResultExtensions for UpdateAssetValidatorV1 {}

pub fn update_asset_validator(
    deps: DepsMutC,
    info: MessageInfo,
    msg: UpdateAssetValidatorV1,
) -> ContractResponse {
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    let mut asset_state = asset_state(deps.storage, &msg.asset_type);
    let mut asset_definition = asset_state.load()?;
    let validator_address = msg.validator.address.clone();
    // If a single validator for the given address cannot be found, data is either corrupt, or the
    // validator does not exist.  Given validation upfront prevents multiple validators with the
    // same address from existing on an asset definition, this generally will indicate that the
    // validator is outright missing
    if !asset_definition
        .validators
        .iter()
        .any(|v| v.address == validator_address)
    {
        return ContractError::NotFound {
            explanation: format!(
                "validator with address {} not found for asset definition for type {}. Trying adding this validator instead",
                msg.validator.address, asset_definition.asset_type
            ),
        }
        .to_err();
    }
    // Declare the attributes up-front before values are moved
    let attributes = EventAttributes::new(EventType::UpdateAssetValidator)
        .set_asset_type(&asset_definition.asset_type)
        .set_validator(&msg.validator.address);
    // Replace the existing validator and save the result to the state
    asset_definition.validators =
        replace_single_matching_vec_element(asset_definition.validators, msg.validator, |v| {
            v.address == validator_address
        })?;
    asset_state.save(&asset_definition)?;
    // Respond with emitted attributes
    Response::new().add_attributes(attributes).to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::contract::execute;
    use crate::core::error::ContractError;
    use crate::core::msg::ExecuteMsg;
    use crate::core::state::{asset_state_read, FeeDestination, ValidatorDetail};
    use crate::execute::update_asset_validator::{update_asset_validator, UpdateAssetValidatorV1};
    use crate::testutil::test_utilities::{
        empty_mock_info, single_attribute_for_key, test_instantiate_success, InstArgs,
        DEFAULT_ASSET_TYPE, DEFAULT_INFO_NAME, DEFAULT_VALIDATOR_ADDRESS,
    };
    use crate::util::aliases::DepsC;
    use crate::util::constants::{
        ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY, NHASH, VALIDATOR_ADDRESS_KEY,
    };
    use crate::util::event_attributes::EventType;
    use crate::validation::validate_init_msg::validate_validator;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_valid_update_asset_validator_via_execute() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let validator = get_valid_update_validator();
        let response = execute(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(),
            ExecuteMsg::UpdateAssetValidator {
                asset_type: DEFAULT_ASSET_TYPE.to_string(),
                validator: validator.clone(),
            },
        )
        .expect("expected the update validator checks to work correctly");
        assert!(
            response.messages.is_empty(),
            "updating an asset validator should not require messages",
        );
        assert_eq!(
            3,
            response.attributes.len(),
            "the correct number of attributes should be produced",
        );
        assert_eq!(
            EventType::UpdateAssetValidator.event_name().as_str(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "expected the proper event type to be emitted",
        );
        assert_eq!(
            DEFAULT_ASSET_TYPE,
            single_attribute_for_key(&response, ASSET_TYPE_KEY),
            "expected the update asset validator main key to include the asset type",
        );
        assert_eq!(
            &validator.address,
            single_attribute_for_key(&response, VALIDATOR_ADDRESS_KEY),
            "expected the validator's address to be the value for the address key",
        );
        test_default_validator_was_updated(&validator, &deps.as_ref());
    }

    #[test]
    fn test_valid_update_asset_validator_via_internal() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = get_valid_update_validator_msg();
        update_asset_validator(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            msg.clone(),
        )
        .expect("expected the update validator function to return properly");
        test_default_validator_was_updated(&msg.validator, &deps.as_ref());
    }

    #[test]
    fn test_invalid_update_asset_validator_for_invalid_asset_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            ExecuteMsg::UpdateAssetValidator {
                // Invalid because the asset type is missing
                asset_type: String::new(),
                validator: get_valid_update_validator(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "when an invalid asset type is provided to execute, the invalid message fields error should be returned",
        );
    }

    #[test]
    fn test_invalid_update_asset_validator_for_invalid_msg() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            ExecuteMsg::UpdateAssetValidator {
                asset_type: DEFAULT_ASSET_TYPE.to_string(),
                validator: ValidatorDetail::new(
                    // Invalid because the address is blank
                    "",
                    Uint128::new(0),
                    NHASH,
                    Decimal::percent(0),
                    vec![],
                ),
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "when an invalid validator is provided to execute, the invalid message fields error should be returned",
        );
    }

    #[test]
    fn test_invalid_update_asset_validator_for_invalid_sender() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = update_asset_validator(
            deps.as_mut(),
            mock_info("bad-guy", &[]),
            get_valid_update_validator_msg(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::Unauthorized { .. }),
            "expected the unauthorized response to be returned when a different address than the admin is the sender",
        );
    }

    #[test]
    fn test_invalid_update_asset_validator_for_provided_funds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = update_asset_validator(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[coin(93849382, "dopehash")]),
            get_valid_update_validator_msg(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "expected the invalid funds response to be returned when funds are provided to the function",
        );
    }

    #[test]
    fn test_invalid_update_asset_validator_for_missing_validator() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = update_asset_validator(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            UpdateAssetValidatorV1::new(
                DEFAULT_ASSET_TYPE,
                ValidatorDetail::new(
                    "unknown-address-guy",
                    Uint128::new(100),
                    NHASH,
                    Decimal::percent(0),
                    vec![],
                ),
            ),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::NotFound { .. }),
            "the not found error should be returned when the provided update validator cannot be located in the asset definition",
        );
    }

    fn test_default_validator_was_updated(validator: &ValidatorDetail, deps: &DepsC) {
        let state_def = asset_state_read(deps.storage, DEFAULT_ASSET_TYPE)
            .load()
            .expect("expected the default asset type to be stored in the state");
        let target_validator = state_def.validators.into_iter().find(|v| v.address == validator.address)
            .expect("expected a single validator to be produced when searching for the updated validator's address");
        assert_eq!(
            validator, &target_validator,
            "expected the validator stored in state to equate to the updated validator",
        );
    }

    // This builds off of the existing default asset validator in test_utilities and adds/tweaks
    // details
    fn get_valid_update_validator() -> ValidatorDetail {
        let validator = ValidatorDetail::new(
            DEFAULT_VALIDATOR_ADDRESS,
            Uint128::new(420),
            NHASH,
            Decimal::percent(100),
            vec![
                FeeDestination::new("first", Decimal::percent(50)),
                FeeDestination::new("second", Decimal::percent(50)),
            ],
        );
        validate_validator(&validator, &mock_dependencies(&[]).as_ref())
            .expect("expected the validator to pass validation");
        validator
    }

    fn get_valid_update_validator_msg() -> UpdateAssetValidatorV1 {
        UpdateAssetValidatorV1::new(DEFAULT_ASSET_TYPE, get_valid_update_validator())
    }
}
