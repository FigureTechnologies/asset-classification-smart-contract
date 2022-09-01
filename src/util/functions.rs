use crate::core::error::ContractError;
use crate::core::types::access_route::AccessRoute;
use crate::util::aliases::AssetResult;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{coin, BankMsg, CosmosMsg};
use provwasm_std::ProvenanceMsg;
use std::collections::HashSet;
use std::hash::Hash;

/// Determines how many elements within the provided reference slice are unique by the given
/// property.
///
/// # Parameters
///
/// * `slice` A reference slice from which to derive values to count.
/// * `selector` A closure that defines the criteria used to determine when a value in the slice
/// should be added to the count.
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
/// # Parameters
///
/// * `asset_type` The value to use at the beginning of the name qualifier.  Should refer to the
/// [asset_type](crate::core::types::asset_definition::AssetDefinitionV3::asset_type) property of an
/// [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3).
/// * `base_contract_name` The value to use at the end of the name qualifier, after the dot.  Should
/// refer to the [base_contract_name](crate::core::state::StateV2::base_contract_name) of the
/// contract's [StateV2](crate::core::state::StateV2) internally-stored value.
///
/// # Examples
/// ```
/// use asset_classification_smart_contract::util::functions::generate_asset_attribute_name;
///
/// let asset_type = "mortgage";
/// let base_contract_name = "asset";
/// let attribute_name = generate_asset_attribute_name(asset_type, base_contract_name);
/// assert_eq!("mortgage.asset", attribute_name.as_str());
/// ```
pub fn generate_asset_attribute_name<T: Into<String>, U: Into<String>>(
    asset_type: T,
    base_contract_name: U,
) -> String {
    format!("{}.{}", asset_type.into(), base_contract_name.into())
}

/// Takes an existing vector, moves it into this function, swaps out a single existing item for
/// a specified replacement item.  If less or more than one existing item matches the given
/// predicate closure, an error is returned.
///
/// # Parameters
///
/// * `v` The vector to move and replace an item within.
/// * `new` The instance of the new item to use in place of an existing item.
/// * `predicate` A closure that defines how to locate the single item to remove from the vector
/// and replace with the new item.
pub fn replace_single_matching_vec_element<T, F>(
    v: Vec<T>,
    new: T,
    predicate: F,
) -> AssetResult<Vec<T>>
where
    F: Fn(&T) -> bool,
{
    let initial_size = v.len();
    let mut resulting_values = v
        .into_iter()
        // Retain all values that do NOT match the predicate
        .filter(|elem| !predicate(elem))
        .collect::<Vec<T>>();
    let total_values_replaced = initial_size - resulting_values.len();
    if total_values_replaced == 1 {
        resulting_values.push(new);
        Ok(resulting_values)
    } else {
        ContractError::generic(format!(
            "expected a single value to be replaced, but found {}",
            total_values_replaced
        ))
        .to_err()
    }
}

/// Creates a message that sends funds of the specified denomination from the contract to the recipient address.
/// Important: The response type is of ProvenanceMsg, which allows this bank send message to match the type
/// used for contract execution routes.
///
/// # Parameters
///
/// * `recipient` The bech32 address of the receiver of the sent funds.
/// * `amount` An amount of coin to send (from the contract's internal funding amount).
/// * `denom` The denomination of coin to send.
pub fn bank_send<R: Into<String>, D: Into<String>>(
    recipient: R,
    amount: u128,
    denom: D,
) -> CosmosMsg<ProvenanceMsg> {
    CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient.into(),
        amount: vec![coin(amount, denom)],
    })
}

/// Trims down a vector of AccessRoute to ensure that the contained values are valid and unique.
/// Does the following:
/// Ensures that access routes have a non-empty route property.
/// Ensures that access routes either have an unset name, or a non-blank, set name.
/// Ensures that all access routes, after being trimmed of trailing whitespace, are unique. Drops duplicates.
///
/// # Parameters
///
/// * `routes` The vector of routes to filter. Moves into this function and is replaced by a new
/// vector containing only valid routes.
pub fn filter_valid_access_routes(routes: Vec<AccessRoute>) -> Vec<AccessRoute> {
    routes
        .into_iter()
        // Ensure all whitespace values are removed from both routes and names to ensure duplicate detection works as intended
        .map(|r| r.trim_values())
        // Drop all proposed entries that contain empty routes or poorly-defined names. These definitions are not useful to downstream consumers
        .filter(|r| {
            !r.route.is_empty()
                && match &r.name {
                    Some(name) => !name.is_empty(),
                    None => true,
                }
        })
        // Temp swap to a HashSet to filter duplicates automagically
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<AccessRoute>>()
}

#[cfg(test)]
mod tests {
    use crate::core::{error::ContractError, types::access_route::AccessRoute};
    use crate::testutil::test_utilities::assert_single_item;
    use crate::util::functions::{filter_valid_access_routes, replace_single_matching_vec_element};
    use cosmwasm_std::{BankMsg, CosmosMsg};

    use super::bank_send;

    #[derive(Debug, PartialEq)]
    struct TestVal(u32);

    #[test]
    fn test_replace_matching_vec_elements_success() {
        let source = vec![TestVal(1), TestVal(2), TestVal(3), TestVal(4), TestVal(5)];
        let result_vec = replace_single_matching_vec_element(source, TestVal(6), |v| v.0 == 5)
            .expect("the replacement should work correctly");
        let expected_result = vec![TestVal(1), TestVal(2), TestVal(3), TestVal(4), TestVal(6)];
        assert_eq!(
            expected_result, result_vec,
            "expected a single result to be replaced correctly",
        );
    }

