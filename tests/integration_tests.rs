use std::str::FromStr;

use asset_classification_smart_contract::core::{
    msg::{ExecuteMsg, InitMsg, QueryMsg},
    types::{
        asset_definition::AssetDefinitionInputV3, asset_onboarding_status::AssetOnboardingStatus,
        asset_scope_attribute::AssetScopeAttribute,
        asset_verification_result::AssetVerificationResult, fee_destination::FeeDestinationV2,
        serialized_enum::SerializedEnum, verifier_detail::VerifierDetailV2,
    },
};
use cosmwasm_std::{coin, from_json, Uint128};
use provwasm_std::{
    metadata_address::MetadataAddress,
    types::{
        cosmos::bank::v1beta1::QueryBalanceRequest,
        provenance::{
            attribute::v1::QueryAttributesRequest,
            metadata::v1::{
                contract_specification::Source, ContractSpecification,
                MsgWriteContractSpecificationRequest, MsgWriteScopeRequest,
                MsgWriteScopeSpecificationRequest, Party, PartyType, Scope, ScopeSpecification,
            },
        },
    },
};
use provwasm_test_tube::{
    wasm::Wasm, Account, FeeSetting, Module, ProvwasmTestApp, SigningAccount,
};
use uuid::Uuid;

fn get_contract_wasm_bytes() -> &'static Vec<u8> {
    use std::sync::OnceLock;

    static OPTIMIZED_CONTRACT_WASM_PATH: OnceLock<Vec<u8>> = OnceLock::new();
    OPTIMIZED_CONTRACT_WASM_PATH.get_or_init(|| {
        let file_path = std::env::var("OPTIMIZED_CONTRACT_WASM_PATH").unwrap_or(String::from(
            "./artifacts/asset_classification_smart_contract.wasm",
        ));
        std::fs::read(file_path).unwrap()
    })
}

