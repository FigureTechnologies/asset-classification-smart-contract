use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::asset_state_read;
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Env, MessageInfo, Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OnboardAssetV1 {
    pub asset_uuid: String,
    pub asset_type: String,
    pub scope_address: String,
    pub validator_address: String,
}
impl OnboardAssetV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<OnboardAssetV1> {
        match msg {
            ExecuteMsg::OnboardAsset {
                asset_uuid,
                asset_type,
                scope_address,
                validator_address,
            } => OnboardAssetV1 {
                asset_uuid,
                asset_type,
                scope_address,
                validator_address,
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::OnboardAsset".to_string(),
            }
            .to_err(),
        }
    }
}
impl ResultExtensions for OnboardAssetV1 {}

pub fn onboard_asset(
    deps: DepsMutC,
    _env: Env,
    info: MessageInfo,
    msg: OnboardAssetV1,
) -> ContractResponse {
    // get asset state config for type, or error if not present
    let asset_state = match asset_state_read(deps.storage, &msg.asset_type).load() {
        Ok(state) => state,
        Err(_) => {
            return ContractError::UnsupportedAssetType {
                asset_type: msg.asset_type.to_string(),
            }
            .to_err()
        }
    };

    // verify perscribed validator is present as a validator in asset state
    let validator_config = match asset_state
        .validators
        .into_iter()
        .find(|validator| validator.address == msg.validator_address)
    {
        Some(validator) => validator,
        None => {
            return ContractError::UnsupportedValidator {
                asset_type: msg.asset_type,
                validator_address: msg.validator_address,
            }
            .to_err()
        }
    };

    // verify sent funds match what is specified in the asset state
    if info.funds.len() != 1 {
        return ContractError::InvalidFunds(
            "Exactly one fund type (of nhash) should be sent".to_string(),
        )
        .to_err();
    }
    let sent_fee = match info.funds.into_iter().find(|funds| funds.denom == "nhash") {
        Some(funds) => funds,
        None => {
            return ContractError::InvalidFunds("Funds not supplied for onboarding".to_string())
                .to_err()
        }
    };
    if sent_fee.amount != validator_config.onboarding_cost {
        return ContractError::InvalidFunds(format!(
            "Improper fee of {}{} provided (expected {}nhash)",
            sent_fee.amount, sent_fee.denom, validator_config.onboarding_cost
        ))
        .to_err();
    };

    // store asset metadata in contract storage, with assigned validator and provided fee (in case fee changes between onboarding and validation)

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_env;
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::error::ContractError,
        testutil::test_utilities::{
            mock_info_with_nhash, test_instantiate, InstArgs, DEFAULT_VALIDATOR_ADDRESS,
        },
    };

    use super::{onboard_asset, OnboardAssetV1};

    #[test]
    fn test_onboard_asset_errors_on_unsupported_asset_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        let err = onboard_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(1000),
            OnboardAssetV1 {
                asset_uuid: "1234".into(),
                asset_type: "bogus".into(),
                scope_address: "scope1234".into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.into(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::UnsupportedAssetType { asset_type } => {
                assert_eq!(
                    "bogus", asset_type,
                    "the unsupported asset type message should reflect the type provided"
                )
            }
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }
}
