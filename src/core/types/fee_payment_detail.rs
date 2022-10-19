use crate::core::error::ContractError;
use crate::core::types::fee_destination::FeeDestinationV2;
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::util::aliases::AssetResult;
use crate::util::functions::bank_send;

use crate::core::types::asset_scope_attribute::AssetScopeAttribute;
use cosmwasm_std::{coin, Addr, Coin, CosmosMsg};
use provwasm_std::ProvenanceMsg;
use result_extensions::ResultExtensions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Defines a fee established from a [VerifierDetailV2](super::verifier_detail::VerifierDetailV2)
/// and its contained [FeeDestinations](super::fee_destination::FeeDestinationV2).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeePaymentDetail {
    /// The bech32 address of the onboarded scope related to the fee.  This address is used as the
    /// unique identifier for the fee, and to retrieve the associated [AssetScopeAttribute](super::asset_scope_attribute::AssetScopeAttribute)
    /// for finding the [requestor's address](super::asset_scope_attribute::AssetScopeAttribute::requestor_address)
    /// to which the fee is charged.
    pub scope_address: String,
    /// The breakdown of each fee charge.  This vector will always at least contain a single charge,
    /// which will be to send a payment to the verifier.
    pub payments: Vec<FeePayment>,
}
impl FeePaymentDetail {
    /// Constructs a new instance of this struct by deriving all required fees from the associated
    /// verifier.
    ///
    /// # Parameters
    ///
    /// * `scope_address` The bech32 address of the scope owned by the requestor, to which the fees
    /// will be charged.
    /// * `onboarding_denom` The denomination to use for all coin amounts when charged as fees.
    /// * `verifier_address` The bech32 address of the verifier that will receive the primary fee.
    /// * `verifier_fee_name` The display name value to use when charging a fee to the onboarding
    /// requestor.
    /// * `onboarding_cost` The cost for onboarding as calculated from a given verifier.  This value
    /// will determine the amount that goes directly to the verifier, as well as any fee values that
    /// are sent to other sources.
    pub fn new<S1: Into<String>, S2: Into<String>>(
        scope_address: S1,
        verifier: &VerifierDetailV2,
        is_retry: bool,
        asset_type: S2,
        existing_scope_attributes: Vec<AssetScopeAttribute>,
    ) -> AssetResult<Self> {
        let mut payments = vec![];
        let mut fee_total: u128 = 0;
        let onboarding_cost =
            verifier.calc_onboarding_cost_source(is_retry, asset_type, existing_scope_attributes);
        // Append a message for each destination
        for destination in onboarding_cost.fee_destinations.iter() {
            payments.push(FeePayment {
                amount: coin(destination.fee_amount.u128(), &verifier.onboarding_denom),
                name: generate_fee_destination_fee_name(destination),
                // All FeeDestination addresses are verified as valid bech32 addresses when they are
                // added to the contract, so this conversion is inherently fine to do
                recipient: Addr::unchecked(&destination.address),
            });
            fee_total += destination.fee_amount.u128();
        }
        // Fee distribution can, at most, be equal to half the onboarding cost (half to account for the 50% fee cut that the custom fee distributes).  The onboarding cost should
        // always reflect the exact total that is taken from the requestor address when onboarding a new
        // scope.
        if fee_total > onboarding_cost.cost.u128() / 2 {
            return ContractError::generic(
                format!("misconfigured fee destinations! fee total ({}{}) was greater than half the specified onboarding cost ({}{} / 2 = {}{})",
                        fee_total,
                        &verifier.onboarding_denom,
                        onboarding_cost.cost.u128(),
                        &verifier.onboarding_denom,
                        onboarding_cost.cost.u128() / 2,
                        &verifier.onboarding_denom,
                )
            ).to_err();
        }
        // The total funds disbursed to the verifier itself is the remainder from subtracting the fee cost from the onboarding cost
        let verifier_cost = (onboarding_cost.cost.u128() / 2) - fee_total;
        // Only append payment info for the verifier if it actually has a cost
        if verifier_cost > 0 {
            payments.push(FeePayment {
                amount: coin(verifier_cost, &verifier.onboarding_denom),
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

    /// Converts all the [payments](self::FeePaymentDetail::payments) into Provenance Blockchain
    /// bank send messages in order to charge them to their respective recipients.
    pub fn to_bank_send_msgs(&self) -> AssetResult<Vec<CosmosMsg<ProvenanceMsg>>> {
        self.payments
            .iter()
            .map(
                |FeePayment {
                     amount: Coin { denom, amount },
                     recipient,
                     ..
                 }| { bank_send(recipient, amount.u128(), denom) },
            )
            .collect::<Vec<_>>()
            .to_ok()
    }

    /// Determines the aggregate amount paid via all payments.
    pub fn sum_costs(&self) -> u128 {
        self.payments
            .iter()
            .map(|payment| payment.amount.amount.u128())
            .sum::<u128>()
    }
}

/// Defines an individual fee to be charged to an account during the asset verification
/// process.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeePayment {
    /// The amount to be charged during the asset verification process.  The denom will always
    /// match the [onboarding_denom](super::verifier_detail::VerifierDetailV2::onboarding_denom)
    /// amount.  The coin's amount will be equal to the amount for a fee_destination in the verifier detail,
    /// and (onboarding_cost / 2) - fee_destination_total for the verifier itself if that amount is > 0.
    pub amount: Coin,
    /// A name describing to the end user (requestor) the purpose and target of the fee.
    pub name: String,
    /// The bech32 address of the recipient of the fee, derived from various fields in the
    /// [VerifierDetailV2](super::verifier_detail::VerifierDetailV2).
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

pub fn generate_verifier_fee_name(verifier: &VerifierDetailV2) -> String {
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
    use crate::core::types::fee_payment_detail::{
        generate_fee_destination_fee_name, generate_verifier_fee_name, FeePaymentDetail,
    };
    use crate::core::types::verifier_detail::VerifierDetailV2;
    use crate::testutil::test_constants::{DEFAULT_ASSET_TYPE, DEFAULT_SCOPE_ADDRESS};
    use crate::testutil::test_utilities::get_default_entity_detail;
    use crate::util::constants::NHASH;
    use crate::util::traits::OptionExtensions;
    use cosmwasm_std::{BankMsg, CosmosMsg, Uint128};
    use provwasm_std::ProvenanceMsg;

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
            NHASH,
            vec![],
            Some(EntityDetail::new(
                "Jeff's Frozen Pizza Emporium",
                "",
                "",
                "",
            )),
            None,
            None,
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
            Uint128::new(200),
            NHASH,
            vec![FeeDestinationV2::new("fee", 101)],
            get_default_entity_detail().to_some(),
            None,
            None,
        );
        let error = FeePaymentDetail::new(
            DEFAULT_SCOPE_ADDRESS,
            &verifier,
            false,
            DEFAULT_ASSET_TYPE,
            vec![],
        )
        .unwrap_err();
        match error {
            ContractError::GenericError { msg } => {
                assert_eq!(
                    "misconfigured fee destinations! fee total (101nhash) was greater than half the specified onboarding cost (200nhash / 2 = 100nhash)",
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
        let verifier = VerifierDetailV2::new(
            "verifier",
            Uint128::new(200),
            NHASH,
            vec![],
            None,
            None,
            None,
        );
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
            Uint128::new(200),
            NHASH,
            vec![FeeDestinationV2::new("fee-destination", 100)],
            None,
            None,
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
            Uint128::new(200),
            NHASH,
            vec![FeeDestinationV2::new("fee-destination", 50)],
            None,
            None,
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
            Uint128::new(400),
            NHASH,
            vec![
                FeeDestinationV2::new("first", 20),
                FeeDestinationV2::new("second", 20),
                FeeDestinationV2::new("third", 40),
                FeeDestinationV2::new("fourth", 5),
                FeeDestinationV2::new("fifth", 15),
            ],
            None,
            None,
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
        FeePaymentDetail::new(
            DEFAULT_SCOPE_ADDRESS,
            &verifier,
            false,
            DEFAULT_ASSET_TYPE,
            vec![],
        )
        .expect("fee payment detail should generate without error")
        .to_bank_send_msgs()
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
                CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                    if to_address.to_owned() == target_address {
                        assert_eq!(
                            1,
                            amount.len(),
                            "exactly one coin should be set on fee bank send message"
                        );
                        assert_eq!(
                            expected_amount,
                            amount.first().unwrap().amount.u128(),
                            "the fee amount should always equal the specified number",
                        );
                        assert_eq!(
                            target_denom,
                            amount.first().unwrap().denom,
                            "the correct denom should be specified in the fee",
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
