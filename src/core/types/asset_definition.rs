use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::types::serialized_enum::SerializedEnum;
use crate::core::types::verifier_detail::VerifierDetailV2;
use crate::{
    core::state::config_read_v2,
    util::{
        aliases::{AssetResult, DepsC},
        functions::generate_asset_attribute_name,
        traits::ResultExtensions,
    },
};

use super::verifier_detail::VerifierDetail;

// TODO: Delete after upgrading all contract instances to AssetDefinitionV2
/// Defines a specific asset type associated with the contract.  Allows its specified type to be
/// onboarded and verified.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetDefinition {
    /// The unique name of the asset associated with the definition.
    pub asset_type: String,
    /// A link to a scope specification that defines this asset type.
    pub scope_spec_address: String,
    /// Individual verifier definitions.  There can be many verifiers for a single asset type.
    pub verifiers: Vec<VerifierDetail>,
    /// Indicates whether or not the asset definition is enabled for use in the contract.  If disabled,
    /// requests to onboard assets of this type will be rejected.
    pub enabled: bool,
}
impl AssetDefinition {
    /// Constructs a new instance of AssetDefinition, setting enabled to `true` by default.
    ///
    /// # Parameters
    ///
    /// * `asset_type` The unique name of the asset associated with the definition.
    /// * `scope_spec_address` A link to a scope specification that defines this asset type.
    /// * `verifiers` Individual verifier definitions.
    pub fn new<S1: Into<String>, S2: Into<String>>(
        asset_type: S1,
        scope_spec_address: S2,
        verifiers: Vec<VerifierDetail>,
    ) -> Self {
        AssetDefinition {
            asset_type: asset_type.into(),
            scope_spec_address: scope_spec_address.into(),
            verifiers,
            enabled: true,
        }
    }

    /// Converts the asset_type value to lowercase and serializes it as bytes,
    /// then uplifts the value to a vector to allow it to be returned.
    pub fn storage_key(&self) -> Vec<u8> {
        self.asset_type.to_lowercase().as_bytes().to_vec()
    }

    /// Helper functionality to retrieve the base contract name from state and use it to create the
    /// Provenance Blockchain Attribute Module name for this asset type.
    ///
    /// # Parameters
    ///
    /// * `deps` A read-only instance of the cosmwasm-provided DepsC value.
    pub fn attribute_name(&self, deps: &DepsC) -> AssetResult<String> {
        let state = config_read_v2(deps.storage).load()?;
        generate_asset_attribute_name(&self.asset_type, state.base_contract_name).to_ok()
    }

    pub fn to_v2(self) -> AssetDefinitionV2 {
        AssetDefinitionV2 {
            asset_type: self.asset_type,
            scope_spec_address: self.scope_spec_address,
            verifiers: self
                .verifiers
                .into_iter()
                .map(|verifier| verifier.to_v2())
                .collect(),
            enabled: self.enabled,
        }
    }
}

