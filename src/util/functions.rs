use std::collections::HashSet;
use std::hash::Hash;

/// Determines how many elements within the provided reference slice are unique by the given
/// property.
///
/// # Examples:
/// ```
/// use asset_classification_smart_contract::util::functions::distinct_count_by_property;
///
/// let values = vec!["a", "b", "c", "a"];
/// let distinct_count = distinct_count_by_property(&values, |s| s);
/// assert_eq!(3, distinct_count);
/// ```
pub fn distinct_count_by_property<F, T, U>(slice: &[T], selector: F) -> usize
where
    U: Sized + Eq + Hash,
    F: FnMut(&T) -> &U,
{
    slice.iter().map(selector).collect::<HashSet<_>>().len()
}

/// Converts an asset type and a contract base name into an asset attribute that will be reserved
/// to the contract for writing scope attributes.
pub fn generate_asset_attribute_name<T: Into<String>, U: Into<String>>(
    asset_type: T,
    contract_base_name: U,
) -> String {
    format!("{}.{}", asset_type.into(), contract_base_name.into())
}
