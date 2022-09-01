use crate::core::types::asset_definition::AssetDefinitionInputV3;
use crate::core::types::serialized_enum::SerializedEnum;
use crate::core::types::verifier_detail::VerifierDetailV2;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::types::access_route::AccessRoute;

/// The struct used to instantiate the contract.  Utilized in the core [contract file](crate::contract::instantiate).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    /// The root name from which all asset names branch.  All sub-names specified in the [AssetDefinitionV3s](super::types::asset_definition::AssetDefinitionV3)
    /// will use this value as their parent name.
    pub base_contract_name: String,
    /// If `true`, the contract will automatically try to bind its [base_contract_name](self::InitMsg::base_contract_name)
    /// during the instantiation process to itself.  No action will be taken if the value is `false`,
    /// but the base name will still be recorded in the contract's [state](super::state::StateV2)
    /// and be used for child names for [AssetDefinitions](super::types::asset_definition::AssetDefinitionV3).
    pub bind_base_name: bool,
    /// All the initial [AssetDefinitionV3s](super::types::asset_definition::AssetDefinitionV3) for the
    /// contract.  This can be left empty and new definitions can be added later using the [Add Asset Definition](crate::execute::add_asset_definition)
    /// functionality.
    pub asset_definitions: Vec<AssetDefinitionInputV3>,
    /// A boolean value allowing for less restrictions to be placed on certain functionalities
    /// across the contract's execution processes.  Notably, this disables a check during the
    /// onboarding process to determine if onboarded scopes include underlying record values.  This
    /// should never be set to true in a mainnet environment.
    pub is_test: Option<bool>,
}

/// Defines all routes in which the contract can be queried.  These are all handled directly in
/// the [contract file](crate::contract::query).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// This route can be used to retrieve a specific [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3) from the contract's
    /// internal storage for inspection of its verifies and other properties.  If the requested value is not found, a null
    /// response will be returned.
    QueryAssetDefinition {
        /// The asset type to query for
        asset_type: String,
    },
    /// This route can be used to retrieve all [AssetDefinitionV3s](super::types::asset_definition::AssetDefinitionV3) stored in the contract.  This response payload can be quite
    /// large if many complex definitions are stored, so it should only used in circumstances where all asset definitions need
    /// to be inspected or displayed.  The query asset definition route is much more efficient.
    QueryAssetDefinitions {},
    /// This route can be used to retrieve a list of existing [AssetScopeAttribute](super::types::asset_scope_attribute::AssetScopeAttribute)s that have
    /// been added to a [Provenance Metadata Scope](https://docs.provenance.io/modules/metadata-module#metadata-scope) by this
    /// contract.  This route will return a null (empty option) if the scope has never had a scope attribute added to it by the contract.
    /// This is a useful route for external consumers of the contract's data to determine if a scope (aka asset) has been
    /// successfully classified by a verifier.
    QueryAssetScopeAttributes {
        /// Expects an [AssetIdentifier](super::types::asset_identifier::AssetIdentifier)-compatible
        /// [SerializedEnum](super::types::serialized_enum::SerializedEnum).
        identifier: SerializedEnum,
    },
    /// This route can be used to retrieve an existing [AssetScopeAttribute](super::types::asset_scope_attribute::AssetScopeAttribute) that has
    /// been added to a [Provenance Metadata Scope](https://docs.provenance.io/modules/metadata-module#metadata-scope) by this
    /// contract for a specific asset type.  This route will return a null (empty option) if the scope has never had a scope attribute added to it by the contract.
    /// This is a useful route for external consumers of the contract's data to determine if a scope (aka asset) has been
    /// successfully classified by a verifier for a specific asset type.
    QueryAssetScopeAttributeForAssetType {
        /// Expects an [AssetIdentifier](super::types::asset_identifier::AssetIdentifier)-compatible
        /// [SerializedEnum](super::types::serialized_enum::SerializedEnum).
        identifier: SerializedEnum,
        /// The asset type to query for
        asset_type: String,
    },
    /// This route can be used to retrieve an existing [FeePaymentDetail](super::types::fee_payment_detail::FeePaymentDetail)
    /// that has been stored from a [VerifierDetailV2](super::types::verifier_detail::VerifierDetailV2)
    /// during the [OnboardAsset](self::ExecuteMsg::OnboardAsset) execution route's processes.  This
    /// route is useful in showing the expected fees to be paid when the [VerifyAsset](self::ExecuteMsg::VerifyAsset)
    /// route is executed.
    QueryFeePayments {
        /// Expects an [AssetIdentifier](super::types::asset_identifier::AssetIdentifier)-compatible
        /// [SerializedEnum](super::types::serialized_enum::SerializedEnum).
        identifier: SerializedEnum,
        /// The asset type to query for pending verification fee payment details
        asset_type: String,
    },
    /// This route can be used to retrieve the internal contract state values.  These are core configurations that denote how
    /// the contract behaves.  They reflect the values created at instantiation and potentially modified during migration.  It
    /// responds with a [StateV2](super::state::StateV2) struct value.
    QueryState {},
    /// This route can be used to retrieve the internal contract version information.  It elucidates the current version of the
    /// contract that was derived through instantiation or the most recent code migration.  It responds with a [VersionInfoV1](crate::migrate::version_info::VersionInfoV1)
    /// struct value.
    QueryVersion {},
}

