/// Allows any Sized type to functionally move itself into a Result<T, U>
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

/// Allows any Sized type to functionally move itself into an Option<T>
pub trait OptionExtensions
where
    Self: Sized,
{
    fn to_option(self) -> Option<Self> {
        Some(self)
    }
}
// Implement for EVERYTHING IN THE UNIVERSE
impl<T> OptionExtensions for T {}

#[cfg(test)]
mod tests {
    use crate::core::error::ContractError;

    use super::{OptionExtensions, ResultExtensions};

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
        let result: Result<(), ContractError> =
            ContractError::InvalidFunds("no u".to_string()).to_err();
        let error = result.unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "the error should unwrap correctly, but got incorrect error: {:?}",
            error,
        );
    }

    #[test]
    fn test_to_option() {
        let option: Option<String> = "hello".to_string().to_option();
        assert_eq!(
            "hello",
            option
                .expect("option should unwrap because it was initialized with a value")
                .as_str(),
            "incorrect value contained in wrapped Option",
        );
    }
}
