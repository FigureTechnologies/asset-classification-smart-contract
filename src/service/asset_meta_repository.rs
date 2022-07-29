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
    fn has_asset<S1: Into<String>>(&self, scope_address: S1) -> AssetResult<bool>;

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

    /// Attempts to fetch a scope attribute currently attached to a scope.  Returns an error if no
    /// scope exists or no scope attribute is attached to the existing scope.
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
    /// asset with the contract.  Moves the [AssetScopeAttribute](crate::core::types::asset_scope_attribute::AssetScopeAttribute)
    /// to [Approved](crate::core::types::asset_onboarding_status::AssetOnboardingStatus::Approved)
    /// status if [trust_verifier](crate::core::types::asset_scope_attribute::AssetScopeAttribute::trust_verifier)
    /// was set to true, or to [AwaitingFinalization](crate::core::types::asset_onboarding_status::AssetOnboardingStatus::AwaitingFinalization)
    /// status if the trust verifier flag was set to false.
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

    /// The third step in the classification process.  This is only necessary when the
    /// [trust_verifier](crate::core::types::asset_scope_attribute::AssetScopeAttribute::trust_verifier)
    /// flag is used during the onboarding process.  This function will mark the scope attribute's
    /// [onboarding_status](crate::core::types::asset_scope_attribute::AssetScopeAttribute::onboarding_status)
    /// as [Approved](crate::core::types::asset_onboarding_status::AssetOnboardingStatus::Approved),
    /// generate custom Provenance Blockchain MsgFees to pay for the verifier's work from a stored
    /// [FeePaymentDetail](crate::core::types::fee_payment_detail::FeePaymentDetail), and then
    /// delete the payment detail.  If the trust verifier flag was set to true, this step will be
    /// skipped, and the asset will be moved to approved status at the end of the [verify_asset](self::AssetMetaRepository::verify_asset)
    /// step.
    ///
    /// # Parameters
    ///
    /// * `env` The environment value supplied during contract execution, used to derive the
    /// contract's address for custom MsgFees.
    /// * `attribute` The attribute attached to an asset scope that is currently in the
    /// [AwaitingFinalization](crate::core::types::asset_onboarding_status::AssetOnboardingStatus::AwaitingFinalization)
    /// onboarding status.
    fn finalize_classification(
        &self,
        env: &Env,
        attribute: &AssetScopeAttribute,
    ) -> AssetResult<()>;
}
