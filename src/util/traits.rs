/// Allows any Sized type to functionally move itself into an Option<T>
pub trait OptionExtensions
where
    Self: Sized,
{
    fn to_some(self) -> Option<Self> {
        Some(self)
    }
}
// Implement for EVERYTHING IN THE UNIVERSE
impl<T> OptionExtensions for T {}

#[cfg(test)]
mod tests {
    use super::OptionExtensions;

    #[test]
    fn test_to_option() {
        let option: Option<String> = "hello".to_string().to_some();
        assert_eq!(
            "hello",
            option
                .expect("option should unwrap because it was initialized with a value")
                .as_str(),
            "incorrect value contained in wrapped Option",
        );
    }
}
