use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::{
    core::types::{access_route::AccessRoute, asset_scope_attribute::AssetScopeAttribute},
    util::aliases::AssetResult,
};
use cosmwasm_std::Env;

/// A trait used for fetching and interacting with asset (Provenance Metadata Scope) values.
pub trait AssetMetaRepository {
    /// Determines if a scope exists with an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
    /// emitted by this contract.
    ///
    /// # Parameters
    ///
    /// * `scope_address` The Provenance Metadata Scope bech32 address with an hrp of "scope" that
    /// refers to an existing scope.
    /// * `asset_type` The asset type to query for existence
    fn has_asset<S1: Into<String>, S2: Into<String>>(
        &self,
        scope_address: S1,
        asset_type: S2,
    ) -> AssetResult<bool>;

    /// Attempts to generate the [CosmosMsg](cosmwasm_std::CosmosMsg) values required to onboard
    /// an asset to the contract.
    ///
    /// # Parameters
    ///
    /// * `attribute` The scope attribute to be appended to the Provenance Metadata Scope as a result
    /// of a successful onboarding process.
    /// * `verifier_detail` The verifier chosen by the onboarding account.
    /// * `is_retry` Indicates that this onboarding action was attempted before, and the scope has
    /// an existing scope attribute with a failed verification on it.
    fn onboard_asset(
        &self,
        env: &Env,
        attribute: &AssetScopeAttribute,
        verifier_detail: &VerifierDetailV2,
        is_retry: bool,
    ) -> AssetResult<()>;

    /// Alters the internal values of the [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
    /// currently attached to a Provenance Metadata Scope with the provided values.  This function
    /// will fail if no attribute exists on the target scope.
    ///
    /// # Parameters
    ///
    /// * `updated_attribute` The new attribute to attach to the scope.  The original values will be
    /// entirely replaced with the values contained within this struct.
    fn update_attribute(&self, updated_attribute: &AssetScopeAttribute) -> AssetResult<()>;

    /// Attempts to fetch all asset attributes currently attached to a scope.  Returns an error if no
    /// scope exists or no scope attribute is attached to the existing scope.
    ///
    /// # Parameters
    ///
    /// * `scope_address` A Provenance Blockchain bech32 address with an hrp of "scope".
    fn get_asset<S1: Into<String>>(
        &self,
        scope_address: S1,
    ) -> AssetResult<Vec<AssetScopeAttribute>>;

    /// Attempts to fetch all asset attributes currently attached to a scope.  Returns an error if bad
    /// data exists on the scope (an unlikely occurrence) or None if no scope or scope attributes
    /// exist.
    ///
    /// # Parameters
    ///
    /// * `scope_address` A Provenance Blockchain bech32 address with an hrp of "scope".
    fn try_get_asset<S1: Into<String>>(
        &self,
        scope_address: S1,
    ) -> AssetResult<Option<Vec<AssetScopeAttribute>>>;

    /// Attempts to fetch an attribute currently attached to a scope by asset type.  Returns an error if no
    /// scope exists or no scope attribute is attached to the existing scope.
    ///
    /// # Parameters
    ///
    /// * `scope_address` A Provenance Blockchain bech32 address with an hrp of "scope".
    /// * `asset_type` The asset type to query for
    fn get_asset_by_asset_type<S1: Into<String>, S2: Into<String>>(
        &self,
        scope_address: S1,
        asset_type: S2,
    ) -> AssetResult<AssetScopeAttribute>;

    /// Attempts to fetch asset attributes currently attached to a scope by asset type.  Returns an error if bad
    /// data exists on the scope (an unlikely occurrence) or None if no scope or scope attribute
    /// exists.
    ///
    /// # Parameters
    ///
    /// * `scope_address` A Provenance Blockchain bech32 address with an hrp of "scope".
    /// * `asset_type` The asset type to query for
    fn try_get_asset_by_asset_type<S1: Into<String>, S2: Into<String>>(
        &self,
        scope_address: S1,
        asset_type: S2,
    ) -> AssetResult<Option<AssetScopeAttribute>>;

    /// Attempts to generate the [CosmosMsg](cosmwasm_std::CosmosMsg) values required to verify an
    /// asset with the contract.  Moves the [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
    /// to [Approved](crate::core::types::asset_onboarding_status::AssetOnboardingStatus::Approved)
    ///
    /// # Parameters
    ///
    /// * `scope_attribute` The [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
    /// to be verified.
    /// * `success` Whether or not the scope should be considered verified when the process completes.
    /// * `verification_message` An optional value that will be displayed to external observers when
    /// fetching the [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
    /// associated with the scope.
    /// * `access_routes` Additional access routes that the verifier provides for external consumers
    /// to retrieve the underlying asset data from the scope, potentially without access an object
    /// store.
    fn verify_asset<S: Into<String>>(
        &self,
        scope_attribute: AssetScopeAttribute,
        success: bool,
        verification_message: Option<S>,
        access_routes: Vec<AccessRoute>,
    ) -> AssetResult<AssetScopeAttribute>;
}
