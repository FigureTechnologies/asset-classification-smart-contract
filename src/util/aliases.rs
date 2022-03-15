use crate::core::error::ContractError;
use cosmwasm_std::{Deps, DepsMut, Response};
use provwasm_std::{ProvenanceMsg, ProvenanceQuery};

/// Shortens the lengthy response type for contract entrypoints
pub type EntryPointResponse = Result<Response<ProvenanceMsg>, ContractError>;

/// All contract pathways with exceptional code should return a result that has a contract error
/// as its resulting error type.
pub type AssetResult<T> = Result<T, ContractError>;

/// Type alias to shorten the lengthy DepsMut<'a, T> declaration. Short for Dependencies Mutable Contract.
pub type DepsMutC<'a> = DepsMut<'a, ProvenanceQuery>;

/// Type alias to shorten the lengthy Deps<'a, T> declaration. Short for Dependencies Contract.
pub type DepsC<'a> = Deps<'a, ProvenanceQuery>;
