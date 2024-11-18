use cosmwasm_std::DepsMut;

/// Allows dynamic delegation of a cosmwasm [DepsMut] to prevent
/// common issues that arise when the struct is moved.
pub trait DepsManager<'a> {
    /// Functionally retrieves the result of a usage of the held [DepsMut] value.
    ///
    /// # Parameters
    ///
    /// * `deps_fn` A closure that receives the held [DepsMut].
    fn use_deps<T, F>(&self, deps_fn: F) -> T
    where
        F: FnMut(&mut DepsMut) -> T;

    /// Moves the held [DepsMut] back to the caller.
    fn into_deps(self) -> DepsMut<'a>;
}
