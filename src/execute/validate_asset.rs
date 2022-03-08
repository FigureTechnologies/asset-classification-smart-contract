use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{asset_meta_read, config_read};
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::functions::generate_asset_attribute_name;
use crate::util::scope_address_utils::get_validate_scope_address;
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Addr, Env, MessageInfo};
use provwasm_std::ProvenanceQuerier;

#[derive(Clone, PartialEq)]
pub struct ValidateAssetV1 {
    pub scope_address: String,
    pub error: Option<String>,
}
impl ValidateAssetV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<ValidateAssetV1> {
        match msg {
            ExecuteMsg::ValidateAsset {
                asset_uuid,
                scope_address,
                error,
            } => {
                let scope_address = get_validate_scope_address(asset_uuid, scope_address)?;

                ValidateAssetV1 {
                    scope_address,
                    error,
                }
                .to_ok()
            }
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::ValidateAsset".to_string(),
            }
            .to_err(),
        }
    }
}
impl ResultExtensions for ValidateAssetV1 {}

pub fn validate_asset(
    deps: DepsMutC,
    _env: Env,
    info: MessageInfo,
    msg: ValidateAssetV1,
) -> ContractResponse {
    // look up asset in meta storage
    let meta_storage = asset_meta_read(deps.storage);
    let meta = meta_storage
        .load(msg.scope_address.as_bytes())
        .map_err(|_| ContractError::AssetNotFound {
            scope_address: msg.scope_address.clone(),
        })?;

    // verify sender is requested validator
    if info.sender != meta.validator_address {
        return ContractError::UnathorizedAssetValidator {
            scope_address: msg.scope_address,
            validator_address: info.sender.into(),
            expected_validator_address: meta.validator_address,
        }
        .to_err();
    }

    // verify asset not already validated? (fetch attribute?)
    let contract_state = config_read(deps.storage).load()?;
    let attribute_name =
        generate_asset_attribute_name(meta.asset_type, contract_state.base_contract_name);
    let querier = ProvenanceQuerier::new(&deps.querier);
    let existing_attributes =
        querier.get_attributes(Addr::unchecked(msg.scope_address), Some(attribute_name))?;

    if let Some(err) = msg.error {
        if !existing_attributes.attributes.is_empty() {
            // if attribute already exists, check if error already exists
            // existing_attributes
        } else {
            // no existing attribute, construct/set fresh
        }
    } else {
        if !existing_attributes.attributes.is_empty() {
            // check if attribute is successful, else reject
        } else {
            // nothing exists, set attribute fresh
        }
    }

    // construct/emit validation attribute
    ContractError::Unimplemented.to_err()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::mock_env, Uint128};
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::{
            error::ContractError,
            state::{asset_meta, AssetMeta},
        },
        testutil::test_utilities::{
            mock_info_with_nhash, test_instantiate, InstArgs, DEFAULT_ASSET_TYPE,
            DEFAULT_ONBOARDING_COST,
        },
    };

    use super::{validate_asset, ValidateAssetV1};

    #[test]
    fn test_validate_asset_not_found_error() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        let err = validate_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(DEFAULT_ONBOARDING_COST),
            ValidateAssetV1 {
                scope_address: "scope1234".to_string(),
                error: None,
            },
        )
        .unwrap_err();

        match err {
            ContractError::AssetNotFound { scope_address } => {
                assert_eq!(
                    "scope1234", scope_address,
                    "the asset not found message should reflect that the asset uuid was not found"
                );
            }
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }

    #[test]
    fn test_validate_asset_wrong_validator_error() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        asset_meta(&mut deps.storage)
            .save(
                b"scope1234",
                &AssetMeta {
                    scope_address: "scope1234".to_string(),
                    asset_type: DEFAULT_ASSET_TYPE.to_string(),
                    validator_address: "tpcorrectvalidator".to_string(),
                },
            )
            .unwrap();

        let info = mock_info_with_nhash(DEFAULT_ONBOARDING_COST);
        let err = validate_asset(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ValidateAssetV1 {
                scope_address: "scope1234".to_string(),
                error: None,
            },
        )
        .unwrap_err();

        match err {
            ContractError::UnathorizedAssetValidator {
                scope_address,
                validator_address,
                expected_validator_address,
            } => {
                assert_eq!(
                    "scope1234", scope_address,
                    "the unauthorized validator message should reflect the scope address"
                );
                assert_eq!(
                    info.sender.to_string(), validator_address,
                    "the unauthorized validator message should reflect the provided (sender) validator address"
                );
                assert_eq!(
                    "tpcorrectvalidator", expected_validator_address,
                    "the unauthorized validator message should reflect the expected validator address (from onboarding)"
                );
            }
            _ => panic!("unexpected error when unsupported asset type provided"),
        }
    }

    #[test]
    fn test_validate_asset_adds_error_message_on_negative_validation() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate(deps.as_mut(), InstArgs::default()).expect("contract should instantiate");

        let info = mock_info_with_nhash(DEFAULT_ONBOARDING_COST);
        asset_meta(&mut deps.storage)
            .save(
                b"scope1234",
                &AssetMeta {
                    scope_address: "scope1234".to_string(),
                    asset_type: DEFAULT_ASSET_TYPE.to_string(),
                    validator_address: info.sender.to_string(),
                },
            )
            .unwrap();

        let err = validate_asset(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ValidateAssetV1 {
                scope_address: "scope1234".to_string(),
                error: Some("Your data sucks".to_string()),
            },
        )
        .unwrap();

        assert_eq!(true, true);
    }
}