// TODO: Delete after upgrading all contract instances to AssetDefinitionV2
/// Allows the user to optionally specify the enabled flag on an asset definition, versus forcing
/// it to be added manually on every request, when it will likely always be specified as `true`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetDefinitionInput {
    /// The name of the asset associated with the definition.  This value must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    pub asset_type: String,
    /// A link to a scope specification that defines this asset type.  A serialized version of a
    /// [ScopeSpecIdentifier](super::scope_spec_identifier::ScopeSpecIdentifier) that allows multiple
    /// different values to be derived as a scope specification address.  Must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    pub scope_spec_identifier: SerializedEnum,
    /// Individual verifier definitions.  There can be many verifiers for a single asset type.  Each
    /// value must have a unique `address` property or requests to add will be rejected.
    pub verifiers: Vec<VerifierDetail>,
    /// Indicates whether or not the asset definition is enabled for use in the contract.  If disabled,
    /// requests to onboard assets of this type will be rejected.
    pub enabled: Option<bool>,
    /// Whether or not to bind a Provenance Blockchain Name Module name to this contract when this
    /// struct is used to add a new asset type to the contract.  If this value is omitted OR set to
    /// true in a request that adds an asset definition, the name derived by combining the
    /// [base_contract_name](crate::core::state::StateV2::base_contract_name) and the `asset_type`
    /// will be bound to the contract.  For example, if the base name is "pb" and the asset type is
    /// "myasset," the resulting bound name would be "myasset.pb".
    pub bind_name: Option<bool>,
}
impl AssetDefinitionInput {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `asset_type` The name of the asset associated with the definition.  This value must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    /// * `scope_spec_identifier` A link to a scope specification that defines this asset type.
    /// A serialized version of a [ScopeSpecIdentifier](super::scope_spec_identifier::ScopeSpecIdentifier) that allows multiple
    /// different values to be derived as a scope specification address.  Must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    /// * `verifiers` Individual verifier definitions.  There can be many verifiers for a single asset type.  Each
    /// value must have a unique `address` property or requests to add will be rejected.
    /// * `enabled` Indicates whether or not the asset definition is enabled for use in the contract.  If disabled,
    /// requests to onboard assets of this type will be rejected.
    /// * `bind_name` Whether or not to bind a Provenance Blockchain Name Module name to this contract when this
    /// struct is used to add a new asset type to the contract.
    pub fn new<S1: Into<String>>(
        asset_type: S1,
        scope_spec_identifier: SerializedEnum,
        verifiers: Vec<VerifierDetail>,
        enabled: Option<bool>,
        bind_name: Option<bool>,
    ) -> AssetDefinitionInput {
        AssetDefinitionInput {
            asset_type: asset_type.into(),
            scope_spec_identifier,
            verifiers,
            enabled,
            bind_name,
        }
    }

    /// Moves this struct into an instance of [AssetDefinition](self::AssetDefinition), converting
    /// the contained `scope_spec_identifier` enum value into a string scope spec address.
    pub fn into_asset_definition(self) -> AssetResult<AssetDefinition> {
        AssetDefinition {
            asset_type: self.asset_type,
            scope_spec_address: self
                .scope_spec_identifier
                .to_scope_spec_identifier()?
                .get_scope_spec_address()?,
            verifiers: self.verifiers,
            enabled: self.enabled.unwrap_or(true),
        }
        .to_ok()
    }

    /// Clones the values contained within this struct into an instance of [AssetDefinition](self::AssetDefinition).
    /// This process is more expensive than moving the struct with [into_asset_definition](self::AssetDefinitionInput::into_asset_definition).
    pub fn as_asset_definition(&self) -> AssetResult<AssetDefinition> {
        AssetDefinition::new(
            &self.asset_type,
            self.scope_spec_identifier
                .to_scope_spec_identifier()?
                .get_scope_spec_address()?,
            self.verifiers.clone(),
        )
        .to_ok()
    }
}

/// Defines a specific asset type associated with the contract.  Allows its specified type to be
/// onboarded and verified.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetDefinitionV2 {
    /// The unique name of the asset associated with the definition.
    pub asset_type: String,
    /// A link to a scope specification that defines this asset type.
    pub scope_spec_address: String,
    /// Individual verifier definitions.  There can be many verifiers for a single asset type.
    pub verifiers: Vec<VerifierDetailV2>,
    /// Indicates whether or not the asset definition is enabled for use in the contract.  If disabled,
    /// requests to onboard assets of this type will be rejected.
    pub enabled: bool,
}
impl AssetDefinitionV2 {
    /// Constructs a new instance of AssetDefinitionV2, setting enabled to `true` by default.
    ///
    /// # Parameters
    ///
    /// * `asset_type` The unique name of the asset associated with the definition.
    /// * `scope_spec_address` A link to a scope specification that defines this asset type.
    /// * `verifiers` Individual verifier definitions.
    pub fn new<S1: Into<String>, S2: Into<String>>(
        asset_type: S1,
        scope_spec_address: S2,
        verifiers: Vec<VerifierDetailV2>,
    ) -> Self {
        AssetDefinitionV2 {
            asset_type: asset_type.into(),
            scope_spec_address: scope_spec_address.into(),
            verifiers,
            enabled: true,
        }
    }

    /// Converts the asset_type value to lowercase and serializes it as bytes,
    /// then uplifts the value to a vector to allow it to be returned.
    pub fn storage_key(&self) -> Vec<u8> {
        self.asset_type.to_lowercase().as_bytes().to_vec()
    }

