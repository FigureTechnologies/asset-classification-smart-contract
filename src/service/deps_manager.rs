use crate::util::aliases::DepsMutC;

pub trait DepsManager<'a> {
    fn use_deps<T, F>(&self, deps_fn: F) -> T
    where
        F: FnMut(&mut DepsMutC) -> T;

    fn into_deps(self) -> DepsMutC<'a>;
}
