use crate::util::traits::ResultExtensions;
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Bech32Error(#[from] bech32::Error),

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

    #[error("Asset {scope_address} already onboarded")]
    AssetAlreadyOnboarded { scope_address: String },

    #[error("Asset {scope_address} not found")]
    AssetNotFound { scope_address: String },

    #[error("Error onboarding asset (type: {asset_type}, address: {scope_address}): {message}")]
    AssetOnboardingError {
        asset_type: String,
        scope_address: String,
        message: String,
    },

    #[error("Asset identifier not supplied, please provide either asset_uuid or scope_address")]
    AssetIdentifierNotSupplied,

    #[error("Asset identifier mismatch, both asset_uuid and scope_address provided, but provided scope_address [{scope_address}] cannot be derived from asset_uuid [{asset_uuid}]")]
    AssetIdentifierMismatch {
        asset_uuid: String,
        scope_address: String,
    },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Requested functionality is not yet implemented")]
    Unimplemented,

    #[error("{0}")]
    UuidError(#[from] uuid::Error),
}
impl ResultExtensions for ContractError {}
