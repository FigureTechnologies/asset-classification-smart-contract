use crate::util::traits::ResultExtensions;
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Bech32Error(#[from] bech32::Error),

    #[error("duplicate/existing asset definition provided as input")]
    DuplicateAssetDefinitionProvided,

    #[error("duplicate/existing validator address provided as input")]
    DuplicateValidatorProvided,

    #[error("{0}")]
    InvalidFunds(String),

    #[error("Message of type [{message_type}] was invalid. Invalid fields: {invalid_fields:?}")]
    InvalidMessageFields {
        message_type: String,
        invalid_fields: Vec<String>,
    },

    #[error("Invalid message type provided. Expected message type {expected_message_type}")]
    InvalidMessageType { expected_message_type: String },

    #[error("Resource not found: {explanation}")]
    NotFound { explanation: String },

    #[error("Unsupported asset type [{asset_type}]")]
    UnsupportedAssetType { asset_type: String },

    #[error("Unsupported validator [{validator_address}] for asset type [{asset_type}]")]
    UnsupportedValidator {
        validator_address: String,
        asset_type: String,
    },

    #[error("Unauthorized: {explanation}")]
    Unauthorized { explanation: String },

    #[error("Requested functionality is not yet implemented")]
    Unimplemented,

    #[error("{0}")]
    UuidError(#[from] uuid::Error),
}
impl ResultExtensions for ContractError {}
impl ContractError {
    pub fn std_err<S: Into<String>>(msg: S) -> ContractError {
        ContractError::Std(StdError::generic_err(msg))
    }
}
