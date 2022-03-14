use crate::{core::asset::AssetScopeAttribute, util::aliases::ContractResult};

pub trait AssetMetaRepository {
    fn has_asset<S1: Into<String>>(&self, scope_address: S1) -> ContractResult<bool>;

    fn onboard_asset(&self, attribute: &AssetScopeAttribute, is_retry: bool) -> ContractResult<()>;

    fn update_attribute(&self, attribute: &AssetScopeAttribute) -> ContractResult<()>;

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