/// Defines all routes in which the contract can be executed.  These are all handled directly in
/// the [contract file](crate::contract::execute).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// This route is the primary interaction point for most consumers.  It consumes an asset uuid or scope address, the type of
    /// asset corresponding to that scope (heloc, mortgage, payable, etc), and, if all checks pass, attaches an attribute to the
    /// provided scope that stages the scope for verification of authenticity by the specified verifier in the request.  The
    /// attribute is attached based on the [base_contract_name](self::InitMsg::base_contract_name) specified in the contract, combined with the specified asset type
    /// in the request.  Ex: if [base_contract_name](self::InitMsg::base_contract_name) is "asset" and the asset type is "myasset", the attribute would be assigned
    /// under the name of "myasset.asset".  All available asset types are queryable, and stored in the contract as [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3)
    /// values.  After onboarding is completed, an [AssetScopeAttribute](super::types::asset_scope_attribute::AssetScopeAttribute) will be
    /// stored on the scope with an [AssetOnboardingStatus](super::types::asset_onboarding_status::AssetOnboardingStatus)
    /// of [Pending](super::types::asset_onboarding_status::AssetOnboardingStatus::Pending),
    /// indicating that the asset has been onboarded to the contract but is awaiting verification.
    OnboardAsset {
        /// Expects an [AssetIdentifier](super::types::asset_identifier::AssetIdentifier)-compatible
        /// [SerializedEnum](super::types::serialized_enum::SerializedEnum).
        identifier: SerializedEnum,
        /// A name that must directly match one of the contract's internal [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3)
        /// names.  Any request with a specified type not matching an asset definition will be rejected outright.
        asset_type: String,
        /// The bech32 address of a Verifier Account associated with the targeted [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3),
        /// within its nested vector of [VerifierDetailV2s](super::types::verifier_detail::VerifierDetailV2).
        verifier_address: String,
        /// An optional parameter that allows the specification of a location to get the underlying asset data
        /// for the specified scope.  The [AccessRoute](super::types::access_route::AccessRoute) struct is very generic in its composition
        /// for the purpose of allowing various different solutions to fetching asset data.  If the verification process requires
        /// generic lookups for each onboarded asset, access routes on the scope's [AssetScopeAttribute](super::types::asset_scope_attribute::AssetScopeAttribute)
        /// can be leveraged to easily determine the source of the underlying data.  If these values are omitted at first, but later needed,
        /// they can always be added by using the [UpdateAccessRoutes](self::ExecuteMsg::UpdateAccessRoutes) execution route.
        /// Note: Access routes can specify a [name](super::types::access_route::AccessRoute::name)
        /// parameter, as well, to indicate the reason for the route, but this is entirely optional.
        access_routes: Option<Vec<AccessRoute>>,
    },
    /// This route is specifically designed to allow a Verifier specified in the [AssetScopeAttribute](super::types::asset_scope_attribute::AssetScopeAttribute)
    /// of a [Provenance Metadata Scope](https://docs.provenance.io/modules/metadata-module#scope-data-structures) to indicate to
    /// the owner of the scope whether or not the content within the scope was valid or not.  The Verifier Account, after determining
    /// validity of the underlying data, will either mark the classification as a success or failure.  This route will reject
    /// all invokers except for Verifiers linked to a scope by the scope attribute, ensuring that only the verifier requested
    /// has the permission needed to classify an asset.  In this way, the process for verification ensures that all involved
    /// parties' requirements for security are satisfied.  In addition, the verifier used in the process is stored on the scope
    /// attribute after the fact, ensuring that external inspectors of the generated attribute can choose which verifications to
    /// acknowledge and which to disregard.
    VerifyAsset {
        /// Expects an [AssetIdentifier](super::types::asset_identifier::AssetIdentifier)-compatible
        /// [SerializedEnum](super::types::serialized_enum::SerializedEnum).
        identifier: SerializedEnum,
        /// The asset type this verification result is for
        asset_type: String,
        /// A boolean indicating whether or not verification was successful.  A value of `false` either indicates that
        /// the underlying data was fetched and it did not meet the requirements for a classified asset, or that a failure occurred
        /// during the verification process.  Note: Verifiers should be wary of returning false immediately on a code failure, as
        /// this incurs additional cost to the onboarding account.  Instead, it is recommended that verification implement some
        /// process that retries logic when exceptions or other code execution issues cause a failed verification.
        success: bool,
        /// An optional string describing the result of the verification process.  If omitted, a standard message
        /// describing success or failure based on the value of `success` will be displayed in the [AssetScopeAttribute](super::types::asset_scope_attribute::AssetScopeAttribute).
        message: Option<String>,
        /// Like in the [OnboardAsset](self::ExecuteMsg::OnboardAsset) message, this parameter allows the verifier to provide access routes for
        /// the assets that it has successfully fetched from the underlying scope data.  This allows for the verifier to define its
        /// own subset of [AccessRoute](super::types::access_route::AccessRoute) values to allow actors with permission to easily fetch asset
        /// data from a new location, potentially without any Provenance Blockchain interaction, facilitating the process of data
        /// interaction.
        access_routes: Option<Vec<AccessRoute>>,
    },
    /// __This route is only accessible to the contract's admin address.__  This route allows a new [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3)
    /// value to be added to the contract's internal storage.  These asset definitions dictate which asset types are allowed to
    /// be onboarded, as well as which verifiers are tied to each asset type.  Each added asset definition must be unique in
    /// the following criteria:
    /// * Its [asset_type](super::types::asset_definition::AssetDefinitionV3::asset_type) value must not yet be registered in a different asset definition.
    AddAssetDefinition {
        /// An asset definition input value defining all of the new [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3)'s
        /// values.  The execution route converts the incoming value to an asset definition.
        asset_definition: AssetDefinitionInputV3,
    },
    /// __This route is only accessible to the contract's admin address.__ This route allows an existing [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3)
    /// value to be updated.  It works by matching the input's [asset_type](super::types::asset_definition::AssetDefinitionV3::asset_type) to an existing asset definition and overwriting the
    /// existing values.  If no asset definition exists for the given type, the request will be rejected.
    UpdateAssetDefinition {
        /// An asset definition input value defining all of the updated [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3)'s
        /// values.  The execution route converts the incoming value to an asset definition.
        asset_definition: AssetDefinitionInputV3,
    },
    /// __This route is only accessible to the contract's admin address.__ This route toggles an existing [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3)
    /// from enabled to disabled, or disabled to enabled.  When disabled, an asset definition will no longer allow new assets to
    /// be onboarded to the contract.  Existing assets already onboarded to the contract and in pending status will still be
    /// allowed to be verified, but new values will be rejected.  This same functionality could be achieved with an invocation of
    /// the [UpdateAssetDefinition](self::ExecuteMsg::UpdateAssetDefinition) route but swapping the [enabled](super::types::asset_definition::AssetDefinitionV3::enabled)
    /// value on the `asset_definition` parameter, but this route is significantly simpler and prevents
    /// accidental data mutation due to it not requiring the entirety of the definition's values.
    ToggleAssetDefinition {
        /// The type of asset for which the definition's [enabled](super::types::asset_definition::AssetDefinitionV3::enabled) value will be toggled.  As the asset type value
        /// on each asset definition is guaranteed to be unique, this key is all that is needed to find the target definition.
        asset_type: String,
        /// The value of [enabled](super::types::asset_definition::AssetDefinitionV3::enabled) after the toggle takes place.  This value is required to ensure that
        /// multiple toggles executed in succession (either by accident or by various unrelated callers) will only be honored if
        /// the asset definition is in the intended state during the execution of the route.
        expected_result: bool,
    },
    /// __This route is only accessible to the contract's admin address or the address of the verifier being updated.__ This route adds a new [VerifierDetailV2](super::types::verifier_detail::VerifierDetailV2)
    /// to an existing [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3).  This route is intended to register new verifiers
    /// without the bulky requirements of the [UpdateAssetDefinition](self::ExecuteMsg::UpdateAssetDefinition) execution route.  This route will reject verifiers added
    /// with addresses that match any other verifiers on the target asset definition.
    AddAssetVerifier {
        /// The type of asset for which the new [VerifierDetailV2](super::types::verifier_detail::VerifierDetailV2) will be added.
        /// This must refer to an existing [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3)'s [asset_type](super::types::asset_definition::AssetDefinitionV3::asset_type)
        /// value, or the request will be rejected.
        asset_type: String,
        /// The new verifier detail to be added to the asset definition, with all of its required
        /// values.  No verifiers within the existing asset definition must have the same [address](super::types::verifier_detail::VerifierDetailV2::address) value of this
        /// parameter, or the request will be rejected.
        verifier: VerifierDetailV2,
    },
    /// __This route is only accessible to the contract's admin address.__ This route updates an existing [VerifierDetailV2](super::types::verifier_detail::VerifierDetailV2)
    /// in an existing [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3).  This route is intended to be used when the values
    /// of a single verifier detail need to change, but not the entire asset definition.  The request will be rejected if the
    /// referenced asset definition is not present within the contract, or if a verifier does not exist within the asset
    /// definition that matches the address of the provided verifier data.
    UpdateAssetVerifier {
        /// The type of asset for which the [VerifierDetailV2](super::types::verifier_detail::VerifierDetailV2) will be updated. This
        /// must refer to an existing [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3)'s [asset_type](super::types::asset_definition::AssetDefinitionV3::asset_type)
        /// value, or the request will be rejected.
        asset_type: String,
        /// The updated verifier detail to be modified in the asset definition. An existing verifier
        /// detail within the target asset definition must have a matching [address](super::types::verifier_detail::VerifierDetailV2::address)
        /// value, or the request will be rejected.
        verifier: VerifierDetailV2,
    },
    /// __This route is only accessible to the contract's admin address OR to the owner of the access routes being updated.__
    /// This route will swap all existing access routes for a specific owner for a specific scope to the provided values. These
    /// access routes either correspond to those created during the onboarding process, or those created during the verification
    /// process.
    UpdateAccessRoutes {
        /// Expects an [AssetIdentifier](super::types::asset_identifier::AssetIdentifier)-compatible
        /// [SerializedEnum](super::types::serialized_enum::SerializedEnum).
        identifier: SerializedEnum,
        /// The asset type to update access routes for
        asset_type: String,
        /// Corresponds to the bech32 address of the account that originally created the [AccessRoutes](super::types::access_route::AccessRoute).
        /// These values can be found in the [AccessDefinition](super::types::access_definition::AccessDefinition) of the [AssetScopeAttribute](super::types::asset_scope_attribute::AssetScopeAttribute)
        /// tied to a scope after the onboarding process occurs.
        owner_address: String,
        /// A vector of [AccessRoute](super::types::access_route::AccessRoute) to be used instead of the existing routes.
        /// If other existing routes need to be maintained and the updated is intended to simply add a new route, then the existing
        /// routes need to be included in the request alongside the new route(s).
        access_routes: Vec<AccessRoute>,
    },
    /// __This route is only accessible to the contract's admin address.__ This route facilitates the removal of bad data.
    /// IMPORTANT: If an asset definition is completely removed, all contract references to it will
    /// fail to function.  This can cause assets currently in the onboarding process for a deleted
    /// type to have failures when interactions occur with them.  This functionality should only be
    /// used for an unused type!
    DeleteAssetDefinition {
        /// The asset type to delete the definition for
        asset_type: String,
    },
}

/// The struct used to migrate the contract from one code instance to another.  Utilized in the core
/// [contract file](crate::contract::migrate).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {
    /// Performs a standard migration using the underlying [migrate_contract](crate::migrate::migrate_contract::migrate_contract)
    /// function.
    ContractUpgrade {
        /// Various optional values that dictate additional behavior that can occur during a contract
        /// upgrade.
        options: Option<MigrationOptions>,
    },
}

/// Sub-level struct that defines optional changes that can occur during the migration process.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrationOptions {
    /// Sets the contract admin to a new address when populated.  Must be a valid Provenance
    /// Blockchain bech32 address.
    pub new_admin_address: Option<String>,
}
impl MigrationOptions {
    /// Notes whether or not any options have been specified.
    pub fn has_changes(&self) -> bool {
        self.new_admin_address.is_some()
    }
}
