use crate::util::traits::ResultExtensions;
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    InvalidFunds(String),

    #[error("Message of type [{message_type}] was invalid. Invalid fields: {invalid_fields:?}")]
    InvalidMessageFields {
        message_type: String,
        invalid_fields: Vec<String>,
    },

    #[error("Invalid message type provided. Expected message type {expected_message_type}")]
    InvalidMessageType { expected_message_type: String },

    #[error("Unsupported asset type [{asset_type}]")]
    UnsupportedAssetType { asset_type: String },

    #[error("Unsupported validator [{validator_address}] for asset type [{asset_type}]")]
    UnsupportedValidator {
        validator_address: String,
        asset_type: String,
    },

    #[error("Asset {asset_uuid} already onboarded")]
    AssetAlreadyOnboarded { asset_uuid: String },

    #[error("Asset {asset_uuid} not found")]
    AssetNotFound { asset_uuid: String },

    #[error("Error onboarding asset (type: {asset_type}, uuid: {asset_uuid}): {message}")]
    AssetOnboardingError {
        asset_type: String,
        asset_uuid: String,
        message: String,
    },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Requested functionality is not yet implemented")]
    Unimplemented,
}
impl ResultExtensions for ContractError {}
