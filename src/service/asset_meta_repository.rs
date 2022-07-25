use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::{
    core::types::{access_route::AccessRoute, asset_scope_attribute::AssetScopeAttribute},
    util::aliases::AssetResult,
};

/// A trait used for fetching and interacting with asset (Provenance Metadata Scope) values.
pub trait AssetMetaRepository {
    /// Determines if a scope exists with an [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
    /// emitted by this contract.
    ///
    /// # Parameters
    ///
    /// * `scope_address` The Provenance Metadata Scope bech32 address with an hrp of "scope" that
    /// refers to an existing scope.
    fn has_asset<S1: Into<String>>(&self, scope_address: S1) -> AssetResult<bool>;

    /// Attempts to generate the [CosmosMsg](cosmwasm_std::CosmosMsg) values required to onboard
    /// an asset to the contract.
    ///
    /// # Parameters
    ///
    /// * `attribute` The scope attribute to be appended to the Provenance Metadata Scope as a result
    /// of a successful onboarding process.
    /// * `latest_verifier_detail` The verifier detail currently in storage when this scope is
    /// onboarded.  Stored in contract storage until a verification has been completed to ensure that
    /// the proper fee distribution is made when verification completes.
    /// * `is_retry` Indicates that this onboarding action was attempted before, and the scope has
    /// an existing scope attribute with a failed verification on it.
    fn onboard_asset(
        &self,
        attribute: &AssetScopeAttribute,
        latest_verifier_detail: &VerifierDetailV2,
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

    /// Attempts to fetch a scope attribute currently attached to a scope.  Returns an error if no
    /// scope exists or no scope atttribute is attached to the existing scope.
    ///
    /// # Parameters
    ///
    /// * `scope_address` A Provenance Blockchain bech32 address with an hrp of "scope".
    fn get_asset<S1: Into<String>>(&self, scope_address: S1) -> AssetResult<AssetScopeAttribute>;

    /// Attempts to fetch a scope attribute currently attached to a scope.  Returns an error if bad
    /// data exists on the scope (an unlikely occurrence) or None if no scope or scope attribute
    /// exist.
    ///
    /// # Parameters
    ///
    /// * `scope_address` A Provenance Blockchain bech32 address with an hrp of "scope".
    fn try_get_asset<S1: Into<String>>(
        &self,
        scope_address: S1,
    ) -> AssetResult<Option<AssetScopeAttribute>>;

    /// Attempts to generate the [CosmosMsg](cosmwasm_std::CosmosMsg) values required to verify an
    /// asset with the contract.
    ///
    /// # Parameters
    ///
    /// * `scope_address` A Provenance Blockchain bech32 address with an hrp of "scope".  Links to
    /// the desired scope to verify.
    /// * `success` Whether or not the scope should be considered verified when the process completes.
    /// * `verification_message` An optional value that will be displayed to external observers when
    /// fetching the [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
    /// associated with the scope.
    /// * `access_routes` Additional access routes that the verifier provides for external consumers
    /// to retrieve the underlying asset data from the scope, potentially without access an object
    /// store.
    fn verify_asset<S1: Into<String>, S2: Into<String>>(
        &self,
        scope_address: S1,
        success: bool,
        verification_message: Option<S2>,
        access_routes: Vec<AccessRoute>,
    ) -> AssetResult<()>;
}
