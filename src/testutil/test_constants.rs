use crate::util::constants::NHASH;

/// All addresses in these test constants were randomly generated for testing purposes
/// This address should be used for the contract administrator address in state
pub const DEFAULT_ADMIN_ADDRESS: &str = "tp1grjeedyfmx0hujsgmqhdr6thjrye4hfesvh2lz";
// DEFAULT_ASSET_UUID is a randomly-generated uuid and the DEFAULT_SCOPE_ADDRESS was generated from it
// They can be expected to convert to each other bidirectionally
pub const DEFAULT_ASSET_UUID: &str = "c55cfe0e-9fed-11ec-8191-0b95c8a1239c";
/// Use this address in a circumstance that is testing a user onboarding and/or interacting with an asset
pub const DEFAULT_SENDER_ADDRESS: &str = "tp1dv7562fvlvf74904t222ze362m036ugtmg45ll";
/// Use this address in a circumstance that is testing an asset definition
pub const DEFAULT_VERIFIER_ADDRESS: &str = "tp1dj50kvzsknr3ydypw3lt8f4dulrrncw4j626vk";
/// Use this address in a circumstance that is testing a fee on verifier detail
pub const DEFAULT_FEE_ADDRESS: &str = "tp1kq5zx7w0x6jvavcay8tutqldync62r29gp8e68";
/// This address should be used when simulating an asset scope attribute or lookup for default onboarding data
pub const DEFAULT_SCOPE_ADDRESS: &str = "scope1qrz4elswnlk3rmypjy9etj9pywwqz6myzw";
/// The default asset definition when using test_instantiate should be expected to be of this type
pub const DEFAULT_ASSET_TYPE: &str = "test_asset";
/// The default asset definition when using test_instantiate should be expected to be of this type
pub const DEFAULT_ASSET_TYPE_DISPLAY_NAME: Option<&str> = Some("Your Favorite Asset");
/// An alternate asset type to use for secondary classification
pub const DEFAULT_SECONDARY_ASSET_TYPE: &str = "test_asset_2";
/// This address should be implicitly be associated with DEFAULT_SCOPE_ADDRESS
pub const DEFAULT_SCOPE_SPEC_ADDRESS: &str = "scopespec1q323khk2jgw5hfada5ukdv3y739ssw53td";
/// This amount directly relates to the amount expected for the default VerifierDetail for onboarding an asset
pub const DEFAULT_ONBOARDING_COST: u128 = 1000;
/// This amount directly relates to the amount expected for the default VerifierDetail for retrying a failed asset
pub const DEFAULT_RETRY_COST: u128 = 500;
/// This amount directly relates to the amount expected for the default VerifierDetail for making a subsequent classification
pub const DEFAULT_SUBSEQUENT_CLASSIFICATION_COST: u128 = 750;
/// This is the default denom expected by the default verifier for onboarding
pub const DEFAULT_ONBOARDING_DENOM: &str = NHASH;
/// This is the default value that test_instantiate uses to create the contract's base name
pub const DEFAULT_CONTRACT_BASE_NAME: &str = "asset";
/// This is the default access route name to be set for the onboarding address
pub const DEFAULT_ACCESS_ROUTE_NAME: &str = "gateway";
/// This is the default access route to be set for the onboarding address
pub const DEFAULT_ACCESS_ROUTE_ROUTE: &str = "https://coolassets.co";
/// This is the default value appended to mocked records
pub const DEFAULT_RECORD_NAME: &str = "test-record";
/// This is the default value used in the default record. The derived result was generated with Provenance's MetadataAddress using the DEFAULT_SCOPE_ADDRESS and a random session UUID
pub const DEFAULT_SESSION_ADDRESS: &str =
    "session1q8z4elswnlk3rmypjy9etj9pyww0amaxfa4xwjj0s7x98k9jyf7a70ngln5";
/// This is the default value used in the default record. The derived result was generated with Provenance's MetadataAddress using the DEFAULT_SCOPE_ADDRESS and the DEFAULT_RECORD_NAME
pub const DEFAULT_RECORD_ADDRESS: &str =
    "record1qtz4elswnlk3rmypjy9etj9pywwzvl7zztch3mmexw06cv32ql2yy93xpz4";
/// This is the default value used in the default record
pub const DEFAULT_RECORD_SPEC_ADDRESS: &str =
    "recspec1qkvaw3xssfcyvmu3s7f4zak4khat2pz8wv2v08m8zgle43ws7u3dscx54v2";
/// This is the default value used in the default record's process
pub const DEFAULT_PROCESS_ADDRESS: &str = "test-process-id";
/// This is the default value used in the default record's process
pub const DEFAULT_PROCESS_METHOD: &str = "testProcess";
/// This is the default value used in the default record's process
pub const DEFAULT_PROCESS_NAME: &str = "test-process-name";
/// This is the default value used in the default record input
pub const DEFAULT_RECORD_INPUT_NAME: &str = "loanType";
/// This is the default value used in the default record input source
pub const DEFAULT_RECORD_INPUT_SOURCE_ADDRESS: &str = "tp1rk3qa624qe504mmvh2nv30zkrtdc5y2455uvew";
/// This is the default value used in the default record output
pub const DEFAULT_RECORD_OUTPUT_HASH: &str = "mock-hash-lkjsdfljsdoinfweounf";
/// This is a default value used in the default verifier detail's entity detail
pub const DEFAULT_ENTITY_DETAIL_NAME: &str = "Provenance Verifier";
/// This is a default value used in the default verifier detail's entity detail
pub const DEFAULT_ENTITY_DETAIL_DESCRIPTION: &str = "Provenance approved verifier";
/// This is a default value used in the default verifier detail's entity detail
pub const DEFAULT_ENTITY_DETAIL_HOME_URL: &str = "http://www.provenance.io/";
/// This is a default value used in the default verifier detail's entity detail (as of writing this, this link is 100% real. Enter if you dare)
pub const DEFAULT_ENTITY_DETAIL_SOURCE_URL: &str = "https://github.com/eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee/eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
