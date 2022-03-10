use std::cell::RefCell;

use crate::util::aliases::DepsMutC;

/// Holds a mutable reference to a DepsMutC, which allows it to be passed to sub-objects
/// relatively easily and then freed when required.
pub struct DepsContainer<'a> {
    deps_cell: RefCell<DepsMutC<'a>>,
}
impl<'a> DepsContainer<'a> {
    /// Constructs a new instance of the DepsContainer.
    /// # Example
    /// ```
    /// use provwasm_mocks::mock_dependencies;
    /// use asset_classification_smart_contract::util::deps_container::DepsContainer;
    ///
    /// let mut mock_deps = mock_dependencies(&[]);
    /// let container = DepsContainer::new(mock_deps.as_mut());
    /// ```
    pub fn new(deps: DepsMutC<'a>) -> Self {
        Self {
            deps_cell: RefCell::new(deps),
        }
    }

    /// Allows the encapsulated DepsMutC value to be used while the service owns it.
    /// Note: In order to release the owned DepsMutC, simply call self.dispose()
    pub fn use_deps<T, F>(&self, mut deps_fn: F) -> T
    where
        F: FnMut(&mut DepsMutC) -> T,
    {
        deps_fn(&mut self.deps_cell.borrow_mut())
    }

    /// Relinquishes the held DepsMutC to the caller
    pub fn get(self) -> DepsMutC<'a> {
        self.deps_cell.into_inner()
    }
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::state::{config, config_read},
        testutil::test_utilities::{test_instantiate_success, InstArgs},
        util::aliases::DepsMutC,
    };

    use super::DepsContainer;

    #[test]
    fn test_container_usage() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let container = DepsContainer::new(deps.as_mut());
        let state_from_container = container.use_deps(|deps_mut| {
            config_read(deps_mut.storage)
                .load()
                .expect("expected config to load successfully")
        });
        let state_from_mut = config_read(deps.as_mut().storage)
            .load()
            .expect("self-owned deps should load state successfully");
        assert_eq!(
            state_from_container, state_from_mut,
            "states should be identical, regardless of source"
        );
    }

    #[test]
    fn test_get_deps() {
        let mut mock_deps = mock_dependencies(&[]);
        test_instantiate_success(mock_deps.as_mut(), InstArgs::default());
        let deps_mut = test_deps_container_from_different_lifetime(mock_deps.as_mut());
        config(deps_mut.storage)
            .load()
            .expect("state should load from the moved deps");
    }

    // This won't even compile if lifetimes aren't working with external references - if that happens,
    // whatever change was made that breaks this will prevent this container from being used to ferry
    // the deps into other structs
    fn test_deps_container_from_different_lifetime(deps: DepsMutC) -> DepsMutC {
        let container = DepsContainer::new(deps);
        container.get()
    }
}
