pub trait ResultExtensions where Self : Sized {
    fn to_ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }

    fn to_err<T>(self) -> Result<T, Self> {
        Err(self)
    }
}
