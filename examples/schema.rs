use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use asset_classification_smart_contract::core::msg::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};
use asset_classification_smart_contract::core::types::asset_definition::AssetDefinition;
use asset_classification_smart_contract::core::types::asset_identifier::AssetIdentifier;
use asset_classification_smart_contract::core::types::asset_qualifier::AssetQualifier;
use asset_classification_smart_contract::core::types::asset_scope_attribute::AssetScopeAttribute;
use asset_classification_smart_contract::core::types::serialized_enum::SerializedEnum;
use asset_classification_smart_contract::core::types::verifier_detail::VerifierDetail;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);
    export_schema(&schema_for!(AssetScopeAttribute), &out_dir);
    export_schema(&schema_for!(AssetDefinition), &out_dir);
    export_schema(&schema_for!(VerifierDetail), &out_dir);
    export_schema(&schema_for!(AssetIdentifier), &out_dir);
    export_schema(&schema_for!(AssetQualifier), &out_dir);
    export_schema(&schema_for!(SerializedEnum), &out_dir);
}
