use crate::core::{error::ContractError, state::STATE_V2};
use crate::util::aliases::AssetResult;

use cosmwasm_std::{Addr, Deps, MessageInfo};
use provwasm_std::types::provenance::msgfees::v1::MsgAssessCustomMsgFeeRequest;
use result_extensions::ResultExtensions;

use super::functions::validate_address;

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
/// use provwasm_mocks::mock_provenance_dependencies;
/// use asset_classification_smart_contract::core::state::{STATE_V2, StateV2};
///
/// let mut deps = mock_provenance_dependencies();
/// STATE_V2.save(deps.as_mut().storage, &StateV2 { base_contract_name: "contract-name".to_string(), admin: Addr::unchecked("admin-name"), is_test: false })
///     .expect("expected state to save successfully");
/// let info = mock_info("admin-name", &[]);
/// check_admin_only(&deps.as_ref(), &info).expect("admin-name was used as the admin and should return a success");
/// ```
pub fn check_admin_only(deps: &Deps, info: &MessageInfo) -> AssetResult<()> {
    let state = STATE_V2.load(deps.storage)?;
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

/// Creates a message for charging a custom fee.
///
/// # Parameters
///
/// * `amount` The amount to be charged
/// * `name` An optional name for the fee
/// * `from` The payer of the fee
/// * `recipient` An optional recipient of the fee
pub fn assess_custom_fee<S: Into<String>>(
    amount: cosmwasm_std::Coin,
    name: Option<S>,
    from: Addr,
    recipient: Option<Addr>,
) -> Result<cosmwasm_std::CosmosMsg, cosmwasm_std::StdError> {
    let coin = provwasm_std::types::cosmos::base::v1beta1::Coin {
        denom: amount.denom,
        amount: amount.amount.to_string(),
    };

    Ok(MsgAssessCustomMsgFeeRequest {
        name: name.map(|s| s.into()).unwrap_or("".to_string()),
        amount: Some(coin),
        recipient: recipient.unwrap_or(Addr::unchecked("")).to_string(),
        from: validate_address(from)?.to_string(),
        recipient_basis_points: "10000".to_string(),
    }
    .into())
}