    /// Helper functionality to retrieve the base contract name from state and use it to create the
    /// Provenance Blockchain Attribute Module name for this asset type.
    ///
    /// # Parameters
    ///
    /// * `deps` A read-only instance of the cosmwasm-provided DepsC value.
    pub fn attribute_name(&self, deps: &DepsC) -> AssetResult<String> {
        let state = config_read_v2(deps.storage).load()?;
        generate_asset_attribute_name(&self.asset_type, state.base_contract_name).to_ok()
    }
}

// TODO: Delete after upgrading all contract instances to AssetDefinitionV2
/// Allows the user to optionally specify the enabled flag on an asset definition, versus forcing
/// it to be added manually on every request, when it will likely always be specified as `true`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AssetDefinitionInputV2 {
    /// The name of the asset associated with the definition.  This value must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    pub asset_type: String,
    /// A link to a scope specification that defines this asset type.  A serialized version of a
    /// [ScopeSpecIdentifier](super::scope_spec_identifier::ScopeSpecIdentifier) that allows multiple
    /// different values to be derived as a scope specification address.  Must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    pub scope_spec_identifier: SerializedEnum,
    /// Individual verifier definitions.  There can be many verifiers for a single asset type.  Each
    /// value must have a unique `address` property or requests to add will be rejected.
    pub verifiers: Vec<VerifierDetailV2>,
    /// Indicates whether or not the asset definition is enabled for use in the contract.  If disabled,
    /// requests to onboard assets of this type will be rejected.
    pub enabled: Option<bool>,
    /// Whether or not to bind a Provenance Blockchain Name Module name to this contract when this
    /// struct is used to add a new asset type to the contract.  If this value is omitted OR set to
    /// true in a request that adds an asset definition, the name derived by combining the
    /// [base_contract_name](crate::core::state::StateV2::base_contract_name) and the `asset_type`
    /// will be bound to the contract.  For example, if the base name is "pb" and the asset type is
    /// "myasset," the resulting bound name would be "myasset.pb".
    pub bind_name: Option<bool>,
}
impl AssetDefinitionInputV2 {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `asset_type` The name of the asset associated with the definition.  This value must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    /// * `scope_spec_identifier` A link to a scope specification that defines this asset type.
    /// A serialized version of a [ScopeSpecIdentifier](super::scope_spec_identifier::ScopeSpecIdentifier) that allows multiple
    /// different values to be derived as a scope specification address.  Must be unique across all
    /// instances persisted in contract storage, or requests to add will be rejected.
    /// * `verifiers` Individual verifier definitions.  There can be many verifiers for a single asset type.  Each
    /// value must have a unique `address` property or requests to add will be rejected.
    /// * `enabled` Indicates whether or not the asset definition is enabled for use in the contract.  If disabled,
    /// requests to onboard assets of this type will be rejected.
    /// * `bind_name` Whether or not to bind a Provenance Blockchain Name Module name to this contract when this
    /// struct is used to add a new asset type to the contract.
    pub fn new<S1: Into<String>>(
        asset_type: S1,
        scope_spec_identifier: SerializedEnum,
        verifiers: Vec<VerifierDetailV2>,
        enabled: Option<bool>,
        bind_name: Option<bool>,
    ) -> Self {
        Self {
            asset_type: asset_type.into(),
            scope_spec_identifier,
            verifiers,
            enabled,
            bind_name,
        }
    }

    /// Moves this struct into an instance of [AssetDefinitionV2](self::AssetDefinitionV2), converting
    /// the contained `scope_spec_identifier` enum value into a string scope spec address.
    pub fn into_asset_definition(self) -> AssetResult<AssetDefinitionV2> {
        AssetDefinitionV2 {
            asset_type: self.asset_type,
            scope_spec_address: self
                .scope_spec_identifier
                .to_scope_spec_identifier()?
                .get_scope_spec_address()?,
            verifiers: self.verifiers,
            enabled: self.enabled.unwrap_or(true),
        }
        .to_ok()
    }