#[test]
#[ignore = "Integration test should not run by default since it depends on the optimized contract WASM file"]
fn happy_path_onboard_and_verify_asset() {
    // provwasm-test-tube v0.2.0 doesn't derive Clone on the account types, so it is impossible
    // to create a meaningfully reusable function for setting up the test environment
    let app = ProvwasmTestApp::default();
    let accs = app
        .init_accounts(&[coin(100_000_000_000_000, "nhash")], 2)
        .expect("initializing accounts should succeed");
    let admin = &accs[0];
    let verifier = &accs[1];
    let originator = app
        .init_account(&[coin(100_000_000_000_000, "nhash")])
        .unwrap();
    let originator_message_fee_nhash = 500_000_000_000;
    let originator = originator.with_fee_setting(FeeSetting::Custom {
        amount: coin(originator_message_fee_nhash, "nhash"),
        gas_limit: 200000000,
    });

    let wasm = Wasm::new(&app);
    let wasm_byte_code = get_contract_wasm_bytes();
    let store_res = wasm
        .store_code(&wasm_byte_code, None, admin)
        .expect("storing the WASM code should succeed");
    let code_id = store_res.data.code_id;
    assert_eq!(code_id, 1);

    let contract_addr = wasm
        .instantiate(
            code_id,
            &InitMsg {
                base_contract_name: String::from("acdemo.pb"),
                bind_base_name: true,
                asset_definitions: vec![AssetDefinitionInputV3 {
                    asset_type: String::from("mortgage"),
                    display_name: Some(String::from("Mortgage")),
                    verifiers: vec![VerifierDetailV2 {
                        address: verifier.address(),
                        onboarding_cost: Uint128::new(30000000000),
                        onboarding_denom: String::from("nhash"),
                        fee_destinations: vec![FeeDestinationV2 {
                            address: verifier.address(),
                            fee_amount: Uint128::new(29999999500),
                            entity_detail: None,
                        }],
                        entity_detail: None,
                        retry_cost: None,
                        subsequent_classification_detail: None,
                    }],
                    enabled: Some(true),
                    bind_name: Some(true),
                }],
                is_test: Some(true),
            },
            Some(&admin.address()),
            Some("testing"),
            &[],
            admin,
        )
        .expect("instantiation should succeed")
        .data
        .address;

    let metadata_module = provwasm_test_tube::metadata::Metadata::new(&app);
    let attribute_module = provwasm_test_tube::attribute::Attribute::new(&app);
    let bank_module = provwasm_test_tube::bank::Bank::new(&app);

    let scope_specification_address = MetadataAddress::scope_specification(
        Uuid::from_str("c370d852-0f3b-4f70-85e6-25983ac07c0f").unwrap(),
    )
    .unwrap();
    let contract_specification_address = MetadataAddress::contract_specification(
        Uuid::from_str("0c163a76-f5b8-8822-c2a6-5fd25d82ed44").unwrap(),
    )
    .unwrap();
    let scope_uuid = "d078755b-6a6b-379a-bddc-01565ffccaea";

    metadata_module
        .write_contract_specification(
            MsgWriteContractSpecificationRequest {
                specification: Some(ContractSpecification {
                    specification_id: contract_specification_address.to_owned().bytes,
                    description: None,
                    owner_addresses: vec![originator.address()],
                    parties_involved: vec![PartyType::Originator.into()],
                    class_name: String::from("SomeClassName"),
                    source: Some(Source::Hash(String::from("someHash"))),
                }),
                signers: vec![originator.address()],
                spec_uuid: String::from("0c163a76-f5b8-8822-c2a6-5fd25d82ed44"),
            },
            &originator,
        )
        .expect("writing a contract specification should succeed");

    metadata_module
        .write_scope_specification(
            MsgWriteScopeSpecificationRequest {
                specification: Some(ScopeSpecification {
                    specification_id: scope_specification_address.to_owned().bytes,
                    description: None,
                    owner_addresses: vec![originator.address()],
                    parties_involved: vec![PartyType::Originator.into()],
                    contract_spec_ids: vec![contract_specification_address.to_owned().bytes],
                }),
                signers: vec![originator.address()],
                spec_uuid: String::from("c370d852-0f3b-4f70-85e6-25983ac07c0f"),
            },
            &originator,
        )
        .expect("writing a scope specification should succeed");

    metadata_module
        .write_scope(
            MsgWriteScopeRequest {
                scope: Some(Scope {
                    scope_id: MetadataAddress::scope(Uuid::from_str(scope_uuid).unwrap())
                        .unwrap()
                        .bytes,
                    specification_id: scope_specification_address.bytes,
                    owners: vec![Party {
                        address: originator.address(),
                        role: PartyType::Originator.into(),
                        optional: false,
                    }],
                    data_access: vec![],
                    value_owner_address: originator.address(),
                    require_party_rollup: true,
                }),
                signers: vec![originator.address()],
                scope_uuid: String::from(scope_uuid),
                spec_uuid: String::new(),
                usd_mills: 0,
            },
            &originator,
        )
        .expect("writing a scope should succeed");

    let get_nhash_balance = |account: &SigningAccount| -> u128 {
        bank_module
            .query_balance(&QueryBalanceRequest {
                address: account.address(),
                denom: String::from("nhash"),
            })
            .expect("querying for nhash balance should succeed")
            .balance
            .expect("nhash balance should exist for account")
            .amount
            .parse::<u128>()
            .expect("account nhash balance should be parseable to a number")
    };

    let originator_nhash_balance_before_onboarding_asset = get_nhash_balance(&originator);

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::OnboardAsset {
            identifier: SerializedEnum {
                r#type: String::from("asset_uuid"),
                value: scope_uuid.to_string(),
            },
            asset_type: String::from("mortgage"),
            verifier_address: verifier.address(),
            access_routes: None,
            add_os_gateway_permission: None,
        },
        &[],
        &originator,
    )
    .expect("onboarding an asset should succeed");

    let originator_nhash_balance_after_onboarding_asset = get_nhash_balance(&originator);

    assert_eq!(
        originator_message_fee_nhash,
        originator_nhash_balance_before_onboarding_asset
            - originator_nhash_balance_after_onboarding_asset,
        "Originator should have been deducated some nhash for executing the onboard asset message"
    );

    let attributes_response = attribute_module
        .query_attributes(&QueryAttributesRequest {
            account: MetadataAddress::scope(Uuid::from_str(scope_uuid).unwrap())
                .unwrap()
                .bech32,
            pagination: None,
        })
        .expect("querying for attributes on the scope should succeed");

    assert_eq!(
        1,
        attributes_response.attributes.len(),
        "Exactly one attribute should exist on the scope"
    );

    let scope_attribute = attributes_response.attributes.first().unwrap();
    let scope_attribute =
        from_json::<AssetScopeAttribute>(scope_attribute.value.as_slice()).unwrap();

    let assert_pending_scope_attribute_is_correct = |scope_attribute: &AssetScopeAttribute| {
        assert_eq!(scope_uuid, scope_attribute.asset_uuid.as_str());
        assert_eq!(
            MetadataAddress::scope(Uuid::from_str(scope_uuid).unwrap())
                .unwrap()
                .bech32,
            scope_attribute.scope_address
        );
        assert_eq!("mortgage", scope_attribute.asset_type.as_str());
        assert_eq!(
            originator.address().as_str(),
            scope_attribute.requestor_address.as_str(),
        );
        assert_eq!(
            verifier.address().as_str(),
            scope_attribute.verifier_address.as_str(),
        );
        assert_eq!(
            AssetOnboardingStatus::Pending,
            scope_attribute.onboarding_status,
        );
        assert_eq!(None, scope_attribute.latest_verification_result);
        assert!(scope_attribute.access_definitions.is_empty());
    };

    assert_pending_scope_attribute_is_correct(&scope_attribute);

    let contract_response_for_scope = wasm
        .query::<QueryMsg, Option<Vec<AssetScopeAttribute>>>(
            &contract_addr,
            &QueryMsg::QueryAssetScopeAttributes {
                identifier: SerializedEnum {
                    r#type: String::from("asset_uuid"),
                    value: scope_uuid.to_string(),
                },
            },
        )
        .expect("querying the contract should succeed");

    if let Some(scope_attributes) = contract_response_for_scope {
        assert_eq!(
            1,
            scope_attributes.len(),
            "Exactly one attribute from the contract should exist on the scope"
        );
        let scope_attribute = scope_attributes.first().unwrap();
        assert_pending_scope_attribute_is_correct(scope_attribute);
    } else {
        panic!("Querying the contract for scope attributes after onboarding returned no results when one result was expected")
    }

    wasm.execute(
        &contract_addr,
        &ExecuteMsg::VerifyAsset {
            identifier: SerializedEnum {
                r#type: String::from("asset_uuid"),
                value: scope_uuid.to_string(),
            },
            asset_type: String::from("mortgage"),
            success: true,
            message: Some(String::from(
                "Successfully verified for the sake of this happy path test",
            )),
            access_routes: None,
        },
        &[],
        verifier,
    )
    .expect("verifying an asset should succeed");

    let attributes_response = attribute_module
        .query_attributes(&QueryAttributesRequest {
            account: MetadataAddress::scope(Uuid::from_str(scope_uuid).unwrap())
                .unwrap()
                .bech32,
            pagination: None,
        })
        .expect("querying for attributes on the scope should succeed");

    assert_eq!(
        1,
        attributes_response.attributes.len(),
        "Exactly one attribute should exist on the scope"
    );

    let scope_attribute = attributes_response.attributes.first().unwrap();
    let scope_attribute =
        from_json::<AssetScopeAttribute>(scope_attribute.value.as_slice()).unwrap();

    let assert_verified_scope_attribute_is_correct = |scope_attribute: &AssetScopeAttribute| {
        assert_eq!(scope_uuid, scope_attribute.asset_uuid.as_str());
        assert_eq!(
            MetadataAddress::scope(Uuid::from_str(scope_uuid).unwrap())
                .unwrap()
                .bech32,
            scope_attribute.scope_address
        );
        assert_eq!("mortgage", scope_attribute.asset_type.as_str());
        assert_eq!(
            originator.address().as_str(),
            scope_attribute.requestor_address.as_str(),
        );
        assert_eq!(
            verifier.address().as_str(),
            scope_attribute.verifier_address.as_str(),
        );
        assert_eq!(
            AssetOnboardingStatus::Approved,
            scope_attribute.onboarding_status,
        );
        assert_eq!(
            Some(AssetVerificationResult {
                message: String::from("Successfully verified for the sake of this happy path test"),
                success: true,
            }),
            scope_attribute.latest_verification_result
        );
        assert!(scope_attribute.access_definitions.is_empty());
    };

    assert_verified_scope_attribute_is_correct(&scope_attribute);

    let contract_response_for_scope = wasm
        .query::<QueryMsg, Option<Vec<AssetScopeAttribute>>>(
            &contract_addr,
            &QueryMsg::QueryAssetScopeAttributes {
                identifier: SerializedEnum {
                    r#type: String::from("asset_uuid"),
                    value: scope_uuid.to_string(),
                },
            },
        )
        .expect("querying the contract should succeed");

    if let Some(scope_attributes) = contract_response_for_scope {
        assert_eq!(
            1,
            scope_attributes.len(),
            "Exactly one attribute from the contract should exist on the scope"
        );
        let scope_attribute = scope_attributes.first().unwrap();
        assert_verified_scope_attribute_is_correct(scope_attribute);
    } else {
        panic!("Querying the contract for scope attributes after verification returned no results when one result was expected")
    }
}
