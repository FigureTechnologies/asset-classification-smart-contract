use crate::core::msg::MigrationOptions;
use crate::core::state::{asset_definitions, insert_asset_definition_v2};
use crate::core::types::asset_definition::AssetDefinitionV2;
use crate::migrate::migrate_contract::migrate_contract;
use crate::util::aliases::{DepsMutC, EntryPointResponse};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::Order;

pub fn migrate_to_asset_definition_v2(
    deps: DepsMutC,
    options: Option<MigrationOptions>,
) -> EntryPointResponse {
    let asset_definitions = asset_definitions()
        .range_raw(deps.storage, None, None, Order::Descending)
        .map(|item| item.unwrap().1.to_v2())
        .collect::<Vec<AssetDefinitionV2>>();
    let migrated_def_count = asset_definitions.len();
    for definition in asset_definitions.into_iter() {
        insert_asset_definition_v2(deps.storage, &definition)?;
    }
    // Pass through the existing migration code to ensure version upgrades also occur
    migrate_contract(deps, options)?
        .add_attribute("asset_definitions_migrated", migrated_def_count.to_string())
        .to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::contract::migrate;
    use crate::core::msg::MigrateMsg;
    use crate::core::state::{
        asset_definitions, load_asset_definition_v2_by_scope_spec, load_asset_definition_v2_by_type,
    };
    use crate::core::types::asset_definition::AssetDefinition;
    use crate::core::types::fee_destination::FeeDestination;
    use crate::core::types::verifier_detail::VerifierDetail;
    use crate::migrate::version_info::{set_version_info, VersionInfoV1, CONTRACT_NAME};
    use crate::testutil::test_utilities::{
        assert_single_item, assert_single_item_by, get_default_entity_detail,
        single_attribute_for_key, test_instantiate_success, InstArgs, MockOwnedDeps,
    };
    use crate::util::traits::OptionExtensions;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::{Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_migrate_without_asset_definitions() {
        let mut deps = mock_dependencies(&[]);
        setup_test(&mut deps);
        let response = migrate(
            deps.as_mut(),
            mock_env(),
            MigrateMsg::MigrateToAssetDefinitionV2 { options: None },
        )
        .expect("expected the migration to run without issue");
        assert!(
            response.messages.is_empty(),
            "expected no messages to be emitted by the migration"
        );
        assert_eq!(
            3,
            response.attributes.len(),
            "expected the proper number of attributes to be emitted"
        );
        assert_eq!(
            "0",
            single_attribute_for_key(&response, "asset_definitions_migrated"),
            "expected no asset definitions to be migrated because none existed in the contract state",
        );
    }

    #[test]
    fn test_migrate_with_asset_definitions() {
        let mut deps = mock_dependencies(&[]);
        setup_test(&mut deps);
        // Insert a basic heloc asset definition with no fee destinations
        asset_definitions()
            .save(
                deps.as_mut().storage,
                "heloc".as_bytes(),
                &AssetDefinition {
                    asset_type: "heloc".to_string(),
                    scope_spec_address: "heloc-scope-spec-address".to_string(),
                    verifiers: vec![VerifierDetail {
                        address: "heloc-verifier-address".to_string(),
                        onboarding_cost: Uint128::new(100),
                        onboarding_denom: "nhash".to_string(),
                        fee_percent: Decimal::zero(),
                        fee_destinations: vec![],
                        entity_detail: get_default_entity_detail().to_some(),
                    }],
                    enabled: true,
                },
            )
            .expect("expected the heloc asset definition to be added to storage");
        // Insert a complex mortgage asset definition with fee destinations
        asset_definitions()
            .save(
                deps.as_mut().storage,
                "mortgage".as_bytes(),
                &AssetDefinition {
                    asset_type: "mortgage".to_string(),
                    scope_spec_address: "mortgage-scope-spec-address".to_string(),
                    verifiers: vec![VerifierDetail {
                        address: "mortgage-verifier-address".to_string(),
                        onboarding_cost: Uint128::new(500),
                        onboarding_denom: "mortmoney".to_string(),
                        fee_percent: Decimal::percent(50),
                        fee_destinations: vec![
                            FeeDestination {
                                address: "fee-destination-1".to_string(),
                                fee_percent: Decimal::percent(70),
                            },
                            FeeDestination {
                                address: "fee-destination-2".to_string(),
                                fee_percent: Decimal::percent(30),
                            },
                        ],
                        entity_detail: None,
                    }],
                    enabled: false,
                },
            )
            .expect("expected the mortgage asset definition to be added to storage");
        let response = migrate(
            deps.as_mut(),
            mock_env(),
            MigrateMsg::MigrateToAssetDefinitionV2 { options: None },
        )
        .expect("expected the migration to complete successfully");
        assert!(
            response.messages.is_empty(),
            "expected no messages to be emitted by the migration"
        );
        assert_eq!(
            3,
            response.attributes.len(),
            "expected the proper number of attributes to be emitted",
        );
        assert_eq!(
            "2",
            single_attribute_for_key(&response, "asset_definitions_migrated"),
            "expected two asset definitions to be migrated because two were in contract storage",
        );
        load_asset_definition_v2_by_scope_spec(deps.as_ref().storage, "heloc-scope-spec-address")
            .expect("expected the heloc definition to be able to load by scope spec address");
        let heloc_definition = load_asset_definition_v2_by_type(deps.as_ref().storage, "heloc")
            .expect("expected the heloc definition to exist in storage by type");
        assert_eq!(
            "heloc", heloc_definition.asset_type,
            "expected the heloc asset type to be properly ported",
        );
        assert_eq!(
            "heloc-scope-spec-address", heloc_definition.scope_spec_address,
            "expected the heloc scope spec address to be properly ported",
        );
        assert!(
            heloc_definition.enabled,
            "expected the heloc enabled property to be properly ported",
        );
        let heloc_verifier = assert_single_item(
            &heloc_definition.verifiers,
            "expected the heloc definition to have a single verifier",
        );
        assert_eq!(
            "heloc-verifier-address", heloc_verifier.address,
            "expected the heloc verifier's address to be properly ported",
        );
        assert_eq!(
            100,
            heloc_verifier.onboarding_cost.u128(),
            "expected the heloc verifier's onboarding cost to be properly ported",
        );
        assert_eq!(
            "nhash", heloc_verifier.onboarding_denom,
            "expected the heloc verifier's onboarding denom to be properly ported",
        );
        assert_eq!(
            0,
            heloc_verifier.fee_amount.u128(),
            "expected the heloc verifier's fee amount to be properly derived",
        );
        assert!(
            heloc_verifier.fee_destinations.is_empty(),
            "expected no definitions to exist for the heloc verifier"
        );
        assert_eq!(
            get_default_entity_detail(),
            heloc_verifier
                .entity_detail
                .expect("expected the heloc verifier to have an entity detail"),
            "expected the heloc verifier's entity detail to be properly ported",
        );
        load_asset_definition_v2_by_scope_spec(
            deps.as_ref().storage,
            "mortgage-scope-spec-address",
        )
        .expect("expected the mortgage definition to be able to load by scope spec address");
        let mortgage_definition =
            load_asset_definition_v2_by_type(deps.as_ref().storage, "mortgage")
                .expect("expected the mortgage definition to exist in storage by type");
        assert_eq!(
            "mortgage", mortgage_definition.asset_type,
            "expected the mortgage asset type to be properly ported",
        );
        assert_eq!(
            "mortgage-scope-spec-address", mortgage_definition.scope_spec_address,
            "expected the mortgage scope spec address to be properly ported",
        );
        assert_eq!(
            false, mortgage_definition.enabled,
            "expected the mortgage enabled property to be properly ported",
        );
        let mortgage_verifier = assert_single_item(
            &mortgage_definition.verifiers,
            "expected the mortgage definition to have a single verifier",
        );
        assert_eq!(
            "mortgage-verifier-address", mortgage_verifier.address,
            "expected the mortgage verifier's address to be properly ported",
        );
        assert_eq!(
            500,
            mortgage_verifier.onboarding_cost.u128(),
            "expected the mortgage verifier's onboarding cost to be properly ported",
        );
        assert_eq!(
            "mortmoney", mortgage_verifier.onboarding_denom,
            "expected the mortgage verifier's onboarding denom to be properly ported",
        );
        assert_eq!(
            250,
            mortgage_verifier.fee_amount.u128(),
            "expected the mortgage verifier's fee amount to be properly derived",
        );
        let fee_destination_1 = assert_single_item_by(
            &mortgage_verifier.fee_destinations,
            "expected a single fee destination to exist for the mortgage verifier with address 'fee-destination-1'",
            |dest| dest.address == "fee-destination-1",
        );
        assert_eq!(
            175,
            fee_destination_1.fee_amount.u128(),
            "expected the fee amount for the first fee destination to be properly derived",
        );
        assert!(
            fee_destination_1.entity_detail.is_none(),
            "expected no entity detail to be populated on a fresh port of a fee destination"
        );
        let fee_destination_2 = assert_single_item_by(
            &mortgage_verifier.fee_destinations,
            "expected a single fee destination to exist for the mortgage verifier with address 'fee-destination-2'",
            |dest| dest.address == "fee-destination-2",
        );
        assert_eq!(
            75,
            fee_destination_2.fee_amount.u128(),
            "expected the fee amount for the second fee destination to be properly derived",
        );
        assert!(
            fee_destination_2.entity_detail.is_none(),
            "expected no entity detail to be populated on a fresh port of a fee destination"
        );
        assert!(
            mortgage_verifier.entity_detail.is_none(),
            "expected the lack of entity detail on the mortgage verifier to be properly ported"
        );
    }

    fn setup_test(deps: &mut MockOwnedDeps) {
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        // Downgrade the app version to ensure version collisions don't cause a rejected migration
        set_version_info(
            deps.as_mut().storage,
            &VersionInfoV1 {
                contract: CONTRACT_NAME.to_string(),
                version: "0.0.0".to_string(),
            },
        )
        .expect("expected the version number override to succeed");
    }
}
