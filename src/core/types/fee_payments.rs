use crate::core::error::ContractError;
use crate::core::types::fee_destination::FeeDestinationV2;
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::util::aliases::AssetResult;
use crate::util::traits::{OptionExtensions, ResultExtensions};
use cosmwasm_std::{coin, Addr, Coin, CosmosMsg, Env};
use provwasm_std::{assess_custom_fee, ProvenanceMsg};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FeePaymentDetail {
    pub scope_address: String,
    pub payments: Vec<FeePayment>,
}
impl FeePaymentDetail {
    pub fn new<S: Into<String>>(
        scope_address: S,
        verifier: &VerifierDetailV2,
    ) -> AssetResult<Self> {
        let mut payments = vec![];
        let mut fee_total: u128 = 0;
        // Append a message for each destination
        for destination in verifier.fee_destinations.iter() {
            payments.push(FeePayment {
                // Always multiply the fee amount by 2.  This is because the Provenance Fee Module
                // takes 50% of the value provided.  Doubling the value in this way will ensure that
                // all fee destinations get exactly the amount requested in the FeeDestination.
                amount: coin(
                    destination.fee_amount.u128() * 2,
                    &verifier.onboarding_denom,
                ),
                name: generate_fee_destination_fee_name(destination),
                // All FeeDestination addresses are verified as valid bech32 addresses when they are
                // added to the contract, so this conversion is inherently fine to do
                recipient: Addr::unchecked(&destination.address),
            });
            fee_total += destination.fee_amount.u128();
        }
        // Fee distribution can, at most, be equal to the onboarding cost.  The onboarding cost should
        // always reflect the exact total that is taken from the requestor address when onboarding a new
        // scope.
        if fee_total > verifier.onboarding_cost.u128() {
            return ContractError::generic(
                format!("misconfigured fee destinations! fee total ({}{}) was greater than the specified onboarding cost ({}{})",
                        fee_total,
                        &verifier.onboarding_denom,
                        verifier.onboarding_cost.u128(),
                        verifier.onboarding_denom,
                )
            ).to_err();
        }
        // The total funds disbursed to the verifier itself is the remainder from subtracting the fee cost from the onboarding cost
        let verifier_cost = verifier.onboarding_cost.u128() - fee_total;
        // Only append payment info for the verifier if it actually has a cost
        if verifier_cost > 0 {
            payments.push(FeePayment {
                // Always double charge to ensure the expected fee amount reaches the verifier
                amount: coin(verifier_cost * 2, &verifier.onboarding_denom),
                name: generate_verifier_fee_name(verifier),
                recipient: Addr::unchecked(&verifier.address),
            });
        }
        FeePaymentDetail {
            scope_address: scope_address.into(),
            payments,
        }
        .to_ok()
    }

