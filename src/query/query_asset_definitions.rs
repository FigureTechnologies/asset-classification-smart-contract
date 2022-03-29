struct AssetDefinitionQueryResult {
    pub asset_type: String,
    pub scope_spec_id: String,
}

pub fn query_asset_definitions(deps: &DepsC) -> AssetResult<Binary> {
    asset_definitions()
}
