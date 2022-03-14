use crate::{
    core::asset::{AssetIdentifier, AssetOnboardingStatus, AssetScopeAttribute, ValidatorDetail},
    util::aliases::ContractResult,
};

pub trait AssetMetaRepository {
    fn has_asset<S1: Into<String>>(&self, scope_address: S1) -> ContractResult<bool>;

    fn add_asset<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        &self,
        identifier: &AssetIdentifier,
        asset_type: S1,
        validator_address: S2,
        requestor_address: S3,
        onboarding_status: AssetOnboardingStatus,
        validator_detail: ValidatorDetail,
        access_routes: Vec<String>,
    ) -> ContractResult<()>;

    fn update_asset(&self, attribute: &AssetScopeAttribute) -> ContractResult<()>;

    fn get_asset<S1: Into<String>>(&self, scope_address: S1)
        -> ContractResult<AssetScopeAttribute>;

    fn try_get_asset<S1: Into<String>>(
        &self,
        scope_address: S1,
    ) -> ContractResult<Option<AssetScopeAttribute>>;

    fn validate_asset<S1: Into<String>, S2: Into<String>>(
        &self,
        scope_address: S1,
        success: bool,
        validation_message: Option<S2>,
        access_routes: Vec<String>,
    ) -> ContractResult<()>;
}
