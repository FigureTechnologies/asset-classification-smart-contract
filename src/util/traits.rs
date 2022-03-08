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
// Implement for EVERYTHING IN THE UNIVERSE
impl<T> ResultExtensions for T {}

#[cfg(test)]
mod tests {
    use crate::core::error::ContractError;

    use super::ResultExtensions;

    #[test]
    fn test_to_ok() {
        let value: Result<String, ContractError> = "hello".to_string().to_ok();
        assert_eq!(
            "hello".to_string(),
            value.unwrap(),
            "expected the value to serialize correctly",
        );
    }

    #[test]
    fn test_to_err() {
        let error: Result<(), ContractError> =
            ContractError::InvalidFunds("no u".to_string()).to_err();
        assert!(
            matches!(error.unwrap_err(), ContractError::InvalidFunds(_)),
            "the error should unwrap correctly",
        );
    }
}
