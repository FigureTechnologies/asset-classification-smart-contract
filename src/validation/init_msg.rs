use crate::core::error::ContractError;
use crate::core::msg::InitMsg;
use cosmwasm_std::{Decimal, Deps};
use provwasm_std::ProvenanceQuery;
use crate::util::traits::ResultExtensions;

pub fn validate_init_msg(msg: &InitMsg, deps: &Deps<ProvenanceQuery>) -> Result<(), ContractError> {
    let mut invalid_fields: Vec<String> = vec![];
    if msg.contract_name.is_empty() {
        invalid_fields.push("contract_name: cannot be blank".to_string());
    }
    if deps.api.addr_validate(&msg.fee_collection_address).is_err() {
        invalid_fields.push("fee_collection_address: must be a valid address".to_string());
    }
    if msg.fee_percent > Decimal::percent(100) {
        invalid_fields.push("fee_percent: must be less than or equal to 100 percent".to_string());
    }
    if !invalid_fields.is_empty() {
        ContractError::InvalidMessageFields {
            message_type: "Instantiate".to_string(),
            invalid_fields,
        }
        .to_err()
    } else {
        Ok(())
    }
}
