use cosmwasm_std::{Binary, Response};

/// Allows any implementing type to functionally move itself into a Result<T, U>
pub trait ResultExtensions
where
    Self: Sized,
{
    /// Converts the caller into an Ok (left-hand-side) result
    fn to_ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }

    /// Converts the caller into an Err (right-hand-side) result
    fn to_err<T>(self) -> Result<T, Self> {
        Err(self)
    }
}
// Implement for commonly-used structs that are out of scope of this project
impl<T> ResultExtensions for Response<T> {}
impl ResultExtensions for Binary {}