    #[test]
    fn test_replace_matching_vec_elements_failure_for_no_matches() {
        let source = vec![TestVal(10), TestVal(20)];
        let error =
            replace_single_matching_vec_element(source, TestVal(99), |v| v.0 == 100).unwrap_err();
        match error {
            ContractError::GenericError { msg } => {
                assert_eq!(
                    "expected a single value to be replaced, but found 0", msg,
                    "the StdError message was not the expected result for no values replaced",
                );
            }
            _ => panic!("unexpected error type encountered: {:?}", error),
        };
    }

    #[test]
    fn test_replace_matching_vec_elements_failure_for_multiple_matches() {
        let source = vec![TestVal(1), TestVal(2)];
        let error =
            replace_single_matching_vec_element(source, TestVal(10), |v| v.0 > 0).unwrap_err();
        match error {
            ContractError::GenericError { msg } => {
                assert_eq!(
                    "expected a single value to be replaced, but found 2", msg,
                    "the StdError message was not the expected result for many values replaced",
                );
            }
            _ => panic!("unexpected error type encountered: {:?}", error),
        };
    }

    #[test]
    fn test_bank_send() {
        let msg = bank_send("address", 150, "fakecoin");
        match msg {
            CosmosMsg::Bank(bank_msg) => match bank_msg {
                BankMsg::Send { to_address, amount } => {
                    assert_eq!(
                        "address",
                        to_address.as_str(),
                        "expected the address to be output correctly",
                    );
                    assert_eq!(
                        1,
                        amount.len(),
                        "expected only one coin to be added to the message",
                    );
                    let coin = amount.first().unwrap();
                    assert_eq!(
                        150,
                        coin.amount.u128(),
                        "expected the coin to contain the proper amount",
                    );
                    assert_eq!(
                        "fakecoin",
                        coin.denom.as_str(),
                        "expected the coin to contain the proper denom",
                    );
                }
                _ => panic!(
                    "unexpected bank msg generated by helper function: {:?}",
                    bank_msg
                ),
            },
            _ => panic!("unexpected message generated by helper function: {:?}", msg),
        }
    }

    #[test]
    fn test_valid_access_routes_drops_none_name_blank_route() {
        let routes = vec![AccessRoute::route_only("   ")];
        assert!(
            filter_valid_access_routes(routes).is_empty(),
            "input with a single, invalid route should produce no output",
        );
    }

    #[test]
    fn test_valid_route_but_blank_name_is_dropped() {
        let routes = vec![AccessRoute::route_and_name("route", "")];
        assert!(
            filter_valid_access_routes(routes).is_empty(),
            "input with an empty name should be dropped",
        );
    }

    #[test]
    fn test_invalid_route_and_set_name_is_dropped() {
        let routes = vec![AccessRoute::route_and_name("   ", "goodname")];
        assert!(
            filter_valid_access_routes(routes).is_empty(),
            "input with an invalid route but good name should be dropped",
        );
    }

    #[test]
    fn test_valid_route_with_no_name_is_kept() {
        let routes = vec![AccessRoute::route_only("goodroute")];
        assert_eq!(
            routes,
            filter_valid_access_routes(routes.clone()),
            "route output should be identical because no values were dropped",
        );
    }

    #[test]
    fn test_valid_route_with_set_name_is_kept() {
        let routes = vec![AccessRoute::route_and_name("goodroute", "goodname")];
        assert_eq!(
            routes,
            filter_valid_access_routes(routes.clone()),
            "route output should be identical because no values were dropped",
        );
    }

    #[test]
    fn test_direct_duplicates_are_dropped() {
        let routes = vec![
            AccessRoute::route_and_name("route", "name"),
            AccessRoute::route_and_name("route", "name"),
        ];
        let result = filter_valid_access_routes(routes.clone());
        let filtered_route = assert_single_item(
            &result,
            "only one route should be in the output because duplicates were filtered",
        );
        assert_eq!(
            routes.first().unwrap().to_owned(),
            filtered_route,
            "the first of two duplicates should equate to the resulting value"
        );
    }

    #[test]
    fn test_trimmed_duplicates_are_dropped() {
        let routes = vec![
            AccessRoute::route_and_name("  route", "  name"),
            AccessRoute::route_and_name("route  ", "name  "),
        ];
        let result = filter_valid_access_routes(routes);
        let filtered_route = assert_single_item(
            &result,
            "only one route should be in the output because the routes were duplicates after filtration",
        );
        assert_eq!(
            "route", filtered_route.route,
            "the resulting route value should be trimmed down from the original",
        );
        assert_eq!(
            "name",
            filtered_route
                .name
                .expect("the resulting route should have a name value"),
            "the resulting name value should be trimmed down from the original",
        );
    }

    #[test]
    fn test_duplicate_routes_but_different_names_are_kept() {
        let routes = vec![
            AccessRoute::route_and_name("route", "name1"),
            AccessRoute::route_and_name("route", "name2"),
        ];
        let result = filter_valid_access_routes(routes);
        assert_eq!(
            2,
            result.len(),
            "both routes should be kept because they have different names",
        );
    }

    #[test]
    fn test_duplicate_routes_with_set_and_unset_names_are_kept() {
        let routes = vec![
            AccessRoute::route_and_name("route", "name"),
            AccessRoute::route_only("route"),
        ];
        let result = filter_valid_access_routes(routes);
        assert_eq!(
            2,
            result.len(),
            "both routes should be kept because one has a name and the other does not",
        );
    }
}
