use cosmwasm_std::{testing::MOCK_CONTRACT_ADDR, CosmosMsg, Response, SubMsg};
use provwasm_std::types::provenance::{
    msgfees::v1::MsgAssessCustomMsgFeeRequest,
    name::v1::{MsgBindNameRequest, NameRecord},
};

use crate::util::functions::{try_into_bind_name_request, try_into_custom_fee_request};

use super::test_constants::DEFAULT_CONTRACT_BASE_NAME;

// Tests that the DEFAULT_CONTRACT_BASE_NAME message was bound in a message contained in the slice
pub fn test_for_default_base_name(messages: &[SubMsg]) {
    test_message_is_name_bind_with_base_name(messages, DEFAULT_CONTRACT_BASE_NAME, true);
}

// Tests that the target asset type was bound as a name (suffixed with the default contract base name)
// in a message in the slice
pub fn test_message_is_name_bind(messages: &[SubMsg], expected_asset_type: &str) {
    test_message_is_name_bind_with_base_name(messages, expected_asset_type, false);
}

pub fn test_no_money_moved_in_response<S: Into<String>>(response: &Response, assertion_prefix: S) {
    let assertion_prefix = assertion_prefix.into();
    for message in response.messages.iter() {
        assert!(
            !matches!(message.msg, CosmosMsg::Bank(..)),
            "{}: expected no bank messages to be included, but got msg: {:?}",
            assertion_prefix,
            message.msg,
        );
        let maybe_custom_fee_request = try_into_custom_fee_request(&message.msg);
        assert!(
            maybe_custom_fee_request.is_none(),
            "{}: expected no provenance messages to be included, but got msg: {:?}",
            assertion_prefix,
            maybe_custom_fee_request,
        );
    }
}

pub fn test_aggregate_msg_fees_are_charged<S: Into<String>>(
    response: &Response,
    expected_fee_amount: u128,
    assertion_message: S,
) {
    let total_fees = response.messages.iter().fold(0u128, |agg, msg| {
        match try_into_custom_fee_request(&msg.msg) {
            Some(MsgAssessCustomMsgFeeRequest {
                amount: maybe_amount,
                ..
            }) => {
                agg + match maybe_amount {
                    Some(provwasm_std::types::cosmos::base::v1beta1::Coin { amount, .. }) => {
                        amount.parse().unwrap_or(0)
                    }
                    None => 0,
                }
            }
            None => agg,
        }
    });
    assert_eq!(
        expected_fee_amount,
        total_fees,
        "{}",
        assertion_message.into(),
    );
}

// Tests that the slice of SubMsg contains the correct name binding by iterating over all
// contained values and extracting the values within. If the is_base_name param is supplied,
// the expected_asset_type value is assumed to be the base name value.
fn test_message_is_name_bind_with_base_name(
    messages: &[SubMsg],
    expected_asset_type: &str,
    is_base_name: bool,
) {
    for message in messages {
        if let Some(MsgBindNameRequest { parent, record }) =
            try_into_bind_name_request(&message.msg)
        {
            match parent {
                Some(NameRecord {
                    name: base_name,
                    address,
                    ..
                }) => {
                    // Wrong name? Go to the next iteration
                    if is_base_name && !base_name.contains(expected_asset_type) {
                        continue;
                    } else {
                        if is_base_name {
                            assert_eq!(
                                expected_asset_type.to_string(),
                                base_name.to_string(),
                                "the default values should be used to derive the attribute base name",
                            );
                        }
                        assert_eq!(
                            MOCK_CONTRACT_ADDR,
                            address.as_str(),
                            "the default contract address should be bound to",
                        );
                    }
                }
                None => continue,
            };
            match record {
                Some(NameRecord {
                    name,
                    address,
                    restricted,
                    ..
                }) => {
                    if !is_base_name && !name.contains(expected_asset_type) {
                        continue;
                    } else {
                        if !is_base_name {
                            assert_eq!(
                                expected_asset_type.to_string(),
                                name,
                                "the default values should be used to derive the attribute name",
                            );
                        }
                        assert_eq!(
                            MOCK_CONTRACT_ADDR,
                            address.as_str(),
                            "the default contract address should be bound to",
                        );
                        assert!(
                            restricted,
                            "the restrict value should be set to true for all bound attributes"
                        );
                    }
                }
                None => continue,
            };
            // Exit early after finding the appropriate value to ensure the trailing
            // panic doesn't fire
            return;
        } else {
            panic!("unexpected message type was emitted: {:?}", &message.msg)
        }
    }
    panic!(
        "failed to find message for expected asset type `{}`",
        expected_asset_type
    );
}
