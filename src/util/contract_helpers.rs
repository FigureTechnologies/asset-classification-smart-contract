use crate::core::error::ContractError;
use crate::core::state::config_read;
use crate::util::aliases::{AssetResult, DepsC};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::MessageInfo;

/// Ensures that only the admin of the contract can call into a route.
///
/// # Examples
/// ```
/// use cosmwasm_std::MessageInfo;
/// use asset_classification_smart_contract::util::contract_helpers::check_admin_only;
/// use cosmwasm_std::Addr;
/// use cosmwasm_std::testing::mock_info;
/// use provwasm_mocks::mock_dependencies;
/// use asset_classification_smart_contract::core::state::{config, State};
///
/// let mut deps = mock_dependencies(&[]);
/// config(deps.as_mut().storage).save(&State { base_contract_name: "contract-name".to_string(), admin: Addr::unchecked("admin-name") })
///     .expect("expected state to save successfully");
/// let info = mock_info("admin-name", &[]);
/// check_admin_only(&deps.as_ref(), &info).expect("admin-name was used as the admin and should return a success");
/// ```
pub fn check_admin_only(deps: &DepsC, info: &MessageInfo) -> AssetResult<()> {
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
///
/// let info = mock_info("admin-nmae", &[]);
/// check_funds_are_empty(&info).expect("no coin provided in info - should be success");
/// ```
pub fn check_funds_are_empty(info: &MessageInfo) -> AssetResult<()> {
    if !info.funds.is_empty() {
        ContractError::InvalidFunds("route requires no funds be present".to_string()).to_err()
    } else {
        Ok(())
    }
}
