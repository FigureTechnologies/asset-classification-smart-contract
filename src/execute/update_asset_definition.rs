use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{asset_state, asset_state_read, config_read, AssetDefinition};
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::attribute_keys::UPDATE_ASSET_DEFINITION_KEY;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct UpdateAssetDefinitionV1 {
    pub asset_definition: AssetDefinition,
}
impl UpdateAssetDefinitionV1 {
    pub fn new(asset_definition: AssetDefinition) -> Self {
        UpdateAssetDefinitionV1 { asset_definition }
    }

    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<UpdateAssetDefinitionV1> {
        match msg {
            ExecuteMsg::UpdateAssetDefinition { asset_definition } => {
                UpdateAssetDefinitionV1 { asset_definition }.to_ok()
            }
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::UpdateAssetDefinition".to_string(),
            }
            .to_err(),
        }
    }
}
impl ResultExtensions for UpdateAssetDefinitionV1 {}

pub fn update_asset_definition(
    deps: DepsMutC,
    info: MessageInfo,
    msg: UpdateAssetDefinitionV1,
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
            "update an asset definition does not require funds".to_string(),
        )
        .to_err();
    }
    // If the asset definition does not exist within the state, there is nothing to update
    if asset_state_read(deps.storage, &msg.asset_definition.asset_type)
        .may_load()?
        .is_none()
    {
        return ContractError::NotFound {
            explanation: format!(
                "no record exists for asset type {}. please use add asset definition instead",
                msg.asset_definition.asset_type
            ),
        }
        .to_err();
    }
    // Overwrite the existing asset definition with the new one
    asset_state(deps.storage, &msg.asset_definition.asset_type).save(&msg.asset_definition)?;
    Response::new()
        .add_attribute(
            UPDATE_ASSET_DEFINITION_KEY,
            &msg.asset_definition.asset_type,
        )
        .to_ok()
}

#[cfg(test)]
mod tests {
    use crate::contract::execute;
    use crate::core::error::ContractError;
    use crate::core::msg::ExecuteMsg;
    use crate::core::state::{asset_state_read, AssetDefinition, FeeDestination, ValidatorDetail};
    use crate::execute::update_asset_definition::{
        update_asset_definition, UpdateAssetDefinitionV1,
    };
    use crate::testutil::test_utilities::{
        test_instantiate_success, InstArgs, DEFAULT_ASSET_TYPE, DEFAULT_INFO_NAME,
    };
    use crate::util::aliases::DepsC;
    use crate::util::attribute_keys::UPDATE_ASSET_DEFINITION_KEY;
    use crate::validation::validate_init_msg::validate_asset_definition;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_valid_update_asset_definition_via_execute() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let asset_definition = get_update_asset_definition();
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            ExecuteMsg::UpdateAssetDefinition {
                asset_definition: asset_definition.clone(),
            },
        )
        .expect("expected the update asset checks to work correctly");
        assert!(
            response.messages.is_empty(),
            "updating an asset definition should not require messages",
        );
        assert_eq!(
            1,
            response.attributes.len(),
            "adding an asset definition should append a single attribute",
        );
        let attribute = response.attributes.first().unwrap();
        assert_eq!(
            UPDATE_ASSET_DEFINITION_KEY,
            attribute.key.as_str(),
            "the updated asset definition key should be the key on the only attribute",
        );
        assert_eq!(
            DEFAULT_ASSET_TYPE,
            attribute.value.as_str(),
            "the value on the attribute should be the loan type of the updated definition",
        );
        test_asset_definition_was_updated(&asset_definition, &deps.as_ref());
    }

    #[test]
    fn test_valid_update_asset_definition_via_internal() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = get_valid_update_asset_definition();
        update_asset_definition(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            msg.clone(),
        )
        .expect("expected the update asset definition function to return properly");
        test_asset_definition_was_updated(&msg.asset_definition, &deps.as_ref());
    }

    #[test]
    fn test_invalid_update_asset_definition_for_invalid_msg() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = ExecuteMsg::UpdateAssetDefinition {
            asset_definition: AssetDefinition::new(DEFAULT_ASSET_TYPE.to_string(), vec![]),
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
    fn test_invalid_update_asset_definition_for_invalid_sender() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = update_asset_definition(
            deps.as_mut(),
            mock_info("not-the-admin", &[]),
            get_valid_update_asset_definition(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::Unauthorized { .. }),
            "expected the unauthorized response to be returned when a different address than the admin is the sender",
        );
    }

    #[test]
    fn test_invalid_update_asset_definition_for_provided_funds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = update_asset_definition(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[coin(420, "usdf")]),
            get_valid_update_asset_definition(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "expected the invalid funds response to be returned when funds are provided to the function",
        );
    }

    #[test]
    fn test_invalid_update_asset_definition_for_missing_loan_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let missing_asset_definition = AssetDefinition::new(
            "nonexistent-type".to_string(),
            vec![ValidatorDetail::new(
                "validator".to_string(),
                Uint128::new(100),
                Decimal::percent(25),
                vec![FeeDestination::new(
                    "fee-guy".to_string(),
                    Decimal::percent(100),
                )],
            )],
        );
        let error = update_asset_definition(
            deps.as_mut(),
            mock_info(DEFAULT_INFO_NAME, &[]),
            UpdateAssetDefinitionV1::new(missing_asset_definition),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::NotFound { .. }),
            "expected the not found response to be returned when an update is attempted for a definition that does not exist",
        );
    }

    fn test_asset_definition_was_updated(asset_definition: &AssetDefinition, deps: &DepsC) {
        let state_def = asset_state_read(deps.storage, &asset_definition.asset_type)
            .load()
            .expect("expected the updated asset definition to be stored in the state");
        assert_eq!(
            asset_definition, &state_def,
            "the value in state should directly equate to the added value",
        );
    }

    // This builds off of the existing default asset definition in test_utilities and adds/tweaks
    // details
    fn get_update_asset_definition() -> AssetDefinition {
        let def = AssetDefinition {
            asset_type: DEFAULT_ASSET_TYPE.into(),
            validators: vec![ValidatorDetail {
                address: "different-validator-address".to_string(),
                onboarding_cost: Uint128::new(1500000),
                fee_percent: Decimal::percent(50),
                fee_destinations: vec![
                    FeeDestination::new("first".to_string(), Decimal::percent(70)),
                    FeeDestination::new("second".to_string(), Decimal::percent(30)),
                ],
            }],
        };
        validate_asset_definition(&def, &mock_dependencies(&[]).as_ref())
            .expect("expected the asset definition to be valid");
        def
    }

    fn get_valid_update_asset_definition() -> UpdateAssetDefinitionV1 {
        UpdateAssetDefinitionV1::new(get_update_asset_definition())
    }
}
