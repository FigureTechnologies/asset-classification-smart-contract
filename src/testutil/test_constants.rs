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
pub const DEFAULT_VALIDATOR_ADDRESS: &str = "tp1dj50kvzsknr3ydypw3lt8f4dulrrncw4j626vk";
/// Use this address in a circumstance that is testing a fee on validator detail
pub const DEFAULT_FEE_ADDRESS: &str = "tp1kq5zx7w0x6jvavcay8tutqldync62r29gp8e68";
/// This address should be used when simulating an asset scope attribute or lookup for default onboarding data
pub const DEFAULT_SCOPE_ADDRESS: &str = "scope1qrz4elswnlk3rmypjy9etj9pywwqz6myzw";
/// The default asset definition when using test_instantiate should be expected to be of this type
pub const DEFAULT_ASSET_TYPE: &str = "test_asset";
/// This address should be implicitly be associated with DEFAULT_SCOPE_ADDRESS
pub const DEFAULT_SCOPE_SPEC_ADDRESS: &str = "scopespec1q323khk2jgw5hfada5ukdv3y739ssw53td";
/// This amount directly relates to the amount expected for the default AssetDefinition for onboarding an asset
pub const DEFAULT_ONBOARDING_COST: u128 = 1000;
/// This is the default denom expected by the default validator for onboarding
pub const DEFAULT_ONBOARDING_DENOM: &str = NHASH;
/// This is the default amount that is taken from the validator to go to fees
pub const DEFAULT_FEE_PERCENT: u64 = 0;
/// This is the default value that test_instantiate uses to create the contract's base name
pub const DEFAULT_CONTRACT_BASE_NAME: &str = "asset";
/// This is the default access route for an asset
pub const DEFAULT_ACCESS_ROUTE: &str = "https://mycoolasset.info";
