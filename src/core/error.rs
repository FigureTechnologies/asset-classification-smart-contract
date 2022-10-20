use cosmwasm_std::StdError;
use thiserror::Error;

use super::types::asset_onboarding_status::AssetOnboardingStatus;

/// A massive enum including all the different error scenarios that can be encountered throughout
/// each process in the contract.
#[derive(Error, Debug)]
pub enum ContractError {
    ///////////////////////
    //                   //
    // INTERCEPTED TYPES //
    //                   //
    ///////////////////////
    /// An interceptor for a [Bech32 Error](bech32::Error).
    #[error("{0}")]
    Bech32Error(#[from] bech32::Error),

    /// An interceptor for a [SemVer Error](semver::Error).
    #[error("Semver parsing error: {0}")]
    SemVer(#[from] semver::Error),

    /// An interceptor for a [Cosmwasm Error](cosmwasm_std::StdError).
    #[error("{0}")]
    Std(#[from] StdError),

    /// An interceptor for a [Uuid Error](uuid::Error).
    #[error("{0}")]
    UuidError(#[from] uuid::Error),

    //////////////////
    //              //
    // CUSTOM TYPES //
    //              //
    //////////////////
    /// This error is encountered when an asset is attempted in the onboarding process, but it has
    /// already been onboarded and classified.
    #[error("Asset {scope_address} has already been fully onboarded as asset type [{asset_type}]")]
    AssetAlreadyOnboarded {
        /// The bech32 scope address of the already-onboarded asset.
        scope_address: String,
        /// The asset type for which onboarding has already been completed
        asset_type: String,
    },

    /// An error emitted when a verifier attempts to run the verification process on an asset that
    /// does not require it.  This can only occur when a verifier attempts to run a duplicate
    /// verification on a scope that has already had its verification process completed.
    #[error("Asset [{scope_address}] was already verified for asset type [{asset_type}] and has status [{status}]")]
    AssetAlreadyVerified {
        /// The bech32 address of the scope that has already been verified.
        scope_address: String,
        /// The asset type for which verification has already been completed for this asset.
        asset_type: String,
        /// The current onboarding status in the [AssetScopeAttribute](super::types::asset_scope_attribute::AssetScopeAttribute)
        /// on the scope that has already been verified.
        status: AssetOnboardingStatus,
    },

    /// This error is encountered when the onboarding process cannot locate the scope specified by
    /// the requestor.
    #[error("Asset {scope_address} not found")]
    AssetNotFound {
        /// The bech32 address of the scope that does not appear to yet exist on the Provenance Blockchain.
        scope_address: String,
    },

    /// This error is encountered when the onboarding process is attempted for an asset that has
    /// already been onboarded as is awaiting a decision from its target verifier.
    #[error(
        "Asset {scope_address} is currently awaiting verification as asset type {asset_type} from address {verifier_address}"
    )]
    AssetPendingVerification {
        /// The bech32 scope address of the asset that has been onboarded.
        scope_address: String,
        /// The asset type for which verification is pending for this asset.
        asset_type: String,
        /// The bech32 address of the verifier that will perform verification on the asset.
        verifier_address: String,
    },

    /// This error indicates that an asset was attempted to be onboarded with an [asset_type](super::types::asset_definition::AssetDefinitionV3::asset_type)
    /// linked to an [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3) that is
    /// currently not [enabled](super::types::asset_definition::AssetDefinitionV3::enabled)]
    #[error("Asset type {asset_type} is currently disabled")]
    AssetTypeDisabled {
        /// The type of asset that is currently disabled.
        asset_type: String,
    },

    /// Denotes that an existing [VerifierDetailV2](super::types::verifier_detail::VerifierDetailV2)
    /// has the same [address](super::types::verifier_detail::VerifierDetailV2::address) property
    /// as the provided [VerifierDetailV2](super::types::verifier_detail::VerifierDetailV2) to be
    /// added to an [AssetDefinitionV3](crate::core::types::asset_definition::AssetDefinitionV3).
    #[error("duplicate/existing verifier address provided as input")]
    DuplicateVerifierProvided,

    /// An error that can be used in a circumstance where a named error is not necessary to be
    /// created.
    #[error("{msg}")]
    GenericError {
        /// A free-form text description of the error that occurred.
        msg: String,
    },

    /// Indicates that a bech32 address was provided that does not meet proper specifications for the
    /// given scenario.
    #[error("Invalid address provided [{address}]: {explanation}")]
    InvalidAddress {
        /// The invalid bech32 address.
        address: String,
        /// A message further explaining the issue.
        explanation: String,
    },

    /// An error that can occur during a migration that indicates that an incorrect stored contract
    /// code was attempted to be provided for a migration.
    #[error("Current contract name [{current_contract}] does not match provided migration name [{migration_contract}]")]
    InvalidContractName {
        /// The name of the existing contract.  Should correlate to the value specified in Cargo.toml's name property.
        current_contract: String,
        /// The name of the incorrect contract.
        migration_contract: String,
    },

    /// An error that can occur during a migration that indicates that the stored contract code that
    /// was used as the migration target has a version that is lower than the current contract version.
    /// This error is to prevent downgrading the contract to a previous version, which could remove
    /// features and cause problems.
    #[error("Current contract version [{current_version}] is higher than provided migration version [{migration_version}]")]
    InvalidContractVersion {
        /// The version of the contract currently active on the Provenance Blockchain.
        current_version: String,
        /// The version of the stored code used in the migration.
        migration_version: String,
    },

    /// A generic error that specifies that some form of provided or utilized coin was invalid.
    #[error("{0}")]
    InvalidFunds(
        /// Denotes the reason that invalid funds were detected.
        String,
    ),

    /// An error emitted by the [validation](crate::validation) that indicates that input values
    /// were not properly specified (blank strings, invalid numbers, etc).
    #[error("Message of type [{message_type}] was invalid. Invalid fields: {invalid_fields:?}")]
    InvalidMessageFields {
        /// Indicates the type of message that was sent, causing the error.  Will be one of the
        /// values in the [msg file](super::msg).
        message_type: String,
        /// A collection of messages that indicate every issue present in the bad [msg](super::msg).
        invalid_fields: Vec<String>,
    },

    /// An error emitted when an internal issue arises, indicating that an incorrect [msg](super::msg)
    /// was sent to a function receiver.  The functions that use this error are all at first called
    /// into in the [contract file](crate::contract).
    #[error("Invalid message type provided. Expected message type {expected_message_type}")]
    InvalidMessageType {
        /// Denotes the correct [msg](super::msg) that was expected to be provided.
        expected_message_type: String,
    },

    /// An error that indicates that a scope inspected during the onboarding process is missing
    /// internal values and is not valid for onboarding, like an internal Provenance Blockchain Metadata
    /// Record.
    #[error("Invalid scope: {explanation}")]
    InvalidScope {
        /// A free-form text description of the reason that the scope is considered invalid.
        explanation: String,
    },

    /// An error that occurs when a lookup is attempted for a contract resource but the resource
    /// does not exist.  For instance, when an [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3)
    /// does not contain a [VerifierDetailV2](super::types::verifier_detail::VerifierDetailV2) with a
    /// specified bech32 [address](super::types::verifier_detail::VerifierDetailV2::address), this
    /// error will occur.
    #[error("Resource not found: {explanation}")]
    NotFound {
        /// A message describing the reason that the resource could not be found.
        explanation: String,
    },

    /// An error that occurs when a unique key is violated during an attempt to add new data to the
    /// contract's internal storage.  Reference: [state](super::state).
    #[error("Existing record found: {explanation}")]
    RecordAlreadyExists {
        /// A free-form text description of the reason that the record already exists.
        explanation: String,
    },

    /// Occurs when a mandatory data lookup is performed on the contract's internal storage, but
    /// the required value is not found.  Reference: [state](super::state).
    #[error("Record not found: {explanation}")]
    RecordNotFound {
        /// A free-form text description of the record that could not be found.
        explanation: String,
    },

    /// A generic error that occurs when an address attempts to perform an operation in the contract
    /// that it does not have the permission to.
    #[error("Unauthorized: {explanation}")]
    Unauthorized {
        /// A free-form text description of why the action was not authorized.
        explanation: String,
    },

    /// An error emitted when an account attempts to initiate the verification process for a scope
    /// when its bech32 address is not listed as the verifier for the particular onboarding instance.
    #[error("Unauthorized verifier [{verifier_address}] for scope [{scope_address}] as asset type [{asset_type}], expected verifier [{expected_verifier_address}]")]
    UnauthorizedAssetVerifier {
        /// The bech32 address of the scope that is awaiting verification.
        scope_address: String,
        /// The asset type that was used to resolve the verifier
        asset_type: String,
        /// The bech32 address of the account that attempted to run verification.
        verifier_address: String,
        /// The bech32 address of the account that has been requested to run verification.
        expected_verifier_address: String,
    },

    /// This error occurs when a [SerializedEnum](super::types::serialized_enum::SerializedEnum) is
    /// received from a caller that cannot be properly converted to its expected underlying type.
    #[error("Unexpected enum value received. Got type [{received_type}]. {explanation}")]
    UnexpectedSerializedEnum {
        /// The [type](super::types::serialized_enum::SerializedEnum::type) value of the serialized enum
        /// that could not be properly converted.
        received_type: String,
        /// A free-form text description of what went wrong in converting the value to its
        /// underlying type.
        explanation: String,
    },

    /// An error that can occur when the contract is in an unexpected state and refuses to perform
    /// an operation.
    #[error("Unexpected state: {explanation}")]
    UnexpectedState {
        /// A free-form text description of why the operation was rejected.
        explanation: String,
    },

    /// A placeholder error that can be used as a stopgap during the implementation of functionality
    /// to allow the tests to compile and run before the feature is completed.  This error should
    /// never be used in a release build.
    #[error("Requested functionality is not yet implemented")]
    Unimplemented,

    /// This error is encountered when an asset type is attempted to be used during the onboarding
    /// process, but the asset type cannot be found in the contract's internal storage.
    #[error("Unsupported asset type [{asset_type}]")]
    UnsupportedAssetType {
        /// The type of asset that could not be located for onboarding.
        asset_type: String,
    },

    /// This error can occur when a target [VerifierDetailV2](super::types::verifier_detail::VerifierDetailV2)
    /// does not exist in an [AssetDefinitionV3](super::types::asset_definition::AssetDefinitionV3) during
    /// the onboarding process.
    #[error("Unsupported verifier [{verifier_address}] for asset type [{asset_type}]")]
    UnsupportedVerifier {
        /// The bech32 address of the target verifier.
        verifier_address: String,
        /// The [asset_type](super::types::asset_definition::AssetDefinitionV3::asset_type) selected
        /// during onboarding.
        asset_type: String,
    },
}
impl ContractError {
    /// Constructs an instance of the [GenericError](self::ContractError::GenericError) variant,
    /// allowing a generic Into<String> implementation.
    ///
    /// # Parameters
    ///
    /// * `msg` The string value to use as the msg field of the generic error.
    pub fn generic<S: Into<String>>(msg: S) -> ContractError {
        ContractError::GenericError { msg: msg.into() }
    }
}