    /// Clones the values contained within this struct into an instance of [AssetDefinitionV2](self::AssetDefinitionV2).
    /// This process is more expensive than moving the struct with [into_asset_definition](self::AssetDefinitionInputV2::into_asset_definition).
    pub fn as_asset_definition(&self) -> AssetResult<AssetDefinitionV2> {
        AssetDefinitionV2 {
            asset_type: self.asset_type.clone(),
            scope_spec_address: self
                .scope_spec_identifier
                .to_scope_spec_identifier()?
                .get_scope_spec_address()?,
            verifiers: self.verifiers.clone(),
            enabled: self.enabled.unwrap_or(true),
        }
        .to_ok()
    }
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::core::types::asset_definition::AssetDefinition;
    use crate::core::types::entity_detail::EntityDetail;
    use crate::core::types::fee_destination::FeeDestination;
    use crate::core::types::verifier_detail::VerifierDetail;
    use crate::testutil::test_utilities::{assert_single_item, assert_single_item_by};
    use crate::util::traits::OptionExtensions;
    use cosmwasm_std::{Decimal, Uint128};

    #[test]
    fn test_to_v2_for_no_fees() {
        let definition = AssetDefinition {
            asset_type: "heloc".to_string(),
            scope_spec_address: "scope-spec-address".to_string(),
            verifiers: vec![VerifierDetail {
                address: "verifier-address".to_string(),
                onboarding_cost: Uint128::new(100),
                onboarding_denom: "nhash".to_string(),
                fee_percent: Decimal::zero(),
                fee_destinations: vec![],
                entity_detail: get_valid_entity_detail().to_some(),
            }],
            enabled: true,
        };
        let v2 = definition.to_v2();
        assert_eq!(
            "heloc", v2.asset_type,
            "expected the asset type to be properly ported",
        );
        assert_eq!(
            "scope-spec-address", v2.scope_spec_address,
            "expected the scope spec address to be properly ported",
        );
        let verifier = assert_single_item(
            &v2.verifiers,
            "expected only a single verifier to be ported",
        );
        assert_eq!(
            "verifier-address", verifier.address,
            "expected the verifier address to be properly ported",
        );
        assert_eq!(
            100,
            verifier.onboarding_cost.u128(),
            "expected the verifier onboarding cost to be properly ported",
        );
        assert_eq!(
            "nhash", verifier.onboarding_denom,
            "expected the verifier onboarding denom to be properly ported",
        );
        assert_eq!(
            0,
            verifier.fee_amount.u128(),
            "expected the verifier fee amount to be properly ported",
        );
        assert!(
            verifier.fee_destinations.is_empty(),
            "expected no fee destinations to be ported",
        );
        assert_eq!(
            get_valid_entity_detail(),
            verifier
                .entity_detail
                .expect("expected the entity detail to be present in the ported value"),
            "expected the entity detail to be properly ported",
        );
    }

    #[test]
    fn test_to_v2_with_one_fee_destination() {
        let definition = AssetDefinition {
            asset_type: "heloc".to_string(),
            scope_spec_address: "scope-spec-address".to_string(),
            verifiers: vec![VerifierDetail {
                address: "verifier-address".to_string(),
                onboarding_cost: Uint128::new(150),
                onboarding_denom: "nhash".to_string(),
                fee_percent: Decimal::percent(50),
                fee_destinations: vec![FeeDestination::new(
                    "fee-destination",
                    Decimal::percent(100),
                )],
                entity_detail: get_valid_entity_detail().to_some(),
            }],
            enabled: true,
        };
        let v2 = definition.to_v2();
        assert_eq!(
            "heloc", v2.asset_type,
            "expected the asset type to be properly ported",
        );
        assert_eq!(
            "scope-spec-address", v2.scope_spec_address,
            "expected the scope spec address to be properly ported",
        );
        let verifier = assert_single_item(
            &v2.verifiers,
            "expected only a single verifier to be ported",
        );
        assert_eq!(
            "verifier-address", verifier.address,
            "expected the verifier address to be properly ported",
        );
        assert_eq!(
            150,
            verifier.onboarding_cost.u128(),
            "expected the verifier onboarding cost to be properly ported",
        );
        assert_eq!(
            "nhash", verifier.onboarding_denom,
            "expected the verifier onboarding denom to be properly ported",
        );
        assert_eq!(
            75,
            verifier.fee_amount.u128(),
            "expected the verifier fee amount to be properly derived",
        );
        let fee_destination = assert_single_item(
            &verifier.fee_destinations,
            "expected a single fee destination to be ported",
        );
        assert_eq!(
            "fee-destination", fee_destination.address,
            "expected the fee destination address to be properly ported",
        );
        assert_eq!(
            75,
            fee_destination.fee_amount.u128(),
            "expected the fee destination's fee amount to be properly derived",
        );
        assert!(
            fee_destination.entity_detail.is_none(),
            "expected no entity detail to be populated on the fee destination",
        );
        assert_eq!(
            get_valid_entity_detail(),
            verifier
                .entity_detail
                .expect("expected the entity detail to be present in the ported value"),
            "expected the entity detail to be properly ported",
        );
    }

