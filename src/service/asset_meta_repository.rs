use crate::{core::asset::AssetScopeAttribute, util::aliases::AssetResult};

pub trait AssetMetaRepository {
    fn has_asset<S1: Into<String>>(&self, scope_address: S1) -> AssetResult<bool>;

    fn onboard_asset(&self, attribute: &AssetScopeAttribute, is_retry: bool) -> AssetResult<()>;

    fn update_attribute(&self, attribute: &AssetScopeAttribute) -> AssetResult<()>;

    fn get_asset<S1: Into<String>>(&self, scope_address: S1) -> AssetResult<AssetScopeAttribute>;

    fn try_get_asset<S1: Into<String>>(
        &self,
        scope_address: S1,
    ) -> AssetResult<Option<AssetScopeAttribute>>;

    fn validate_asset<S1: Into<String>, S2: Into<String>>(
        &self,
        scope_address: S1,
        success: bool,
        validation_message: Option<S2>,
        access_routes: Vec<String>,
    ) -> AssetResult<()>;
}
