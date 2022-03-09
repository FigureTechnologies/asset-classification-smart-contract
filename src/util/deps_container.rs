use std::cell::RefCell;

use crate::util::aliases::DepsMutC;

/// Holds a mutable reference to a DepsMutC, which allows it to be passed to sub-objects
/// relatively easily and then freed when required.
pub struct DepsContainer<'a> {
    deps_cell: RefCell<&'a mut DepsMutC<'a>>,
}
impl<'a> DepsContainer<'a> {
    /// Constructs a new instance of the DepsContainer.
    /// # Example
    /// ```
    /// use provwasm_mocks::mock_dependencies;
    /// use asset_classification_smart_contract::util::deps_container::DepsContainer;
    ///
    /// let mut mock_deps = mock_dependencies(&[]);
    /// let mut deps_mut = mock_deps.as_mut();
    /// let container = DepsContainer::new(&mut deps_mut);
    /// ```
    pub fn new(deps: &'a mut DepsMutC<'a>) -> Self {
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
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::state::config_read,
        testutil::test_utilities::{test_instantiate_success, InstArgs},
    };

    use super::DepsContainer;

    #[test]
    fn test_container_usage() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let mut deps_mut = deps.as_mut();
        let container = DepsContainer::new(&mut deps_mut);
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
}