    pub fn to_fee_msgs(&self, env: &Env) -> AssetResult<Vec<CosmosMsg<ProvenanceMsg>>> {
        let mut messages = vec![];
        for payment in self.payments.iter().cloned() {
            messages.push(assess_custom_fee(
                payment.amount,
                payment.name.to_some(),
                env.contract.address.to_owned(),
                Some(payment.recipient),
            )?);
        }
        messages.to_ok()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FeePayment {
    pub amount: Coin,
    pub name: String,
    pub recipient: Addr,
}

fn generate_fee_destination_fee_name(destination: &FeeDestinationV2) -> String {
    format!(
        "Fee for {}",
        destination
            .entity_detail
            .to_owned()
            .and_then(|detail| detail.name)
            .unwrap_or_else(|| destination.address.to_owned()),
    )
}

fn generate_verifier_fee_name(verifier: &VerifierDetailV2) -> String {
    verifier
        .entity_detail
        .to_owned()
        .and_then(|detail| detail.name)
        .map(|detail_name| format!("{} Verifier Fee", detail_name))
        .unwrap_or_else(|| "Verifier Fee".to_string())
}

#[cfg(test)]
mod tests {
    use crate::core::error::ContractError;
    use crate::core::types::entity_detail::EntityDetail;
    use crate::core::types::fee_destination::FeeDestinationV2;
    use crate::core::types::fee_payments::{
        generate_fee_destination_fee_name, generate_verifier_fee_name, FeePaymentDetail,
    };
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::testutil::test_constants::DEFAULT_SCOPE_ADDRESS;
    use crate::testutil::test_utilities::get_default_entity_detail;
    use crate::util::constants::{NHASH, USD};
    use crate::util::traits::OptionExtensions;
    use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{CosmosMsg, Uint128};
    use provwasm_std::{MsgFeesMsgParams, ProvenanceMsg, ProvenanceMsgParams};

    #[test]
    fn test_generate_fee_destination_fee_name() {
        let mut fee_destination = FeeDestinationV2 {
            address: "someaddress".to_string(),
            fee_amount: Uint128::new(150),
            entity_detail: Some(EntityDetail::new("selling fake doors", "", "", "")),
        };
        assert_eq!(
            "Fee for selling fake doors".to_string(),
            generate_fee_destination_fee_name(&fee_destination),
            "the correct fee name should be generated when an entity detail is set on the fee detail",
        );
        if let Some(entity_detail) = &mut fee_destination.entity_detail {
            entity_detail.name = None;
        }
        assert_eq!(
            "Fee for someaddress".to_string(),
            generate_fee_destination_fee_name(&fee_destination),
            "the correct fee name should be generated from the destination address when the destination has no entity detail name",
        );
        fee_destination.entity_detail = None;
        assert_eq!(
            "Fee for someaddress".to_string(),
            generate_fee_destination_fee_name(&fee_destination),
            "the correct fee name should be generated from the destination address when the destination has no entity detail",
        );
    }

    #[test]
    fn test_generate_verifier_fee_name() {
        let mut verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            USD,
            vec![],
            Some(EntityDetail::new(
                "Jeff's Frozen Pizza Emporium",
                "",
                "",
                "",
            )),
        );
        assert_eq!(
            "Jeff's Frozen Pizza Emporium Verifier Fee".to_string(),
            generate_verifier_fee_name(&verifier),
            "the correct fee name should be used when an entity detail exists",
        );
        if let Some(entity_detail) = &mut verifier.entity_detail {
            entity_detail.name = None;
        };
        assert_eq!(
            "Verifier Fee".to_string(),
            generate_verifier_fee_name(&verifier),
            "the correct fee name should be used when the entity detail has no name",
        );
        verifier.entity_detail = None;
        assert_eq!(
            "Verifier Fee".to_string(),
            generate_verifier_fee_name(&verifier),
            "the correct fee name should be used when the entity detail does not exist",
        );
    }

    #[test]
    fn test_invalid_verifier_greater_fee_than_onboarding_cost() {
        // This verifier tries to send 150% of the fee to the fee destination. NO BUENO!
        let verifier = VerifierDetailV2::new(
            "address",
            Uint128::new(100),
            NHASH,
            vec![FeeDestinationV2::new("fee", Uint128::new(101))],
            get_default_entity_detail().to_some(),
        );
        let error = FeePaymentDetail::new(DEFAULT_SCOPE_ADDRESS, &verifier).unwrap_err();
        match error {
            ContractError::GenericError { msg } => {
                assert_eq!(
                    "misconfigured fee destinations! fee total (101nhash) was greater than the specified onboarding cost (100nhash)",
                    msg.as_str(),
                    "unexpected error message generated",
                );
            }
            _ => panic!(
                "unexpected error encountered when providing a bad verifier: {:?}",
                error
            ),
        }
    }

    #[test]
    fn test_only_send_to_verifier() {
        let verifier = VerifierDetailV2::new("verifier", Uint128::new(100), NHASH, vec![], None);
        let messages = test_get_messages(&verifier);
        assert_eq!(
            1,
            messages.len(),
            "expected only a single message to be sent"
        );
        test_messages_contains_fee_for_address(
            &messages,
            "verifier",
            100,
            NHASH,
            "expected all funds to be sent to the verifier",
        );
    }

    #[test]
    fn test_only_send_to_single_fee_destination() {
        let verifier = VerifierDetailV2::new(
            "verifier",
            Uint128::new(100),
            NHASH,
            vec![FeeDestinationV2::new("fee-destination", Uint128::new(100))],
            None,
        );
        let messages = test_get_messages(&verifier);
        assert_eq!(
            1,
            messages.len(),
            "expected only a single message to be sent",
        );
        test_messages_contains_fee_for_address(
            &messages,
            "fee-destination",
            100,
            NHASH,
            "expected all funds to be sent to the fee destination",
        );
    }

    #[test]
    fn test_even_split_between_verifier_and_single_fee_destination() {
        let verifier = VerifierDetailV2::new(
            "verifier",
            Uint128::new(100),
            NHASH,
            vec![FeeDestinationV2::new("fee-destination", Uint128::new(50))],
            None,
        );
        let messages = test_get_messages(&verifier);
        assert_eq!(2, messages.len(), "expected two messages to be sent",);
        test_messages_contains_fee_for_address(
            &messages,
            "verifier",
            50,
            NHASH,
            "expected half of the funds to be sent to the verifier",
        );
        test_messages_contains_fee_for_address(
            &messages,
            "fee-destination",
            50,
            NHASH,
            "expected half of the funds to be sent to the fee destination",
        );
    }

    #[test]
    fn test_many_fee_destinations_and_some_to_verifier() {
        let verifier = VerifierDetailV2::new(
            "verifier",
            Uint128::new(200),
            NHASH,
            vec![
                FeeDestinationV2::new("first", Uint128::new(20)),
                FeeDestinationV2::new("second", Uint128::new(20)),
                FeeDestinationV2::new("third", Uint128::new(40)),
                FeeDestinationV2::new("fourth", Uint128::new(5)),
                FeeDestinationV2::new("fifth", Uint128::new(15)),
            ],
            None,
        );
        let messages = test_get_messages(&verifier);
        assert_eq!(6, messages.len(), "expected six messages to be sent");
        test_messages_contains_fee_for_address(
            &messages,
            "verifier",
            100,
            NHASH,
            "expected half of all funds to be sent to the verifier",
        );
        test_messages_contains_fee_for_address(
            &messages,
            "first",
            20,
            NHASH,
            "expected 20 nhash of the fee to be sent to the first fee destination",
        );
        test_messages_contains_fee_for_address(
            &messages,
            "second",
            20,
            NHASH,
            "expected 20 nhash of the fee to be sent to the second fee destination",
        );
        test_messages_contains_fee_for_address(
            &messages,
            "third",
            40,
            NHASH,
            "expected 40 nhash of the fee to be sent to the third fee destination",
        );
        test_messages_contains_fee_for_address(
            &messages,
            "fourth",
            5,
            NHASH,
            "expected 5 nhash of the fee to be sent to the fourth fee destination",
        );
        test_messages_contains_fee_for_address(
            &messages,
            "fifth",
            15,
            NHASH,
            "expected 15 nhash of the fee to be sent to the fifth fee destination",
        );
    }

    fn test_get_messages(verifier: &VerifierDetailV2) -> Vec<CosmosMsg<ProvenanceMsg>> {
        FeePaymentDetail::new(DEFAULT_SCOPE_ADDRESS, verifier)
            .expect("fee payment detail should generate without error")
            .to_fee_msgs(&mock_env())
            .expect("fee messages should generate without error")
    }

    /// Loops through all messages contained in the input slice until it finds a message with the given address,
    /// ensuring that the expected amount was sent in the expected denom to that address.  All output errors are
    /// prefixed with the input error_message string.
    fn test_messages_contains_fee_for_address<S: Into<String>, D: Into<String>, M: Into<String>>(
        messages: &[CosmosMsg<ProvenanceMsg>],
        address: S,
        expected_amount: u128,
        expected_denom: D,
        error_message: M,
    ) {
        let target_address: String = address.into();
        let target_denom: String = expected_denom.into();
        let err_msg: String = error_message.into();
        messages
            .iter()
            .find(|msg| match msg {
                CosmosMsg::Custom(ProvenanceMsg {
                    params:
                        ProvenanceMsgParams::MsgFees(MsgFeesMsgParams::AssessCustomFee {
                            amount,
                            name,
                            from,
                            recipient,
                        }),
                    ..
                }) => {
                    if recipient
                        .to_owned()
                        .expect("all recipients should be set in generated fees")
                        .as_str()
                        == target_address
                    {
                        assert_eq!(
                            expected_amount * 2,
                            amount.amount.u128(),
                            "the fee amount should always be double the specified number",
                        );
                        assert_eq!(
                            target_denom, amount.denom,
                            "the correct denom should be specified in the fee",
                        );
                        assert!(name.is_some(), "fee names should always be set",);
                        assert_eq!(
                            MOCK_CONTRACT_ADDR,
                            from.as_str(),
                            "the contract address should always bet set in the from field",
                        );
                        // Return true - this is the correct address and has passed assertions
                        true
                    } else {
                        // Return false - this is a custom fee message, but not to the expected address
                        false
                    }
                }
                _ => false,
            })
            .unwrap_or_else(|| {
                panic!(
                    "{}: could not find address {} in any custom fee messages",
                    err_msg, target_address,
                )
            });
    }
}
