use crate::core::asset::ValidatorDetail;
use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{load_asset_definition_by_type, replace_asset_definition};
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::contract_helpers::{check_admin_only, check_funds_are_empty};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};

#[derive(Clone, PartialEq)]
pub struct AddAssetValidatorV1 {
    pub asset_type: String,
    pub validator: ValidatorDetail,
}
impl AddAssetValidatorV1 {
    pub fn new<S: Into<String>>(asset_type: S, validator: ValidatorDetail) -> Self {
        AddAssetValidatorV1 {
            asset_type: asset_type.into(),
            validator,
        }
    }

    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<AddAssetValidatorV1> {
        match msg {
            ExecuteMsg::AddAssetValidator {
                asset_type,
                validator,
            } => AddAssetValidatorV1::new(asset_type, validator).to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::AddAssetValidator".to_string(),
            }
            .to_err(),
        }
    }
}

pub fn add_asset_validator(
    deps: DepsMutC,
    info: MessageInfo,
    msg: AddAssetValidatorV1,
) -> ContractResponse {
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    let mut asset_definition = load_asset_definition_by_type(deps.storage, &msg.asset_type)?;
    // If the asset definition has any validators on it (only ever should be 1 max) with a matching
    // address to the new validator, this request should be an update, not an add
    if asset_definition
        .validators
        .iter()
        .any(|validator| validator.address == msg.validator.address)
    {
        return ContractError::DuplicateValidatorProvided.to_err();
    }
    // Declare all attributes before values are moved
    let attributes = EventAttributes::new(EventType::AddAssetValidator)
        .set_asset_type(&asset_definition.asset_type)
        .set_validator(&msg.validator.address);
    // Store the new validator in the definition and save it to storage
    asset_definition.validators.push(msg.validator);
    replace_asset_definition(deps.storage, &asset_definition)?;
    // Respond with emitted attributes
    Response::new().add_attributes(attributes).to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::contract::execute;
    use crate::core::asset::{FeeDestination, ValidatorDetail};
    use crate::core::error::ContractError;
    use crate::core::msg::ExecuteMsg;
    use crate::core::state::load_asset_definition_by_type;
    use crate::execute::add_asset_validator::{add_asset_validator, AddAssetValidatorV1};
    use crate::testutil::test_utilities::{
        single_attribute_for_key, test_instantiate_success, InstArgs, DEFAULT_ASSET_TYPE,
        DEFAULT_INFO_NAME, DEFAULT_VALIDATOR_ADDRESS,
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
    fn test_valid_add_asset_validator_via_execute() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let validator = get_valid_new_validator();
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            ExecuteMsg::AddAssetValidator {
                asset_type: DEFAULT_ASSET_TYPE.to_string(),
                validator: validator.clone(),
            },
        )
        .expect("expected the add validator function to execute properly");
        assert!(
            response.messages.is_empty(),
            "adding an asset validator should not require messages",
        );
        assert_eq!(
            3,
            response.attributes.len(),
            "adding an asset validator should produce the correct number of attributes",
        );
        assert_eq!(
            EventType::AddAssetValidator.event_name().as_str(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "expected the correct event type to be emitted",
        );
        assert_eq!(
            DEFAULT_ASSET_TYPE,
            single_attribute_for_key(&response, ASSET_TYPE_KEY),
            "expected the default asset type to be used for the main add key",
        );
        assert_eq!(
            &validator.address,
            single_attribute_for_key(&response, VALIDATOR_ADDRESS_KEY),
            "expected the new validator's address to be emitted as an attribute",
        );
        test_default_validator_was_added(&validator, &deps.as_ref());
    }

    #[test]
    fn test_valid_add_asset_validator_via_internal() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = get_add_validator();
        add_asset_validator(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            msg.clone(),
        )
        .expect("expected the add validator function to return properly");
        test_default_validator_was_added(&msg.validator, &deps.as_ref());
    }

    #[test]
    fn test_invalid_add_asset_validator_for_invalid_asset_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            ExecuteMsg::AddAssetValidator {
                // Invalid because the asset type is missing
                asset_type: String::new(),
                validator: get_valid_new_validator(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "when an invalid asset type is provided to execute, the invalid message fields error should be returned",
        );
    }

    #[test]
    fn test_invalid_add_asset_validator_for_invalid_msg() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            ExecuteMsg::AddAssetValidator {
                asset_type: DEFAULT_ASSET_TYPE.to_string(),
                // Invalid because the address is blank
                validator: ValidatorDetail::new(
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
    fn test_invalid_add_asset_validator_for_invalid_sender() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = add_asset_validator(
            deps.as_mut(),
            mock_info("non-admin-person", &[]),
            get_add_validator(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::Unauthorized { .. }),
            "expected the unauthorized response to be returned when a different address than the admin is the sender",
        );
    }

    #[test]
    fn test_invalid_add_asset_validator_for_provided_funds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = add_asset_validator(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[coin(6900, "nhash")]),
            get_add_validator(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "expected the invalid funds response to be returned when funds are provided to the function",
        );
    }

    #[test]
    fn test_invalid_add_asset_validator_for_duplicate_validator_address() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = add_asset_validator(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            AddAssetValidatorV1::new(
                DEFAULT_ASSET_TYPE,
                ValidatorDetail::new(
                    DEFAULT_VALIDATOR_ADDRESS,
                    Uint128::new(100),
                    NHASH,
                    Decimal::percent(0),
                    vec![],
                ),
            ),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::DuplicateValidatorProvided),
            "expected the duplcate validator error to be returned when the validator to be added is already placed on the asset definition",
        );
    }

    // Checks that the validator passed in was added to the default asset type's definition
    fn test_default_validator_was_added(validator: &ValidatorDetail, deps: &DepsC) {
        let state_def = load_asset_definition_by_type(deps.storage, DEFAULT_ASSET_TYPE)
            .expect("expected the default asset type to be stored in the state");
        let target_validator = state_def.validators.into_iter().find(|v| v.address == validator.address)
            .expect("expected a single validator to be produced when searching for the new validator's address");
        assert_eq!(
            validator, &target_validator,
            "expected the validator stored in state to equate to the newly-added validator",
        );
    }

    fn get_valid_new_validator() -> ValidatorDetail {
        let validator = ValidatorDetail::new(
            "new-validator_address",
            Uint128::new(500000),
            NHASH,
            Decimal::percent(10),
            vec![FeeDestination::new("fees", Decimal::percent(100))],
        );
        validate_validator(&validator, &mock_dependencies(&[]).as_ref())
            .expect("expected the new validator to pass validation");
        validator
    }

    fn get_add_validator() -> AddAssetValidatorV1 {
        AddAssetValidatorV1 {
            asset_type: DEFAULT_ASSET_TYPE.to_string(),
            validator: get_valid_new_validator(),
        }
    }
}
