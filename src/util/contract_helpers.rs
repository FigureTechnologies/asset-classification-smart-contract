use crate::core::error::ContractError;
use crate::core::state::config_read_v2;
use crate::util::aliases::{AssetResult, DepsC};

use cosmwasm_std::MessageInfo;
use result_extensions::ResultExtensions;

/// Ensures that only the admin of the contract can call into a route.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
///
/// # Examples
/// ```
/// use cosmwasm_std::MessageInfo;
/// use asset_classification_smart_contract::util::contract_helpers::check_admin_only;
/// use cosmwasm_std::Addr;
/// use cosmwasm_std::testing::mock_info;
/// use provwasm_mocks::mock_dependencies;
/// use asset_classification_smart_contract::core::state::{config_v2, StateV2};
///
/// let mut deps = mock_dependencies(&[]);
/// config_v2(deps.as_mut().storage).save(&StateV2 { base_contract_name: "contract-name".to_string(), admin: Addr::unchecked("admin-name"), is_test: false })
///     .expect("expected state to save successfully");
/// let info = mock_info("admin-name", &[]);
/// check_admin_only(&deps.as_ref(), &info).expect("admin-name was used as the admin and should return a success");
/// ```
pub fn check_admin_only(deps: &DepsC, info: &MessageInfo) -> AssetResult<()> {
    let state = config_read_v2(deps.storage).load()?;
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
/// # Parameters
///
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
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
