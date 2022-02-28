use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Requested functionality is not yet implemented")]
    Unimplemented,
}
impl ContractError {
    /// Allows ContractError instances to be generically returned as a Response in a fluent manner
    /// instead of wrapping in an Err() call, improving readability.
    /// Ex: return ContractError::NameNotFound.to_result();
    /// vs
    ///     return Err(ContractError::NameNotFound);
    pub fn to_result<T>(self) -> Result<T, ContractError> {
        Err(self)
    }
}
