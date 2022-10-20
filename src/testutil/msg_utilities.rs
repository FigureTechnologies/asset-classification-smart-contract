use cosmwasm_std::{testing::MOCK_CONTRACT_ADDR, CosmosMsg, Response, SubMsg};
use provwasm_std::{MsgFeesMsgParams, NameMsgParams, ProvenanceMsg, ProvenanceMsgParams};

use crate::util::functions::generate_asset_attribute_name;

use super::test_constants::DEFAULT_CONTRACT_BASE_NAME;

// Tests that the DEFAULT_CONTRACT_BASE_NAME message was bound in a message contained in the slice
pub fn test_for_default_base_name(messages: &[SubMsg<ProvenanceMsg>]) {
    test_message_is_name_bind_with_base_name(messages, DEFAULT_CONTRACT_BASE_NAME, true);
}

// Tests that the target asset type was bound as a name (suffixed with the default contract base name)
// in a message in the slice
pub fn test_message_is_name_bind(messages: &[SubMsg<ProvenanceMsg>], expected_asset_type: &str) {
    test_message_is_name_bind_with_base_name(messages, expected_asset_type, false);
}

pub fn test_no_money_moved_in_response<S: Into<String>>(
    response: &Response<ProvenanceMsg>,
    assertion_prefix: S,
) {
    let assertion_prefix = assertion_prefix.into();
    for message in response.messages.iter() {
        assert!(
            !matches!(message.msg, CosmosMsg::Bank(..)),
            "{}: expected no bank messages to be included, but got msg: {:?}",
            assertion_prefix,
            message.msg,
        );
        assert!(
            !matches!(
                message.msg,
                CosmosMsg::Custom(ProvenanceMsg {
                    params: ProvenanceMsgParams::MsgFees(..),
                    ..
                })
            ),
            "{}: expected no provenance messages to be included, but got msg: {:?}",
            assertion_prefix,
            message.msg,
        );
    }
}

pub fn test_aggregate_msg_fees_are_charged<S: Into<String>>(
    response: &Response<ProvenanceMsg>,
    expected_fee_amount: u128,
    assertion_message: S,
) {
    let total_fees = response
        .messages
        .iter()
        .fold(0u128, |agg, msg| match &msg.msg {
            CosmosMsg::Custom(ProvenanceMsg {
                params:
                    ProvenanceMsgParams::MsgFees(MsgFeesMsgParams::AssessCustomFee { amount, .. }),
                ..
            }) => agg + amount.amount.u128(),
            _ => agg,
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
    messages: &[SubMsg<ProvenanceMsg>],
    expected_asset_type: &str,
    is_base_name: bool,
) {
    for message in messages {
        match &message.msg {
            CosmosMsg::Custom(msg) => match &msg.params {
                ProvenanceMsgParams::Name(param) => match param {
                    NameMsgParams::BindName {
                        name,
                        address,
                        restrict,
                    } => {
                        // Wrong name? Go to the next iteration
                        if !name.contains(expected_asset_type) {
                            continue;
                        }
                        assert_eq!(
                            if is_base_name {
                                expected_asset_type.to_string()
                            } else {
                                generate_asset_attribute_name(
                                    expected_asset_type,
                                    DEFAULT_CONTRACT_BASE_NAME,
                                )
                            },
                            name.to_string(),
                            "the default values should be used to derive the attribute name",
                        );
                        assert_eq!(
                            MOCK_CONTRACT_ADDR,
                            address.as_str(),
                            "the default contract address should be bound to",
                        );
                        assert!(
                            restrict,
                            "the restrict value should be set to true for all bound attributes"
                        );
                        // Exit early after finding the appropriate value to ensure the trailing
                        // panic doesn't fire
                        return;
                    }
                    _ => panic!(
                        "unexpected name module message type was emitted: {:?}",
                        param
                    ),
                },
                _ => panic!(
                    "unexpected provenance message type was emitted: {:?}",
                    &msg.params
                ),
            },
            _ => panic!("unexpected message type was emitted: {:?}", &message.msg),
        }
    }
    panic!(
        "failed to find message for expected asset type `{}`",
        expected_asset_type
    );
}
