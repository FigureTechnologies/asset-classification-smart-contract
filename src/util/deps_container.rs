use std::cell::RefCell;

use crate::util::aliases::DepsMutC;

/// Holds a ref cell to a DepsMutC, which allows it to be passed to sub-objects
/// relatively easily and then freed when required.
pub struct DepsContainer<'a> {
    /// A ref cell used to control access to the held deps mut without causing it to be moved through
    /// various actions.
    deps_cell: RefCell<DepsMutC<'a>>,
}
impl<'a> DepsContainer<'a> {
    /// Constructs a new instance of the DepsContainer.
    ///
    /// # Parameters
    ///
    /// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
    /// resources like contract internal storage and a querier to retrieve blockchain objects.
    ///
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
    /// Note: In order to release the owned DepsMutC, simply call `self.dispose()`.
    ///
    /// # Parameters
    ///
    /// * `deps_fn` A closure that utilizes the internally-held [DepsMutC](super::aliases::DepsMutC) reference
    pub fn use_deps<T, F>(&self, mut deps_fn: F) -> T
    where
        F: FnMut(&mut DepsMutC) -> T,
    {
        deps_fn(&mut self.deps_cell.borrow_mut())
    }

    /// Relinquishes the held DepsMutC to the caller with a move.
    pub fn get(self) -> DepsMutC<'a> {
        self.deps_cell.into_inner()
    }
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::state::{config_read_v2, config_v2},
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
            config_read_v2(deps_mut.storage)
                .load()
                .expect("expected config to load successfully")
        });
        let state_from_mut = config_read_v2(deps.as_mut().storage)
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
        config_v2(deps_mut.storage)
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
