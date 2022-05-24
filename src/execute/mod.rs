//! Contains all execution routes used by the [contract file](crate::contract).

/// Contains the functionality used by the [AddAssetDefinition](crate::core::msg::ExecuteMsg::AddAssetDefinition)
/// [ExecuteMsg](crate::core::msg::ExecuteMsg) variant when invoked via the [execute](crate::contract::execute)
/// function.
pub mod add_asset_definition;
/// Contains the functionality used by the [AddAssetVerifier](crate::core::msg::ExecuteMsg::AddAssetVerifier)
/// [ExecuteMsg](crate::core::msg::ExecuteMsg) variant when invoked via the [execute](crate::contract::execute)
/// function.
pub mod add_asset_verifier;
/// Contains the functionality used by the [BindContractAlias](crate::core::msg::ExecuteMsg::BindContractAlias)
/// [ExecuteMsg](crate::core::msg::ExecuteMsg) variant when invoked via the [execute](crate::contract::execute)
/// function.
pub mod bind_contract_alias;
/// Contains the functionality used by the [DeleteAssetDefinition](crate::core::msg::ExecuteMsg::DeleteAssetDefinition)
/// [ExecuteMsg](crate::core::msg::ExecuteMsg) variant when invoked via the [execute](crate::contract::execute)
/// function.
pub mod delete_asset_definition;
/// Contains the functionality used by the [OnboardAsset](crate::core::msg::ExecuteMsg::OnboardAsset)
/// [ExecuteMsg](crate::core::msg::ExecuteMsg) variant when invoked via the [execute](crate::contract::execute)
/// function.
pub mod onboard_asset;
/// Contains the functionality used by the [ToggleAssetDefinition](crate::core::msg::ExecuteMsg::ToggleAssetDefinition)
/// [ExecuteMsg](crate::core::msg::ExecuteMsg) variant when invoked via the [execute](crate::contract::execute)
/// function.
pub mod toggle_asset_definition;
/// Contains the functionality used by the [UpdateAccessRoutes](crate::core::msg::ExecuteMsg::UpdateAccessRoutes)
/// [ExecuteMsg](crate::core::msg::ExecuteMsg) variant when invoked via the [execute](crate::contract::execute)
/// function.
pub mod update_access_routes;
/// Contains the functionality used by the [UpdateAssetDefinition](crate::core::msg::ExecuteMsg::UpdateAssetDefinition)
/// [ExecuteMsg](crate::core::msg::ExecuteMsg) variant when invoked via the [execute](crate::contract::execute)
/// function.
pub mod update_asset_definition;
/// Contains the functionality used by the [UpdateAssetVerifier](crate::core::msg::ExecuteMsg::UpdateAssetVerifier)
/// [ExecuteMsg](crate::core::msg::ExecuteMsg) variant when invoked via the [execute](crate::contract::execute)
/// function.
pub mod update_asset_verifier;
/// Contains the functionality used by the [VerifyAsset](crate::core::msg::ExecuteMsg::VerifyAsset)
/// [ExecuteMsg](crate::core::msg::ExecuteMsg) variant when invoked via the [execute](crate::contract::execute)
/// function.
pub mod verify_asset;
