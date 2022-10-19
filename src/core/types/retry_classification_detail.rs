use crate::core::types::fee_destination::FeeDestinationV2;
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RetryClassificationDetail {
    pub retry_cost: Uint128,
    pub fee_destinations: Vec<FeeDestinationV2>,
}
impl RetryClassificationDetail {
    pub fn new(retry_cost: u128, fee_destinations: &[FeeDestinationV2]) -> Self {
        Self {
            retry_cost: Uint128::new(retry_cost),
            fee_destinations: fee_destinations.iter().cloned().collect(),
        }
    }
}
