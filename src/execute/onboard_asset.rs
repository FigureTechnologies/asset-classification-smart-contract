use crate::core::asset::AssetIdentifier;
use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::load_asset_definition_by_type;
use crate::service::asset_meta_repository::AssetMetaRepository;
use crate::service::deps_manager::DepsManager;
use crate::service::message_gathering_service::MessageGatheringService;
use crate::util::aliases::{ContractResponse, ContractResult};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};
use provwasm_std::ProvenanceQuerier;

#[derive(Clone, Debug, PartialEq)]
pub struct OnboardAssetV1 {
    pub identifier: AssetIdentifier,
    pub asset_type: String,
    pub validator_address: String,
}
impl OnboardAssetV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<OnboardAssetV1> {
        match msg {
            ExecuteMsg::OnboardAsset {
                identifier,
                asset_type,
                validator_address,
            } => OnboardAssetV1 {
                identifier,
                asset_type,
                validator_address,
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::OnboardAsset".to_string(),
            }
            .to_err(),
        }
    }
}

pub fn onboard_asset<'a, T>(
    repository: T,
    info: MessageInfo,
    msg: OnboardAssetV1,
) -> ContractResponse
where
    T: AssetMetaRepository + MessageGatheringService + DepsManager<'a>,
{
    let asset_identifiers = msg.identifier.to_identifiers()?;
    // get asset definition config for type, or error if not present
    let asset_definition =
        match repository.use_deps(|d| load_asset_definition_by_type(d.storage, &msg.asset_type)) {
            Ok(state) => {
                if !state.enabled {
                    return ContractError::AssetTypeDisabled {
                        asset_type: msg.asset_type,
                    }
                    .to_err();
                }
                state
            }
            Err(_) => {
                return ContractError::UnsupportedAssetType {
                    asset_type: msg.asset_type,
                }
                .to_err()
            }
        };

    // verify perscribed validator is present as a validator in asset definition
    let validator_config = match asset_definition
        .validators
        .into_iter()
        .find(|validator| validator.address == msg.validator_address)
    {
        Some(validator) => validator,
        None => {
            return ContractError::UnsupportedValidator {
                asset_type: msg.asset_type,
                validator_address: msg.validator_address,
            }
            .to_err()
        }
    };

    // verify sent funds match what is specified in the asset state
    if info.funds.len() != 1 {
        return ContractError::InvalidFunds(
            "Exactly one fund type (of nhash) should be sent".to_string(),
        )
        .to_err();
    }

    let sent_fee = match info.funds.iter().find(|funds| funds.denom == "nhash") {
        Some(funds) => funds,
        None => {
            return ContractError::InvalidFunds(format!(
                "Improper funds supplied for onboarding (expected {}nhash)",
                validator_config.onboarding_cost
            ))
            .to_err()
        }
    };

    if sent_fee.amount != validator_config.onboarding_cost {
        return ContractError::InvalidFunds(format!(
            "Improper fee of {}{} provided (expected {}nhash)",
            sent_fee.amount, sent_fee.denom, validator_config.onboarding_cost
        ))
        .to_err();
    };

    // verify asset (scope) exists
    let scope = match repository.use_deps(|d| {
        ProvenanceQuerier::new(&d.querier).get_scope(&asset_identifiers.scope_address)
    }) {
        Err(..) => {
            return ContractError::AssetNotFound {
                scope_address: asset_identifiers.scope_address,
            }
            .to_err()
        }
        Ok(scope) => scope,
    };

    // verify that the sender of this message is a scope owner
    if !scope
        .owners
        .iter()
        .any(|owner| owner.address == info.sender)
    {
        return ContractError::Unauthorized {
            explanation: "sender address does not own the scope".to_string(),
        }
        .to_err();
    }

    // verify asset meta doesn't already contain this asset (i.e. it hasn't already been onboarded)
    if repository.has_asset(&asset_identifiers.scope_address)? {
        return ContractError::AssetAlreadyOnboarded {
            scope_address: asset_identifiers.scope_address,
        }
        .to_err();
    }

    // pull scope records for validation - if no records exist on the scope, the querier will produce an error here
    let records = repository
        .use_deps(|d| ProvenanceQuerier::new(&d.querier).get_records(&scope.scope_id))?
        .records;

    // verify scope has at least one record that is not empty
    if !records.into_iter().any(|record| !record.outputs.is_empty()) {
        return ContractError::InvalidScope {
            explanation: format!(
                "cannot onboard scope [{}]. scope must have at least one non-empty record",
                scope.scope_id,
            ),
        }
        .to_err();
    }

    // store asset metadata in contract storage, with assigned validator and provided fee (in case fee changes between onboarding and validation)
    repository.add_asset(
        &msg.identifier,
        &msg.asset_type,
        &msg.validator_address,
        info.sender,
        crate::core::asset::AssetOnboardingStatus::Pending,
        validator_config,
    )?;

    Ok(Response::new()
        .add_attributes(
            EventAttributes::for_asset_event(
                EventType::OnboardAsset,
                &msg.asset_type,
                &asset_identifiers.scope_address,
            )
            .set_validator(msg.validator_address),
        )
        .add_messages(repository.get_messages()))
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::{from_binary, Coin, CosmosMsg, StdError, SubMsg, Uint128};
    use provwasm_mocks::mock_dependencies;
    use provwasm_std::{
        AttributeMsgParams, Process, ProcessId, ProvenanceMsg, ProvenanceMsgParams, Record, Records,
    };

    use crate::{
        core::{
            asset::{AssetIdentifier, AssetOnboardingStatus, AssetScopeAttribute},
            error::ContractError,
        },
        execute::toggle_asset_definition::{toggle_asset_definition, ToggleAssetDefinitionV1},
        service::asset_meta_service::AssetMetaService,
        testutil::{
            onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset},
            test_constants::{
                DEFAULT_ADMIN_ADDRESS, DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME,
                DEFAULT_ONBOARDING_COST, DEFAULT_RECORD_SPEC_ADDRESS, DEFAULT_SCOPE_ADDRESS,
                DEFAULT_SENDER_ADDRESS, DEFAULT_SESSION_ADDRESS, DEFAULT_VALIDATOR_ADDRESS,
            },
            test_utilities::{
                empty_mock_info, get_default_scope, mock_info_with_funds, mock_info_with_nhash,
                setup_test_suite, test_instantiate_success, InstArgs,
            },
        },
        util::{
            constants::{
                ASSET_EVENT_TYPE_KEY, ASSET_SCOPE_ADDRESS_KEY, ASSET_TYPE_KEY,
                VALIDATOR_ADDRESS_KEY,
            },
            functions::generate_asset_attribute_name,
        },
    };

    use super::{onboard_asset, OnboardAssetV1};

    #[test]
    fn test_onboard_asset_errors_on_unsupported_asset_type() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, 1000),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: "bogus".into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.into(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::UnsupportedAssetType { asset_type } => {
                assert_eq!(
                    "bogus", asset_type,
                    "the unsupported asset type message should reflect the type provided"
                )
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_disabled_asset_type() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        toggle_asset_definition(
            deps.as_mut(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ToggleAssetDefinitionV1::new(DEFAULT_ASSET_TYPE, false),
        )
        .expect("toggling the asset definition to be disabled should succeed");
        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, 1000),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.into(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, ContractError::AssetTypeDisabled { .. }),
            "the request should be rejected for a disabled asset type, but got: {:?}",
            err,
        );
    }

    #[test]
    fn test_onboard_asset_errors_on_unsupported_validator() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, 1000),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string() + "bogus".into(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::UnsupportedValidator {
                asset_type,
                validator_address,
            } => {
                assert_eq!(
                    DEFAULT_ASSET_TYPE, asset_type,
                    "the unsupported validator message should reflect the asset type provided"
                );
                assert_eq!(DEFAULT_VALIDATOR_ADDRESS.to_string() + "bogus".into(), validator_address, "the unsupported validator message should reflect the validator address provided");
            }
            _ => panic!(
                "unexpected error when unsupported validator provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_no_funds() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            empty_mock_info(DEFAULT_SENDER_ADDRESS),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::InvalidFunds(message) => {
                assert_eq!(
                    "Exactly one fund type (of nhash) should be sent", message,
                    "the invalid funds message should reflect invalid amount of funds list"
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_extra_funds() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_funds(
                DEFAULT_SENDER_ADDRESS,
                &[
                    Coin {
                        denom: "nhash".into(),
                        amount: Uint128::from(123u128),
                    },
                    Coin {
                        denom: "otherdenom".into(),
                        amount: Uint128::from(2432u128),
                    },
                ],
            ),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::InvalidFunds(message) => {
                assert_eq!(
                    "Exactly one fund type (of nhash) should be sent", message,
                    "the invalid funds message should reflect invalid amount of funds list"
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_wrong_fund_denom() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_funds(
                DEFAULT_SENDER_ADDRESS,
                &[Coin {
                    denom: "otherdenom".into(),
                    amount: Uint128::from(2432u128),
                }],
            ),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::InvalidFunds(message) => {
                assert_eq!(
                    "Improper funds supplied for onboarding (expected 1000nhash)", message,
                    "the invalid funds message should reflect that improper funds were sent"
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_wrong_fund_amount() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST + 1),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::InvalidFunds(message) => {
                assert_eq!(
                    format!(
                        "Improper fee of {}nhash provided (expected {}nhash)",
                        DEFAULT_ONBOARDING_COST + 1,
                        DEFAULT_ONBOARDING_COST
                    ),
                    message,
                    "the invalid funds message should reflect that improper funds were sent"
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_asset_not_found() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        // Some random scope address unrelated to the default scope address, which is mocked during setup_test_suite
        let bogus_scope_address = "scope1qp9szrgvvpy5ph5fmxrzs2euyltssfc3lu";

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(bogus_scope_address),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::AssetNotFound { scope_address } => {
                assert_eq!(
                    bogus_scope_address,
                    scope_address.as_str(),
                    "the asset not found message should reflect that the asset uuid was not found"
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_already_onboarded_asset() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();

        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();

        match err {
            ContractError::AssetAlreadyOnboarded { scope_address } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS,
                    scope_address,
                    "the asset already onboarded message should reflect that the asset uuid was already onboarded"
                );
            }
            _ => panic!(
                "unexpected error when unsupported asset type provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_onboard_asset_errors_on_no_records() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        // Setup the default scope as the result value of a scope query, but don't establish any records
        deps.querier.with_scope(get_default_scope());
        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();
        match err {
            ContractError::Std(e) => match e {
                StdError::GenericErr { msg, .. } => {
                    assert!(
                        msg.contains("Querier system error"),
                        "the message should denote that the querier failed",
                    );
                    assert!(
                        msg.contains("metadata not found"),
                        "the message should denote that the issue was related to metadata",
                    );
                    assert!(
                        msg.contains("get_records"),
                        "the message should denote that the issue was related to records",
                    );
                },
                _ => panic!("unexpected StdError encountered when onboarding a scope with no records: {:?}", e),
            },
            _ => panic!("expected the provenance querier to return an error when no records are present for the scope, but got error: {:?}", err),
        };
    }

    #[test]
    fn test_onboard_asset_errors_on_empty_records() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        // Setup the default scope and add a record, but make sure the record is not formed properly
        let scope = get_default_scope();
        deps.querier.with_scope(scope.clone());
        deps.querier.with_records(
            scope,
            Records {
                records: vec![Record {
                    name: "record-name".to_string(),
                    session_id: DEFAULT_SESSION_ADDRESS.to_string(),
                    specification_id: DEFAULT_RECORD_SPEC_ADDRESS.to_string(),
                    process: Process {
                        process_id: ProcessId::Address {
                            address: String::new(),
                        },
                        method: String::new(),
                        name: String::new(),
                    },
                    inputs: vec![],
                    outputs: vec![],
                }],
            },
        );
        let err = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, ContractError::InvalidScope { .. }),
            "expected the error to indicate that the scope was invalid for records",
        );
    }

    #[test]
    fn test_onboard_asset_succeeds() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let result = onboard_asset(
            AssetMetaService::new(deps.as_mut()),
            mock_info_with_nhash(DEFAULT_SENDER_ADDRESS, DEFAULT_ONBOARDING_COST),
            OnboardAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                asset_type: DEFAULT_ASSET_TYPE.into(),
                validator_address: DEFAULT_VALIDATOR_ADDRESS.to_string(),
            },
        )
        .unwrap();

        assert_eq!(
            1,
            result.messages.len(),
            "Onboarding should produce only one (bind attribute) message"
        );

        let msg = result.messages.first();

        match msg {
            Some(SubMsg {
                msg:
                    CosmosMsg::Custom(ProvenanceMsg {
                        params:
                            ProvenanceMsgParams::Attribute(AttributeMsgParams::AddAttribute {
                                name,
                                value,
                                ..
                            }),
                        ..
                    }),
                ..
            }) => {
                assert_eq!(
                    &generate_asset_attribute_name(DEFAULT_ASSET_TYPE, DEFAULT_CONTRACT_BASE_NAME),
                    name,
                    "bound asset name should match what is expected for the asset_type"
                );
                let deserialized: AssetScopeAttribute = from_binary(value).unwrap();
                assert_eq!(
                    DEFAULT_ASSET_TYPE.to_string(),
                    deserialized.asset_type,
                    "Asset type in attribute should match what was provided at onboarding"
                );
                assert_eq!(
                    AssetOnboardingStatus::Pending,
                    deserialized.onboarding_status,
                    "Onboarding status should initially be Pending"
                );
            }
            _ => panic!("Unexpected message from onboard_asset: {:?}", msg),
        }

        assert_eq!(
            vec![
                (ASSET_EVENT_TYPE_KEY, "onboard_asset"),
                (ASSET_TYPE_KEY, DEFAULT_ASSET_TYPE),
                (ASSET_SCOPE_ADDRESS_KEY, DEFAULT_SCOPE_ADDRESS),
                (VALIDATOR_ADDRESS_KEY, DEFAULT_VALIDATOR_ADDRESS)
            ],
            result.attributes
        );
    }
}
