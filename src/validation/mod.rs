//! Functionality used to ensure the logical integrity of received external values.

/// Validates the integrity of an intercepted [ExecuteMsg](crate::core::msg::ExecuteMsg) variant.
pub mod validate_execute_msg;
/// Validates the integrity of an intercepted [InitMsg](crate::core::msg::InitMsg) and its
/// associated [AssetDefinitionV2](crate::core::types::asset_definition::AssetDefinitionV2) values.
pub mod validate_init_msg;
