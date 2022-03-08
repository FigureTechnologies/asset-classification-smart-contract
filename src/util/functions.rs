use crate::core::error::ContractError;
use crate::util::aliases::ContractResult;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{coin, Addr, BankMsg, CosmosMsg, Decimal, Uint128};
use provwasm_std::ProvenanceMsg;
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
/// let attribute_name = generate_asset_attribute_name(asset_type, contract_base_name);
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

/// Takes an existing vector, moves it into this function, swaps out a single existing item for
/// a specified replacement item.  If less or more than one existing item matches the given
/// predicate closure, an error is returned.
pub fn replace_single_matching_vec_element<T, F>(
    v: Vec<T>,
    new: T,
    predicate: F,
) -> ContractResult<Vec<T>>
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
        ContractError::std_err(format!(
            "expected a single value to be replaced, but found {}",
            total_values_replaced
        ))
        .to_err()
    }
}

/// Creates a message that sends funds of the specified denomination from the contract to the recipient address.
/// Important: The response type is of ProvenanceMsg, which allows this bank send message to match the type
/// used for contract execution routes.
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

/// A helper that ensures address params are non-empty. Taken from non-exposed provwasm codebase.
/// Allows address validation without having to pass in deps.
/// TODO: Make this check to see if the address is valid Bech32
pub fn validate_address<A: Into<String>>(input: A) -> ContractResult<Addr> {
    let address_string: String = input.into();
    if address_string.trim().is_empty() {
        ContractError::std_err("address must not be empty").to_err()
    } else {
        Ok(Addr::unchecked(address_string))
    }
}

#[cfg(test)]
mod tests {
    use crate::core::error::ContractError;
    use crate::util::functions::replace_single_matching_vec_element;
    use cosmwasm_std::{BankMsg, CosmosMsg, StdError};

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
            ContractError::Std(e) => match e {
                StdError::GenericErr { msg, .. } => {
                    assert_eq!(
                        "expected a single value to be replaced, but found 0", msg,
                        "the StdError message was not the expected result for no values replaced",
                    );
                }
                _ => panic!("unexpected StdError variant found"),
            },
            _ => panic!("unexpected error type encountered"),
        };
    }

    #[test]
    fn test_replace_matching_vec_elements_failure_for_multiple_matches() {
        let source = vec![TestVal(1), TestVal(2)];
        let error =
            replace_single_matching_vec_element(source, TestVal(10), |v| v.0 > 0).unwrap_err();
        match error {
            ContractError::Std(e) => match e {
                StdError::GenericErr { msg, .. } => {
                    assert_eq!(
                        "expected a single value to be replaced, but found 2", msg,
                        "the StdError message was not the expected result for many values replaced",
                    )
                }
                _ => panic!("unexpected StdError variant found"),
            },
            _ => panic!("unexpected error type encountered"),
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
                _ => panic!("unexpected bank msg generated by helper function"),
            },
            _ => panic!("unexpected message generated by helper function"),
        }
    }
}
