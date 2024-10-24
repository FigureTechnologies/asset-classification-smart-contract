//! Complex structs used to perform intensive operations in a centralized location.

/// Defines a trait used for fetching and interacting with asset (Provenance Metadata Scope) values.
pub mod asset_meta_repository;
/// Ties all service code together into a cohesive struct to use for complex operations during the
/// onboarding and verification processes.
pub mod asset_meta_service;
/// Allows dynamic delegation of a cosmwasm [DepsMut](cosmwasm_std::DepsMut) to prevent
/// common issues that arise when the struct is moved.
pub mod deps_manager;
/// Specifies a trait used for dynamically aggregating [CosmosMsg](cosmwasm_std::CosmosMsg) values
/// without requiring the owning struct to be mutable.
pub mod message_gathering_service;
