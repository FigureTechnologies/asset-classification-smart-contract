use crate::core::error::ContractError;
use cosmwasm_std::Response;

/// Shortens the lengthy response type for contract entrypoints.
pub type EntryPointResponse = Result<Response, ContractError>;

/// All contract pathways with exceptional code should return a result that has a contract error
/// as its resulting error type.
pub type AssetResult<T> = Result<T, ContractError>;
