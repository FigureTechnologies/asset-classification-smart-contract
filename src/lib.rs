//! Asset Classification Smart Contract
//!
//! This contract uses [Cosmwasm](https://github.com/CosmWasm/cosmwasm)'s provided architecture in
//! conjunction with [Provwasm](#https://github.com/provenance-io/provwasm) to create a wasm smart
//! contract that can be deployed to and interact with the Provenance Blockchain.

#![warn(clippy::all)]
/// The entrypoint for all external commands sent to the compiled wasm.
pub mod contract;
pub mod core;
pub mod execute;
pub mod instantiate;
pub mod migrate;
pub mod query;
pub mod service;
pub mod util;
pub mod validation;

// Conditional modules
#[cfg(feature = "enable-test-utils")]
/// A special module only used for facilitating test code, not included in the compiled wasm.
pub mod testutil;
