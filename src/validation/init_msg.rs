use crate::core::error::ContractError;
use crate::core::msg::InitMsg;
use crate::core::state::{AssetDefinition, ValidatorDetail};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Decimal, Deps};
use provwasm_std::ProvenanceQuery;

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
    if msg.asset_definitions.is_empty() {
        invalid_fields.push("asset_definitions: must not be empty".to_string());
    }
    let mut asset_messages = msg
        .asset_definitions
        .iter()
        .flat_map(|asset| validate_asset_definition(&asset, deps))
        .collect::<Vec<String>>();
    invalid_fields.append(&mut asset_messages);
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

fn validate_asset_definition(
    asset_definition: &AssetDefinition,
    deps: &Deps<ProvenanceQuery>,
) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if asset_definition.asset_type.is_empty() {
        invalid_fields.push("asset_type: must not be blank".to_string());
    }
    if asset_definition.validators.is_empty() {
        invalid_fields
            .push("validators: at least one validator must be supplied per asset type".to_string());
    }
    let mut validator_messages = asset_definition
        .validators
        .iter()
        .flat_map(|valid| validate_validator(&valid, deps))
        .collect::<Vec<String>>();
    invalid_fields.append(&mut validator_messages);
    invalid_fields
}

fn validate_validator(validator: &ValidatorDetail, deps: &Deps<ProvenanceQuery>) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if deps.api.addr_validate(&validator.address).is_err() {
        invalid_fields.push("validator address: must be a valid address".to_string());
    }
    invalid_fields
}
