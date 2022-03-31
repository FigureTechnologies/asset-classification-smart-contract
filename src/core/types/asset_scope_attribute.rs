use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    core::{error::ContractError, types::access_definition::AccessDefinitionType},
    util::{
        aliases::AssetResult,
        functions::filter_valid_access_routes,
        scope_address_utils::bech32_string_to_addr,
        traits::{OptionExtensions, ResultExtensions},
    },
};

use super::{
    access_definition::AccessDefinition, access_route::AccessRoute,
    asset_identifier::AssetIdentifier, asset_onboarding_status::AssetOnboardingStatus,
    asset_verification_result::AssetVerificationResult, verifier_detail::VerifierDetail,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetScopeAttribute {
    pub asset_uuid: String,
    pub scope_address: String,
    pub asset_type: String,
    pub requestor_address: Addr,
    pub verifier_address: Addr,
    pub onboarding_status: AssetOnboardingStatus,
    pub latest_verifier_detail: Option<VerifierDetail>,
    pub latest_verification_result: Option<AssetVerificationResult>,
    pub access_definitions: Vec<AccessDefinition>,
}
impl AssetScopeAttribute {
    /// Constructs a new instance of AssetScopeAttribute from the input params
    /// Prefer initializing a scope attribute with this function!
    /// It ensures passed addresses are valid, as well as ensuring that the
    /// asset uuid and scope address match each other
    pub fn new<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        identifier: &AssetIdentifier,
        asset_type: S1,
        requestor_address: S2,
        verifier_address: S3,
        onboarding_status: Option<AssetOnboardingStatus>,
        latest_verifier_detail: VerifierDetail,
        access_routes: Vec<AccessRoute>,
    ) -> AssetResult<Self> {
        let identifiers = identifier.to_identifiers()?;
        let req_addr = bech32_string_to_addr(requestor_address)?;
        let ver_addr = bech32_string_to_addr(verifier_address)?;
        if ver_addr != latest_verifier_detail.address {
            return ContractError::generic(format!("provided verifier address [{}] did not match the verifier detail's address [{}]", ver_addr, latest_verifier_detail.address).as_str()).to_err();
        }
        // Remove all access routes that are empty strings to prevent bad data from being provided
        let filtered_access_routes = filter_valid_access_routes(access_routes);
        // If access routes were provided as an empty array, or the array only contains empty strings, don't create an access definition for the requestor
        let access_definitions = if filtered_access_routes.is_empty() {
            vec![]
        } else {
            vec![AccessDefinition::new_checked(
                &req_addr,
                filtered_access_routes,
                AccessDefinitionType::Requestor,
            )?]
        };
        AssetScopeAttribute {
            asset_uuid: identifiers.asset_uuid,
            scope_address: identifiers.scope_address,
            asset_type: asset_type.into(),
            requestor_address: req_addr,
            verifier_address: ver_addr,
            onboarding_status: onboarding_status.unwrap_or(AssetOnboardingStatus::Pending),
            latest_verifier_detail: latest_verifier_detail.to_some(),
            latest_verification_result: None,
            access_definitions,
        }
        .to_ok()
    }
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::{
        core::types::{
            access_route::AccessRoute, asset_identifier::AssetIdentifier,
            asset_onboarding_status::AssetOnboardingStatus,
            asset_scope_attribute::AssetScopeAttribute,
        },
        testutil::{
            test_constants::{
                DEFAULT_ASSET_UUID, DEFAULT_SENDER_ADDRESS, DEFAULT_VERIFIER_ADDRESS,
            },
            test_utilities::{assert_single_item, get_default_verifier_detail},
        },
        util::traits::OptionExtensions,
    };

    #[test]
    fn test_new_asset_scope_attribute_filters_bad_access_routes() {
        let attribute = AssetScopeAttribute::new(
            &AssetIdentifier::asset_uuid(DEFAULT_ASSET_UUID),
            "heloc",
            DEFAULT_SENDER_ADDRESS,
            DEFAULT_VERIFIER_ADDRESS,
            AssetOnboardingStatus::Pending.to_some(),
            get_default_verifier_detail(),
            vec![
                AccessRoute::route_only("    "),
                AccessRoute::route_only("  "),
                AccessRoute::route_only(""),
                AccessRoute::route_only("good route"),
            ],
        )
        .expect("validation should succeed for a properly-formatted asset scope attribute");
        let access_definition = assert_single_item(
            &attribute.access_definitions,
            "there should be one access definition created when at least one valid route is provided in the access routes",
        );
        let access_route = assert_single_item(
            &access_definition.access_routes,
            "only one access definition should be added because the rest were invalid strings",
        );
        assert_eq!(
            "good route", access_route.route,
            "the only access route should be the route that contained a proper string",
        );
    }

    #[test]
    fn test_new_asset_scope_attribute_creates_no_definition_when_no_valid_routes_are_provided() {
        let attribute = AssetScopeAttribute::new(
            &AssetIdentifier::asset_uuid(DEFAULT_ASSET_UUID),
            "heloc",
            DEFAULT_SENDER_ADDRESS,
            DEFAULT_VERIFIER_ADDRESS,
            AssetOnboardingStatus::Pending.to_some(),
            get_default_verifier_detail(),
            vec![
                AccessRoute::route_only("    "),
                AccessRoute::route_only("  "),
                AccessRoute::route_only(""),
            ],
        )
        .expect("validation should succeed for a properly-formatted asset scope attribute");
        assert!(
            attribute.access_definitions.is_empty(),
            "there should not be any access definitions when no valid access routes are provided",
        );
    }

    #[test]
    fn test_new_asset_scope_attribute_creates_no_definition_when_no_routes_are_provided() {
        let attribute = AssetScopeAttribute::new(
            &AssetIdentifier::asset_uuid(DEFAULT_ASSET_UUID),
            "heloc",
            DEFAULT_SENDER_ADDRESS,
            DEFAULT_VERIFIER_ADDRESS,
            AssetOnboardingStatus::Pending.to_some(),
            get_default_verifier_detail(),
            vec![],
        )
        .expect("validation should succeed for a properly-formatted asset scope attribute");
        assert!(
            attribute.access_definitions.is_empty(),
            "there should not be any access definitions when no access routes are provided",
        );
    }

    #[test]
    fn test_new_asset_scope_attribute_trims_access_routes() {
        let attribute = AssetScopeAttribute::new(
            &AssetIdentifier::asset_uuid(DEFAULT_ASSET_UUID),
            "heloc",
            DEFAULT_SENDER_ADDRESS,
            DEFAULT_VERIFIER_ADDRESS,
            AssetOnboardingStatus::Pending.to_some(),
            get_default_verifier_detail(),
            vec![AccessRoute::route_and_name(
                "   test-route   ",
                "my cool name                 ",
            )],
        )
        .expect("validation should succeed for a properly-formatted asset scope attribute");
        let access_definition = assert_single_item(
            &attribute.access_definitions,
            "a single access definition should be kept because an access route was defined",
        );
        let access_route = assert_single_item(
            &access_definition.access_routes,
            "only one access route should be present in the access definition",
        );
        assert_eq!(
            "test-route", access_route.route,
            "the access route's route property should be trimmed",
        );
        assert_eq!(
            "my cool name",
            access_route
                .name
                .expect("the access route should have a valid name"),
            "the access route's name property should be properly trimmed",
        );
    }

    #[test]
    fn test_new_asset_scope_attribute_keeps_duplicate_routes_with_different_names() {
        let attribute = AssetScopeAttribute::new(
            &AssetIdentifier::asset_uuid(DEFAULT_ASSET_UUID),
            "heloc",
            DEFAULT_SENDER_ADDRESS,
            DEFAULT_VERIFIER_ADDRESS,
            AssetOnboardingStatus::Pending.to_some(),
            get_default_verifier_detail(),
            vec![
                AccessRoute::route_and_name("test-route", "name1"),
                AccessRoute::route_and_name("test-route", "name2"),
            ],
        )
        .expect("validation should succeed for a properly-formatted asset scope attribute");
        assert_eq!(
            1,
            attribute.access_definitions.len(),
            "an access definition should be kept because access routes were defined",
        );
        let routes = attribute
            .access_definitions
            .first()
            .unwrap()
            .access_routes
            .clone();
        assert_eq!(
            2,
            routes.len(),
            "both access routes should be kept because they have different names",
        );
        assert!(
            routes.iter().any(
                |r| r.to_owned().name.expect("all names should be Some") == "name1"
                    && r.route == "test-route"
            ),
            "expected the name1 access route to be included in the vector and keep its proper name",
        );
        assert!(
            routes.iter().any(
                |r| r.to_owned().name.expect("name should be Some") == "name2"
                    && r.route == "test-route"
            ),
            "expected the name2 access route to be included in the vector and keep its proper name",
        );
    }

    #[test]
    fn test_new_asset_scope_attribute_keeps_duplicate_routes_with_some_and_none_names() {
        let attribute = AssetScopeAttribute::new(
            &AssetIdentifier::asset_uuid(DEFAULT_ASSET_UUID),
            "heloc",
            DEFAULT_SENDER_ADDRESS,
            DEFAULT_VERIFIER_ADDRESS,
            AssetOnboardingStatus::Pending.to_some(),
            get_default_verifier_detail(),
            vec![
                AccessRoute::route_and_name("test-route", "hey look at my name right here"),
                AccessRoute::route_only("test-route"),
            ],
        )
        .expect("validation should succeed for a properly-formatted asset scope attribute");
        assert_eq!(
            1,
            attribute.access_definitions.len(),
            "an access definition should be kept because access routes were defined",
        );
        let routes = attribute
            .access_definitions
            .first()
            .unwrap()
            .access_routes
            .clone();
        assert_eq!(
            2,
            routes.len(),
            "both access routes should be kept because they have different names",
        );
        assert!(
            routes
                .iter()
                .any(|r| r.to_owned().name.unwrap_or("not the expected name".to_string())
                    == "hey look at my name right here"
                    && r.route == "test-route"),
            "expected the populated name access route to be included in the vector and keep its proper name",
        );
        assert!(
            routes
                .iter()
                .any(|r| r.to_owned().name.is_none() && r.route == "test-route"),
            "expected the name2 access route to be included in the vector and keep its proper name",
        );
    }

    #[test]
    fn test_new_asset_scope_attribute_skips_duplicate_routes_after_trimming_them() {
        let attribute = AssetScopeAttribute::new(
            &AssetIdentifier::asset_uuid(DEFAULT_ASSET_UUID),
            "heloc",
            DEFAULT_SENDER_ADDRESS,
            DEFAULT_VERIFIER_ADDRESS,
            AssetOnboardingStatus::Pending.to_some(),
            get_default_verifier_detail(),
            vec![
                AccessRoute::route_and_name("test-route     ", "myname"),
                AccessRoute::route_and_name("test-route", "myname    "),
            ],
        )
        .expect("validation should succeed for a properly-formatted asset scope attribute");
        let access_definition = assert_single_item(
            &attribute.access_definitions,
            "a single access definition should be kept because an access route should be added",
        );
        let access_route = assert_single_item(
            &access_definition.access_routes,
            "only one access route should be present in the access definition due to them being identical after trimming",
        );
        assert_eq!(
            "test-route", access_route.route,
            "the trimmed route name should be produced correctly",
        );
        assert_eq!(
            "myname",
            access_route.name.expect("the name should be set"),
            "the trimmed name should be produced correctly",
        );
    }
}
