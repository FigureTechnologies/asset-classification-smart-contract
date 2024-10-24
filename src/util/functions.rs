use crate::core::error::ContractError;
use crate::core::types::access_route::AccessRoute;
use crate::util::aliases::AssetResult;

use cosmwasm_std::{coin, to_json_binary, Addr, BankMsg, Binary, CosmosMsg, StdError, StdResult};
use provwasm_std::types::provenance::attribute::v1::{
    AttributeType, MsgAddAttributeRequest, MsgUpdateAttributeRequest,
};
use provwasm_std::types::provenance::msgfees::v1::MsgAssessCustomMsgFeeRequest;
use provwasm_std::types::provenance::name::v1::{MsgBindNameRequest, NameRecord};
use result_extensions::ResultExtensions;
use serde::Serialize;
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

/// Converts an asset type and scope address into a grant id for use with [Object Store Gateway](https://github.com/FigureTechnologies/object-store-gateway).
/// This combination will create a value unique to each verification's process, ensuring that each
/// selected verifier will always have access to its required scope values until any number of
/// pending verifications complete.
///
/// # Parameters
///
/// * `asset_type` The value to use at the beginning of the grant id.  Should refer to the
/// [asset_type](crate::core::types::asset_definition::AssetDefinitionV3::asset_type) property of an
/// [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3).
/// * `scope_address` The bech32 address with a prefix of "scope" that uniquely defines the scope
/// that is currently in the process of classification.
pub fn generate_os_gateway_grant_id<T: Into<String>, U: Into<String>>(
    asset_type: T,
    scope_address: U,
) -> String {
    format!("{}-{}", asset_type.into(), scope_address.into())
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
) -> CosmosMsg {
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

/// A helper to form a message for adding an attribute
/// Adapted from [provwasm-std](https://github.com/provenance-io/provwasm/blob/83ec2b8ec4339af2ee6a00e5a0318a5306f3438f/contracts/attrs/src/helpers.rs#L38-L61).
pub fn add_attribute<S: Into<String>, B: Into<Binary>>(
    address: impl Into<Addr>,
    contract_address: impl Into<Addr>,
    name: S,
    value: B,
    value_type: AttributeType,
) -> StdResult<CosmosMsg> {
    if value_type == AttributeType::Unspecified {
        return Err(StdError::generic_err(
            "cannot add attribute with unspecified value type",
        ));
    }

    let bin: Binary = value.into();
    Ok(MsgAddAttributeRequest {
        name: validate_string(name, "name")?,
        value: bin.to_vec(),
        attribute_type: value_type.into(),
        account: validate_address(address)?.to_string(),
        owner: validate_address(contract_address)?.to_string(),
        expiration_date: None,
    }
    .into())
}

/// A helper to form a message for adding a JSON attribute.
/// Adapted from [provwasm-std](https://github.com/provenance-io/provwasm/blob/83ec2b8ec4339af2ee6a00e5a0318a5306f3438f/contracts/attrs/src/helpers.rs#L63-L76).
pub fn add_json_attribute<S: Into<String>, T: Serialize + ?Sized>(
    address: impl Into<Addr>,
    contract_address: impl Into<Addr>,
    name: S,
    data: &T,
) -> StdResult<CosmosMsg> {
    add_attribute(
        address,
        contract_address,
        name,
        to_json_binary(data)?,
        AttributeType::Json,
    )
}

/// A helper to form a message for updating an existing attribute.
/// Copied from [provwasm-std](https://github.com/provenance-io/provwasm/blob/83ec2b8ec4339af2ee6a00e5a0318a5306f3438f/contracts/attrs/src/helpers.rs#L107-L137).
pub fn update_attribute<H: Into<Addr>, S: Into<String>, B: Into<Binary>>(
    address: H,
    contract_address: H,
    name: S,
    original_value: B,
    original_value_type: AttributeType,
    update_value: B,
    update_value_type: AttributeType,
) -> StdResult<CosmosMsg> {
    if original_value_type == AttributeType::Unspecified {
        return Err(StdError::generic_err(
            "cannot update attribute with unspecified original value type",
        ));
    }
    if update_value_type == AttributeType::Unspecified {
        return Err(StdError::generic_err(
            "cannot update attribute with unspecified update value type",
        ));
    }

    Ok(MsgUpdateAttributeRequest {
        original_value: original_value.into().to_vec(),
        update_value: update_value.into().to_vec(),
        original_attribute_type: original_value_type.into(),
        update_attribute_type: update_value_type.into(),
        account: validate_address(address)?.to_string(),
        owner: validate_address(contract_address)?.to_string(),
        name: validate_string(name, "name")?,
    }
    .into())
}

/// A helper that ensures string params are non-empty.
/// Copied from [provwasm-std](https://github.com/provenance-io/provwasm/blob/83ec2b8ec4339af2ee6a00e5a0318a5306f3438f/contracts/attrs/src/helpers.rs#L159-L168).
pub fn validate_string<S: Into<String>>(input: S, param_name: &str) -> StdResult<String> {
    let s: String = input.into();
    if s.trim().is_empty() {
        let errm = format!("{} must not be empty", param_name);
        Err(StdError::generic_err(errm))
    } else {
        Ok(s)
    }
}

/// A helper that ensures address params are non-empty.
/// Copied from [provwasm-std](https://github.com/provenance-io/provwasm/blob/83ec2b8ec4339af2ee6a00e5a0318a5306f3438f/contracts/attrs/src/helpers.rs#L170-L178).
pub fn validate_address<H: Into<Addr>>(input: H) -> StdResult<Addr> {
    let h: Addr = input.into();
    if h.to_string().trim().is_empty() {
        Err(StdError::generic_err("address must not be empty"))
    } else {
        Ok(h)
    }
}

/// Generates a [name bind message](MsgBindNameRequest) that will properly assign the given name value
/// to a target address.  Assumes the parent name is unrestricted or that the contract has access to
/// bind a name to the parent name.
/// Adapted from [funding-trading-bridge-smart-contract](https://github.com/FigureTechnologies/funding-trading-bridge-smart-contract/blob/a92bafb4397360ac0a4febfbc8390c7a54080e84/src/util/provenance_utils.rs#L10-L72).
///
/// # Parameters
/// * `name` The dot-qualified name to use on-chain for name binding. Ex: myname.sc.pb will generate
/// a msg that binds "myname" to the existing parent name "sc.pb".
/// * `bind_to_address` The bech32 address to which the name will be bound.
/// * `restricted` If true, the name will be bound as a restricted name, preventing future name
/// bindings from using it as a parent name.
pub fn msg_bind_name<S1: Into<String>, S2: Into<String>>(
    name: S1,
    bind_to_address: S2,
    restricted: bool,
) -> AssetResult<MsgBindNameRequest> {
    let fully_qualified_name = name.into();
    let mut name_parts = fully_qualified_name.split('.').collect::<Vec<&str>>();
    let bind_address = bind_to_address.into();
    let bind_record = if let Some(bind) = name_parts.to_owned().first() {
        if bind.is_empty() {
            return ContractError::GenericError {
                msg: format!(
                    "cannot bind to an empty name string [{}]",
                    fully_qualified_name
                ),
            }
            .to_err();
        }
        Some(NameRecord {
            name: bind.to_string(),
            address: bind_address.to_owned(),
            restricted,
        })
    } else {
        return ContractError::GenericError {
            msg: format!(
                "cannot derive bind name from input [{}]",
                fully_qualified_name
            ),
        }
        .to_err();
    };
    let parent_record = if name_parts.len() > 1 {
        // Trim the first element, because that is the new name to be bound
        name_parts.remove(0);
        let parent_name = name_parts.join(".").to_string();
        Some(NameRecord {
            name: parent_name.to_owned(),
            // The parent record must also use the address being bound to as its address in order for
            // the bind to succeed.  This is the only way in which Provenance accepts a non-restricted
            // name bind
            address: bind_address,
            restricted: false,
        })
    } else {
        None
    };
    MsgBindNameRequest {
        record: bind_record,
        parent: parent_record,
    }
    .to_ok()
}

/// Attempts to convert a [CosmosMsg] into a [MsgAddAttributeRequest]
pub fn try_into_add_attribute_request(msg: &CosmosMsg) -> Option<MsgAddAttributeRequest> {
    match &msg {
        CosmosMsg::Any(cosmwasm_std::AnyMsg { type_url: _, value }) => {
            match MsgAddAttributeRequest::try_from(value.to_owned()) {
                Ok(message) => Some(message),
                Err(_) => None,
            }
        }
        _ => None,
    }
}

/// Attempts to convert a [CosmosMsg] into a [MsgUpdateAttributeRequest]
pub fn try_into_update_attribute_request(msg: &CosmosMsg) -> Option<MsgUpdateAttributeRequest> {
    match &msg {
        CosmosMsg::Any(cosmwasm_std::AnyMsg { type_url: _, value }) => {
            match MsgUpdateAttributeRequest::try_from(value.to_owned()) {
                Ok(message) => Some(message),
                Err(_) => None,
            }
        }
        _ => None,
    }
}

/// Attempts to convert a [CosmosMsg] into a [MsgAssessCustomMsgFeeRequest]
pub fn try_into_custom_fee_request(msg: &CosmosMsg) -> Option<MsgAssessCustomMsgFeeRequest> {
    match &msg {
        CosmosMsg::Any(cosmwasm_std::AnyMsg { type_url: _, value }) => {
            match MsgAssessCustomMsgFeeRequest::try_from(value.to_owned()) {
                Ok(message) => Some(message),
                Err(_) => None,
            }
        }
        _ => None,
    }
}

/// Attempts to convert a [CosmosMsg] into a [MsgBindNameRequest]
pub fn try_into_bind_name_request(msg: &CosmosMsg) -> Option<MsgBindNameRequest> {
    match &msg {
        CosmosMsg::Any(cosmwasm_std::AnyMsg { type_url: _, value }) => {
            match MsgBindNameRequest::try_from(value.to_owned()) {
                Ok(message) => Some(message),
                Err(_) => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{error::ContractError, types::access_route::AccessRoute};
    use crate::testutil::test_utilities::assert_single_item;
    use crate::util::functions::{
        filter_valid_access_routes, generate_os_gateway_grant_id, msg_bind_name,
        replace_single_matching_vec_element,
    };
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

    #[test]
    fn test_generate_os_gateway_grant_id() {
        assert_eq!(
            "heloc-scopescopescope",
            generate_os_gateway_grant_id("heloc", "scopescopescope"),
            "the output value should equate to the asset type concatenated to the scope address with a hyphen",
        );
    }

    /// Copied from [funding-trading-bridge-smart-contract](https://github.com/FigureTechnologies/funding-trading-bridge-smart-contract/blob/a92bafb4397360ac0a4febfbc8390c7a54080e84/src/util/provenance_utils.rs#L237-L269).
    #[test]
    fn msg_bind_name_creates_proper_binding_with_fully_qualified_name() {
        let name = "test.name.bro";
        let address = "some-address";
        let msg =
            msg_bind_name(name, address, true).expect("valid input should not yield an error");
        let parent = msg.parent.expect("the result should include a parent msg");
        assert_eq!(
            "name.bro", parent.name,
            "parent name should be properly derived",
        );
        assert_eq!(
            address, parent.address,
            "parent address value should be set as the bind address because that's what enables binds to unrestricted parent addresses",
        );
        assert!(
            !parent.restricted,
            "parent restricted should always be false",
        );
        let bind = msg.record.expect("the result should include a name record");
        assert_eq!(
            "test", bind.name,
            "the bound name should be properly derived",
        );
        assert_eq!(
            address, bind.address,
            "the bound name should have the specified address",
        );
        assert!(
            bind.restricted,
            "the restricted value should equate to the value specified",
        );
    }

    /// Adapted from [funding-trading-bridge-smart-contract](https://github.com/FigureTechnologies/funding-trading-bridge-smart-contract/blob/a92bafb4397360ac0a4febfbc8390c7a54080e84/src/util/provenance_utils.rs#L271-L294).
    #[test]
    fn msg_bind_name_creates_proper_binding_with_single_node_name() {
        let name = "name";
        let address = "address";
        let msg = msg_bind_name(name, address, false)
            .expect("proper input should produce a success result");
        assert!(
            msg.parent.is_none(),
            "the parent record should not be set because the name bind does not require it",
        );
        let bind = msg.record.expect("the result should include a name record");
        assert_eq!(
            "name", bind.name,
            "the bound name should be properly derived",
        );
        assert_eq!(
            address, bind.address,
            "the bound name should have the specified address",
        );
        assert!(
            !bind.restricted,
            "the restricted value should equate to the value specified",
        );
    }

    /// Adapted from [funding-trading-bridge-smart-contract](https://github.com/FigureTechnologies/funding-trading-bridge-smart-contract/blob/a92bafb4397360ac0a4febfbc8390c7a54080e84/src/util/provenance_utils.rs#L296-L320).
    #[test]
    fn msg_bind_name_should_properly_guard_against_bad_input() {
        let _expected_error_message = "cannot derive bind name from input []".to_string();
        assert!(
            matches!(
                msg_bind_name("", "address", true)
                    .expect_err("an error should occur when no name is specified"),
                ContractError::GenericError {
                    msg: _expected_error_message,
                },
            ),
            "unexpected error message when specifying an empty name",
        );
        let _expected_error_message = "cannot bind to an empty name string [.suffix]".to_string();
        assert!(
            matches!(
                msg_bind_name(".suffix", "address", true)
                    .expect_err("an error should occur when specifying a malformed name"),
                ContractError::GenericError {
                    msg: _expected_error_message,
                },
            ),
            "unexpected error message when specifying a malformed name",
        );
    }
}
