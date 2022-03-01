use cosmwasm_std::{Decimal, Uint128};
use std::collections::HashSet;
use std::hash::Hash;
use std::ops::Mul;

/// Determines how many elements within the provided reference slice are unique by the given
/// property.
///
/// # Examples
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
///
/// # Examples
/// ```
/// use asset_classification_smart_contract::util::functions::generate_asset_attribute_name;
///
/// let asset_type = "mortgage";
/// let contract_base_name = "asset";
/// let attribute_name = generate_asset_attribute_name(&asset_type, &contract_base_name);
/// assert_eq!("mortgage.asset", attribute_name.as_str());
/// ```
pub fn generate_asset_attribute_name<T: Into<String>, U: Into<String>>(
    asset_type: T,
    contract_base_name: U,
) -> String {
    format!("{}.{}", asset_type.into(), contract_base_name.into())
}

/// Converts a decimal to a display string, like "1%".
///
/// # Examples
/// ```
/// use cosmwasm_std::Decimal;
/// use asset_classification_smart_contract::util::functions::decimal_display_string;
///
/// let decimal = Decimal::percent(25);
/// let display_string = decimal_display_string(&decimal);
/// assert_eq!("25%", display_string.as_str());
/// ```
pub fn decimal_display_string(decimal: &Decimal) -> String {
    format!("{}%", Uint128::new(100).mul(*decimal))
}
