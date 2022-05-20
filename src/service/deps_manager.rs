use crate::util::aliases::DepsMutC;

/// Allows dynamic delegation of a cosmwasm [DepsMutC](crate::util::aliases::DepsMutC) to prevent
/// common issues that arise when the struct is moved.
pub trait DepsManager<'a> {
    /// Functionally retrieves the result of a usage of the held [DepsMutC](crate::util::aliases::DepsMutC)
    /// value.
    ///
    /// # Parameters
    ///
    /// * `deps_fn` A closure that receives the held [DepsMutC](crate::util::aliases::DepsMutC).
    fn use_deps<T, F>(&self, deps_fn: F) -> T
    where
        F: FnMut(&mut DepsMutC) -> T;

    /// Moves the held [DepsMutC](crate::util::aliases::DepsMutC) back to the caller.
    fn into_deps(self) -> DepsMutC<'a>;
}
