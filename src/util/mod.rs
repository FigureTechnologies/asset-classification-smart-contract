//! Miscellaneous functionalities that do not logically belong to a concrete group.

/// Defines various types with type aliases to shorten syntax used elsewhere in the contract code.
pub mod aliases;
/// Defines all global constant values used throughout the contract.
pub mod constants;
/// Functions that perform common actions for the [execute](crate::contract::execute), [query](crate::contract::query),
/// [instantiate](crate::contract::instantiate), and [migrate](crate::contract::migrate) functions.
pub mod contract_helpers;
/// Allows dynamic delegation of a cosmwasm [DepsMutC](crate::util::aliases::DepsMutC) to prevent
/// common issues that arise when the struct is moved.
pub mod deps_container;
/// Helpers to ensure that emitting event attributes on [execute](crate::contract::execute) calls
/// occurs with standard values throughout the contract.
pub mod event_attributes;
/// Calculation functions for determining how fees should be spent during the [onboarding](crate::execute::onboard_asset::onboard_asset)
/// process.
pub mod fees;
/// Miscellaneous functions to use in various scenarios throughout the contract's execution.
pub mod functions;
/// Utility functions that facilitate interaction with Provenance Blockchain modules.
pub mod provenance_util;
/// Utility functions for interacting with bech32 addresses in the Provenance Blockchain
/// environment.
pub mod scope_address_utils;
/// Global traits to be used across various areas of the contract.
pub mod traits;
/// A container that allows a struct to manage a Vec, mutating its contents without it itself being
/// declared as a mutable instance.
pub mod vec_container;
