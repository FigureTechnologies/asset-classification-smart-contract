use crate::core::error::ContractError;
use crate::core::state::config_read;
use crate::util::aliases::{ContractResult, DepsC};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::MessageInfo;

/// Ensures that only the admin of the contract can call into a route.
///
/// # Examples
/// ```
/// use asset_classification_smart_contract::util::aliases::{ContractResponse, DepsMutC};
/// use cosmwasm_std::{MessageInfo, Response};
/// use asset_classification_smart_contract::util::contract_helpers::check_admin_only;
/// use asset_classification_smart_contract::util::traits::ResultExtensions;
/// use cosmwasm_std::testing::mock_info;
/// use provwasm_mocks::mock_dependencies;
/// use asset_classification_smart_contract::testutil::test_utilities::DEFAULT_INFO_NAME;
///
/// fn route(deps: DepsMutC, info: MessageInfo, msg: String) -> ContractResponse {
///     check_admin_only(&deps.as_ref(), &info)?;
///     Response::new().to_ok()
/// }
/// let mut deps = mock_dependencies(&[]);
/// let info = mock_info(DEFAULT_INFO_NAME, &[]);
/// route(deps.as_mut(), info, "something".to_string()).expect("should be the admin because tests use DEFAULT_INFO_NAME");
/// ```
pub fn check_admin_only(deps: &DepsC, info: &MessageInfo) -> ContractResult<()> {
    let state = config_read(deps.storage).load()?;
    if info.sender != state.admin {
        ContractError::Unauthorized {
            explanation: "admin required".to_string(),
        }
        .to_err()
    } else {
        Ok(())
    }
}

/// Ensures that the info provided to the route does not include any funds.
///
/// # Examples
/// ```
/// use asset_classification_smart_contract::util::aliases::{ContractResponse, DepsMutC};
/// use cosmwasm_std::{MessageInfo, Response};
/// use asset_classification_smart_contract::util::contract_helpers::check_funds_are_empty;
/// use asset_classification_smart_contract::util::traits::ResultExtensions;
/// use cosmwasm_std::testing::mock_info;
/// use provwasm_mocks::mock_dependencies;
/// use asset_classification_smart_contract::testutil::test_utilities::DEFAULT_INFO_NAME;
///
/// fn route(deps: DepsMutC, info: MessageInfo, msg: String) -> ContractResponse {
///     check_funds_are_empty(&info)?;
///     Response::new().to_ok()
/// }
/// let mut deps = mock_dependencies(&[]);
/// let info = mock_info(DEFAULT_INFO_NAME, &[]);
/// route(deps.as_mut(), info, "idk".to_string()).expect("no funds were provided so this should unwrap correctly");
/// ```
pub fn check_funds_are_empty(info: &MessageInfo) -> ContractResult<()> {
    if !info.funds.is_empty() {
        ContractError::InvalidFunds("route requires no funds be present".to_string()).to_err()
    } else {
        Ok(())
    }
}
