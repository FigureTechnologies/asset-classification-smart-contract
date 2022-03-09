use std::{cell::RefCell, ops::Deref};

/// Contains a RefCell that manages a Vec of Clone-implementing values.
/// Allows for the internal values to be mutated without declaring
/// the value as mutable.
pub struct VecContainer<T> {
    pub values: RefCell<Vec<T>>,
}
impl<T> VecContainer<T> {
    /// Construct a new instance of a container, starting with an empty vector
    pub fn new() -> Self {
        Self {
            values: RefCell::new(vec![]),
        }
    }

    /// Pushes a single owned item to the contained Vec
    pub fn push(&self, msg: T) {
        self.values.borrow_mut().push(msg);
    }

    /// Appends an owned, mutable instance of a Vec containing instances of T to the contained Vec
    pub fn append(&self, msgs: &mut Vec<T>) {
        self.values.borrow_mut().append(msgs)
    }

    /// Fetches the actual value inside the RefCell, moving the internalized value
    /// and disposing of this container in the process
    pub fn get(self) -> Vec<T> {
        self.values.into_inner()
    }
}
impl<T: Clone> VecContainer<T> {
    /// Fetches a cloned set of the owned values. Useful for early fetches without disposing of the container
    pub fn get_cloned(&self) -> Vec<T> {
        self.values.borrow().deref().to_owned().to_vec()
    }
}
impl<T: Copy> VecContainer<T> {
    /// Fetches a copied set of the owned values.  Copying is more efficient than cloning, so this function should
    /// be preferred over get_cloned when an object contained in the VecContainer implements both
    pub fn get_copied(&self) -> Vec<T> {
        self.values.borrow().deref().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::VecContainer;

    #[test]
    fn test_container_push() {
        let container = VecContainer::new();
        container.push("hello world or whatever".to_string());
        let strings = container.get();
        assert_eq!(
            1,
            strings.len(),
            "only one string should be contained in the container"
        );
    }

    #[test]
    fn test_container_append() {
        let container = VecContainer::new();
        container.append(&mut vec![1, 2, 3]);
        let ints = container.get();
        assert_eq!(
            3,
            ints.len(),
            "three ints should be contained in the container",
        );
        assert_eq!(1, ints[0], "the first value should be 1");
        assert_eq!(2, ints[1], "the second value should be 2");
        assert_eq!(3, ints[2], "the third value should be 3");
    }

    #[test]
    fn test_container_get_cloned() {
        let container = VecContainer::new();
        container.push("first_string".to_string());
        let first_vec = container.get_cloned();
        // Proves that the value is not moved after the first get_cloned()
        let second_vec = container.get_cloned();
        assert_eq!(1, first_vec.len(), "the vector should only have one value");
        assert_eq!(
            "first_string",
            first_vec.first().unwrap().as_str(),
            "the value should be identical to the input pushed to the container"
        );
        assert_eq!(
            first_vec, second_vec,
            "the cloned vectors should be identical"
        );
    }

    #[test]
    fn test_container_get_copied() {
        let container = VecContainer::new();
        container.push("first_string");
        let first_vec = container.get_copied();
        // Proves that the value is not moved after the first get_copied()
        let second_vec = container.get_copied();
        assert_eq!(1, first_vec.len(), "the vector should only have one value");
        assert_eq!(
            "first_string",
            first_vec.first().unwrap().to_owned(),
            "the value should be identical to the input pushed to the container",
        );
        assert_eq!(
            first_vec, second_vec,
            "the copied vectors should be identical",
        );
    }
}