    #[test]
    fn test_to_v2_with_multiple_fee_destinations() {
        let definition = AssetDefinition {
            asset_type: "mortgage".to_string(),
            scope_spec_address: "scope-spec-address".to_string(),
            verifiers: vec![VerifierDetail {
                address: "verifier-address".to_string(),
                onboarding_cost: Uint128::new(1000),
                onboarding_denom: "nhash".to_string(),
                fee_percent: Decimal::percent(20),
                fee_destinations: vec![
                    FeeDestination::new("fee-destination-1", Decimal::percent(75)),
                    FeeDestination::new("fee-destination-2", Decimal::percent(25)),
                ],
                entity_detail: get_valid_entity_detail().to_some(),
            }],
            enabled: true,
        };
        let v2 = definition.to_v2();
        assert_eq!(
            "mortgage", v2.asset_type,
            "expected the asset type to be properly ported",
        );
        assert_eq!(
            "scope-spec-address", v2.scope_spec_address,
            "expected the scope spec address to be properly ported",
        );
        let verifier = assert_single_item(
            &v2.verifiers,
            "expected only a single verifier to be ported",
        );
        assert_eq!(
            "verifier-address", verifier.address,
            "expected the verifier address to be properly ported",
        );
        assert_eq!(
            1000,
            verifier.onboarding_cost.u128(),
            "expected the verifier onboarding cost to be properly ported",
        );
        assert_eq!(
            "nhash", verifier.onboarding_denom,
            "expected the verifier onboarding denom to be properly ported",
        );
        assert_eq!(
            200,
            verifier.fee_amount.u128(),
            "expected the verifier fee amount to be properly derived",
        );
        let fee_destination_1 = assert_single_item_by(
            &verifier.fee_destinations,
            "expected only a single fee destination to be ported with the address 'fee-destination-1'",
            |dest| dest.address == "fee-destination-1",
        );
        assert_eq!(
            150,
            fee_destination_1.fee_amount.u128(),
            "expected the first fee destination's fee amount to be properly derived",
        );
        assert!(
            fee_destination_1.entity_detail.is_none(),
            "expected no entity detail to be populated on the first fee destination",
        );
        let fee_destination_2 = assert_single_item_by(
            &verifier.fee_destinations,
            "expected only a single fee destination to be ported with address 'fee-destination-2'",
            |dest| dest.address == "fee-destination-2",
        );
        assert_eq!(
            50,
            fee_destination_2.fee_amount.u128(),
            "expected the second fee destination's fee amount to be properly derived",
        );
        assert!(
            fee_destination_2.entity_detail.is_none(),
            "expected no entity detail to be populated on the second fee destination",
        );
        assert_eq!(
            get_valid_entity_detail(),
            verifier
                .entity_detail
                .expect("expected the entity detail to be present in the ported value"),
            "expected the entity detail to be properly ported",
        );
    }

