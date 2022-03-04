use crate::core::error::ContractError;
use crate::core::state::config_read;
use crate::util::aliases::{ContractResult, DepsC};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::MessageInfo;

/// Ensures that only the admin of the contract can call into a route.
///
/// # Examples
/// ```
/// use cosmwasm_std::MessageInfo;
/// use asset_classification_smart_contract::util::contract_helpers::check_admin_only;
/// use cosmwasm_std::testing::mock_info;
/// use provwasm_mocks::mock_dependencies;
/// use asset_classification_smart_contract::testutil::test_utilities::{test_instantiate_success, InstArgs, DEFAULT_INFO_NAME};
///
/// let mut deps = mock_dependencies(&[]);
/// test_instantiate_success(deps.as_mut(), InstArgs::default());
/// let info = mock_info(DEFAULT_INFO_NAME, &[]);
/// check_admin_only(&deps.as_ref(), &info).expect("DEFAULT_INFO_NAME was used as the admin and should return a success");
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
/// use asset_classification_smart_contract::util::contract_helpers::check_funds_are_empty;
/// use cosmwasm_std::testing::mock_info;
/// use asset_classification_smart_contract::testutil::test_utilities::DEFAULT_INFO_NAME;
///
/// let info = mock_info(DEFAULT_INFO_NAME, &[]);
/// check_funds_are_empty(&info).expect("no coin provided in info - should be success");
/// ```
pub fn check_funds_are_empty(info: &MessageInfo) -> ContractResult<()> {
    if !info.funds.is_empty() {
        ContractError::InvalidFunds("route requires no funds be present".to_string()).to_err()
    } else {
        Ok(())
    }
}
