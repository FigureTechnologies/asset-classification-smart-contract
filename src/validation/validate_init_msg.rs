use crate::core::error::ContractError;
use crate::core::msg::InitMsg;
use crate::core::state::{AssetDefinition, FeeDestination, ValidatorDetail};
use crate::util::aliases::DepsC;
use crate::util::functions::distinct_count_by_property;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::Decimal;

pub fn validate_init_msg(msg: &InitMsg, deps: &DepsC) -> Result<(), ContractError> {
    let mut invalid_fields: Vec<String> = vec![];
    if msg.base_contract_name.is_empty() {
        invalid_fields.push("base_contract_name: must not be blank".to_string());
    }
    if distinct_count_by_property(&msg.asset_definitions, |def| &def.asset_type)
        != msg.asset_definitions.len()
    {
        invalid_fields.push(
            "asset_definitions: each definition must specify a unique asset type".to_string(),
        );
    }
    let mut asset_messages = msg
        .asset_definitions
        .iter()
        .flat_map(|asset| validate_asset_definition(asset, deps))
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

fn validate_asset_definition(asset_definition: &AssetDefinition, deps: &DepsC) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if asset_definition.asset_type.is_empty() {
        invalid_fields.push("asset_definition:asset_type: must not be blank".to_string());
    }
    if asset_definition.validators.is_empty() {
        invalid_fields.push(
            "asset_definition:validators: at least one validator must be supplied per asset type"
                .to_string(),
        );
    }
    let mut validator_messages = asset_definition
        .validators
        .iter()
        .flat_map(|valid| validate_validator(valid, deps))
        .collect::<Vec<String>>();
    invalid_fields.append(&mut validator_messages);
    invalid_fields
}

fn validate_validator(validator: &ValidatorDetail, deps: &DepsC) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if deps.api.addr_validate(&validator.address).is_err() {
        invalid_fields.push("validator:address: must be a valid address".to_string());
    }
    if validator.fee_percent > Decimal::percent(100) {
        invalid_fields
            .push("validator:fee_percent: must be less than or equal to 100%".to_string());
    }
    if validator.fee_destinations.is_empty() && validator.fee_percent != Decimal::zero() {
        invalid_fields.push(
            "validator:fee_percent: Cannot specify a fee percent if fee destinations are supplied"
                .to_string(),
        );
    }
    if !validator.fee_destinations.is_empty() && validator.fee_percent > Decimal::zero() {
        invalid_fields.push("validator:fee_destinations: at least one fee destination must be provided when the fee percent is greater than zero".to_string());
    }
    if !validator.fee_destinations.is_empty()
        && validator
            .fee_destinations
            .iter()
            .map(|d| d.fee_percent)
            .sum::<Decimal>()
            != Decimal::percent(100)
    {
        invalid_fields.push("validator:fee_destinations: Fee destinations' fee_percents must always sum to a 100% distribution".to_string());
    }
    let mut fee_destination_messages = validator
        .fee_destinations
        .iter()
        .flat_map(|destination| validate_destination(destination, deps))
        .collect::<Vec<String>>();
    invalid_fields.append(&mut fee_destination_messages);
    invalid_fields
}

fn validate_destination(destination: &FeeDestination, deps: &DepsC) -> Vec<String> {
    let mut invalid_fields: Vec<String> = vec![];
    if deps.api.addr_validate(&destination.address).is_err() {
        invalid_fields.push("fee_destination:address: must be a valid address".to_string());
    }
    if destination.fee_percent > Decimal::percent(100) {
        invalid_fields
            .push("fee_destination:fee_percent: must be less than or equal to 100%".to_string());
    }
    invalid_fields
}