    #[test]
    fn test_to_v2_with_multiple_verifiers() {
        let definition = AssetDefinition {
            asset_type: "pl".to_string(),
            scope_spec_address: "scope-spec-address".to_string(),
            verifiers: vec![
                VerifierDetail {
                    address: "verifier-address-1".to_string(),
                    onboarding_cost: Uint128::new(1000),
                    onboarding_denom: "nhash".to_string(),
                    fee_percent: Decimal::percent(20),
                    fee_destinations: vec![
                        FeeDestination::new("fee-destination-1", Decimal::percent(75)),
                        FeeDestination::new("fee-destination-2", Decimal::percent(25)),
                    ],
                    entity_detail: get_valid_entity_detail().to_some(),
                },
                VerifierDetail {
                    address: "verifier-address-2".to_string(),
                    onboarding_cost: Uint128::new(200),
                    onboarding_denom: "noucoin".to_string(),
                    fee_percent: Decimal::percent(2),
                    fee_destinations: vec![FeeDestination::new(
                        "fee-destination",
                        Decimal::percent(100),
                    )],
                    entity_detail: None,
                },
            ],
            enabled: true,
        };
        let v2 = definition.to_v2();
        assert_eq!(
            "pl", v2.asset_type,
            "expected the asset type to be properly ported",
        );
        assert_eq!(
            "scope-spec-address", v2.scope_spec_address,
            "expected the scope spec address to be properly ported",
        );
        let verifier_1 = assert_single_item_by(
            &v2.verifiers,
            "expected only a single verifier to be ported with address 'verifier-address-1'",
            |verifier| verifier.address == "verifier-address-1",
        );
        assert_eq!(
            1000,
            verifier_1.onboarding_cost.u128(),
            "expected the first verifier's onboarding cost to be properly ported",
        );
        assert_eq!(
            "nhash", verifier_1.onboarding_denom,
            "expected the first verifier's onboarding denom to be properly ported",
        );
        assert_eq!(
            200,
            verifier_1.fee_amount.u128(),
            "expected the first verifier's fee amount to be properly derived",
        );
        let fee_destination_1 = assert_single_item_by(
            &verifier_1.fee_destinations,
            "expected only a single fee destination to be ported with the address 'fee-destination-1'",
            |dest| dest.address == "fee-destination-1",
        );
        assert_eq!(
            150,
            fee_destination_1.fee_amount.u128(),
            "expected the first fee destination's fee amount to be properly derived",
        );
        assert!(
            fee_destination_1.entity_detail.is_none(),
            "expected no entity detail to be populated on the first fee destination",
        );
        let fee_destination_2 = assert_single_item_by(
            &verifier_1.fee_destinations,
            "expected only a single fee destination to be ported with address 'fee-destination-2'",
            |dest| dest.address == "fee-destination-2",
        );
        assert_eq!(
            50,
            fee_destination_2.fee_amount.u128(),
            "expected the second fee destination's fee amount to be properly derived",
        );
        assert!(
            fee_destination_2.entity_detail.is_none(),
            "expected no entity detail to be populated on the second fee destination",
        );
        assert_eq!(
            get_valid_entity_detail(),
            verifier_1
                .entity_detail
                .expect("expected the entity detail to be present in the ported value for the first verifier"),
            "expected the entity detail for the first verifier to be properly ported",
        );
        let verifier_2 = assert_single_item_by(
            &v2.verifiers,
            "expected only a single verifier to be ported with address 'verifier-address-2'",
            |verifier| verifier.address == "verifier-address-2",
        );
        assert_eq!(
            200,
            verifier_2.onboarding_cost.u128(),
            "expected the second verifier's onboarding cost to be properly ported",
        );
        assert_eq!(
            "noucoin", verifier_2.onboarding_denom,
            "expected the second verifier's onboarding denom to be properly derived",
        );
        assert_eq!(
            4,
            verifier_2.fee_amount.u128(),
            "expected the second verifier's fee amount to be properly derived",
        );
        let fee_destination = assert_single_item(
            &verifier_2.fee_destinations,
            "expected only a single fee destination to be ported for the second verifier",
        );
        assert_eq!(
            "fee-destination", fee_destination.address,
            "expected the fee destination's address to be properly ported for the second verifier",
        );
        assert_eq!(
            4,
            fee_destination.fee_amount.u128(),
            "expected the fee destination's fee amount to be properly derived",
        );
        assert!(
            fee_destination.entity_detail.is_none(),
            "expected no entity detail to be populated on the second verifier's fee destination",
        );
        assert!(
            verifier_2.entity_detail.is_none(),
            "expected the entity detail on the second verifier to not be present",
        );
    }

    fn get_valid_entity_detail() -> EntityDetail {
        EntityDetail {
            name: "entity-name".to_string().to_some(),
            description: "description".to_string().to_some(),
            home_url: "www.website.websiteother.webstuff.com/whatever"
                .to_string()
                .to_some(),
            source_url: "www.github.com/asset-classification-smart-contract/not-real"
                .to_string()
                .to_some(),
        }
    }
}
