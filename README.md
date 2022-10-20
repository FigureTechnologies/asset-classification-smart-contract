# Asset Classification Smart Contract

This contract analyzes Provenance Blockchain [scope](https://docs.provenance.io/modules/metadata-module#scope-data-structures)
metadata to verify that basic portions of newly-generated asset structures are correct, and then allows third parties
to verify the contents of the asset by granting permission to them.

## Status
[![Latest Release][release-badge]][release-latest]
[![Apache 2.0 License][license-badge]][license-url]

[license-badge]: https://img.shields.io/github/license/FigureTechnologies/asset-classification-smart-contract.svg
[license-url]: https://github.com/FigureTechnologies/asset-classification-smart-contract/blob/main/LICENSE
[release-badge]: https://img.shields.io/github/tag/FigureTechnologies/asset-classification-smart-contract.svg
[release-latest]: https://github.com/FigureTechnologies/asset-classification-smart-contract/releases/latest

## Documentation

For more information on how the contract is composed, check out the code [documentation](https://figuretechnologies.github.io/asset-classification-smart-contract/).

## Build

To build the smart contract, simply run the following:
```shell
make optimize
```

## Kotlin Library

[Asset Classification Libs](https://github.com/FigureTechnologies/asset-classification-libs) includes a `client` library
for making requests from a Kotlin project to an instantiated instance of this smart contract.

## Process / Concepts

* __Onboarding__: The concept of onboarding an asset is the primary reason for this contract's functionality.  Onboarding an
asset (contained within a [Provenance Metadata Scope](https://docs.provenance.io/modules/metadata-module#scope-data-structures))
to the contract essentially means that the account that owns the scope has requested that a Verifier process the underlying
data within the scope and mark the scope as a "classified asset," indicating its authenticity as a properly-formed
object that the Provenance Blockchain recognizes.  The process of a successful onboarding will create and store an
[AssetScopeAttribute](src/core/types/asset_scope_attribute.rs) struct value, serialized as JSON in a [Provenance Metadata Attribute](https://docs.provenance.io/modules/account)
on the scope.

* __Verification__: This is the process of downloading or otherwise accessing the underlying data of a Provenance Metadata Scope,
determining that its contents meet the organization's requirements for its specified asset type, and signaling to the
contract instance that verification has completed.  Verification can be completed with a success or failure status,
indicating to external consumers and the contract itself whether or not the scope has been successfully classified as its
requested asset type.  The verification statuses are indicated in the code as an [AssetOnboardingStatus](src/core/types/asset_onboarding_status.rs),
and the most recent verification result is always stored as an [AssetVerificationResult](src/core/types/asset_verification_result.rs)
on the scope's [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs).  On a failed verification, the process can
always be retried, at the cost of paying another onboarding fee.

## Account Roles

The contract interacts with various Provenance Blockchain accounts throughout its various execution processes.  This is
meant to be an exhaustive list of all different types of accounts that may have interactions:

* __Admin Account__: This account is used to store and instantiate the smart contract.  When the contract is instantiated,
the sender address is automatically used as the admin.  This account is required to execute many of the
contract execution endpoints.  This address can later be changed when running a contract migration.

* __Verifier Account__: This account type is used in the contract's [AssetDefinitionV3](src/core/types/asset_definition.rs)'s
[VerifierDetailV2](src/core/types/verifier_detail.rs).  It indicates an account that will inspect the events emitted by
the contract, receive all or a portion of the fees sent to the contract during the onboarding process, and perform
verification of an underlying scope.  Eventually, this account is tasked with calling into the [Verify Asset](src/execute/verify_asset.rs)
execution route to specify whether or not an onboarded scope is valid and therefore verified.

* __Onboarding Account__: This account is not stored in the contract, and can be any Provenance Blockchain address.  It
is the primary consumer of the contract's functionality, and calls into the [Onboard Asset](src/execute/onboard_asset.rs)
execution route, specifying a Verifier Account and paying the fees required by the verifier's [fee destinations](src/core/types/fee_destination.rs).

* __Fee Account__: This account is an optional specification in a [VerifierDetailV2](src/core/types/verifier_detail.rs) and,
when specified, indicates that some or all of the fees provided during the onboarding process should be sent to this address.
The fee account is specified directly in a [FeeDestinationV2](src/core/types/fee_destination.rs), nested within the [VerifierDetailV2](src/core/types/verifier_detail.rs).
There can be multiple Fee Accounts for a single Verifier Account, ensuring that any amount of fee division can occur.

## Contract Interaction

### [Instantiation](src/instantiate/init_contract.rs)

Instantiating a smart contract is a core portion of the contract creation process.  Instantiating the asset classification
smart contract utilizes a standard [InitMsg](src/core/msg.rs).  For a json breakdown, check out the [InitMsg Schema](schema/init_msg.json).
When the contract is instantiated, it does the following actions:

* Optionally binds a [Provenance Blockchain Name Module](https://docs.provenance.io/modules/name-module) name to itself
based on an input value.  This can be omitted if the contract does not want to actually bind its root name.  This
circumstance can arise if the root name of the contract already exists, or if it is restricted.

* Optionally establishes an initial set of [AssetDefinitionV3](src/core/types/asset_definition.rs) values if any are
provided, and binds their names, also optionally.  The names bound will be their asset types, branched from the contract's
base name value.

* Constructs an initial contract state, stored internally as a [StateV2](src/core/state.rs) value.  This takes the sender
address from instantiation and uses it as the contract's admin address, initially.  This admin value can be changed
later during a contract migration.

* Establishes contract version information based on the value of [Cargo.toml](Cargo.toml)'s version property.

#### Request Parameters

* `base_contract_name`: This name serves as the basis for all generated [Provenance Attributes](https://docs.provenance.io/modules/account).
All [AssetDefinitionV3](src/core/types/asset_definition.rs) names established will use this name as the root value.
For instance, if the `base_contract_name` is `testasset` and an asset definition's `asset_type` is specified as `donut`,
then the attribute name used for created [AssetScopeAttributes](src/core/types/asset_scope_attribute.rs) will be
`donut.testasset`.

* `bind_base_name`: If set to `true`, the contract will try to bind the provided name to itself.  This will fail if the
provided name uses a restricted root name, so using a value of `false` can circumvent this issue and the name can be
bound later or potentially not at all.  The contract needs to own the subnames used for generated attributes in order
for its bindings to work, but owning the root name is not necessary.

* `asset_definitions`: An array of [AssetDefinitionInputV3](src/core/types/asset_definition.rs) values that will be used
to establish an initial set of [AssetDefinitionV3](src/core/types/asset_definition.rs)s in the contract's internal storage.
These definitions will automatically attempt to bind their own names, branching from the `base_contract_name`, but the
[AssetDefinitionInputV3](src/core/types/asset_definition.rs) includes a `bind_name` boolean that allows this functionality
to be disabled if that behavior is not desired.

* `is_test`: A boolean value allowing for less restrictions to be placed on certain functionalities across the contract's
execution processes.  Notably, this disables a check during the onboarding process to determine if onboarded scopes include
underlying record values.  This should never be set to true in a mainnet environment.

#### Emitted Attributes
* `asset_event_type`: This value will always be populated as `instantiate_contract`.

#### Request Sample
```json
{
  "base_contract_name": "testasset.pb",
  "bind_base_name": true,
  "asset_definitions": [
    {
      "asset_type": "cat",

      "verifiers": [
        {
          "address": "tp14w3jf4em4uszs77yaqnmfrlxwcmqux5g6hfpdf",
          "onboarding_cost": "150",
          "onboarding_denom": "nhash",
          "fee_destinations": [
            {
              "address": "tp1u7r46zkgcmvel59tqa9352k5rycl985ywqnjp7",
              "fee_amount": "75",
              "entity_detail": {
                "name": "Cat Auxiliary Fund Source",
                "description": "Extra fees for extra cute cats"
              }
            }
          ],
          "entity_detail": {
            "name": "Cat Verifier",
            "description": "Ensures that your cats are adorable and have all of their legs",
            "home_url": "https://www.catsareadorable.gov/itstrue",
            "source_url": "https://www.github/mycatorganization/cat-verifier"
          },
          "retry_cost": {
            "onboarding_cost": "20",
            "fee_destinations": [
              {
                "address": "tp1u7r46zkgcmvel59tqa9352k5rycl985ywqnjp7",
                "fee_amount": "1",
                "entity_detail": {
                  "name": "Cat Auxiliary Fund Source",
                  "description": "Extra fees for extra cute cats"
                }
              }
            ]
          },
          "subsequent_classification_detail": {
            "cost": {
              "onboarding_cost": "70",
              "fee_destinations": [
                {
                  "address": "tp1u7r46zkgcmvel59tqa9352k5rycl985ywqnjp7",
                  "fee_amount": "10",
                  "entity_detail": {
                    "name": "Cat Auxiliary Fund Source",
                    "description": "Extra fees for extra cute cats"
                  }
                }
              ]
            },
            "applicable_asset_types": ["pet", "lovable"]
          }
        }
      ],
      "enabled": true,
      "bind_name": false
    }
  ],
  "is_test": false
}
```

### [Migration](src/migrate/migrate_contract.rs)

The contract maintains a very basic migration strategy.  Migrating the asset classification smart contract utilizes a
standard [MigrationMsg](src/core/msg.rs) enum to determine the processes executed.  This allows for a flexible design
structure if nonstandard migrations need to be added in the future.  This section will cover the basic migration pathway.
When a contract migration is run, the following actions are taken:

* The version of the existing instance of the contract is checked against the new contract code's version using a semver
comparison to ensure that the migration will only run if the new code has a version equal to or greater than the existing
contract's version.  Lower versions are rejected outright and the migration will fail.

* The contract's internal versioning storage is updated to reflect the new contract code's version.

* If any options are provided in the message's [MigrationOptions](src/core/msg.rs), their specific actions are executed.

#### Request Parameters
* `options`: An instance of [MigrationOptions](src/core/msg.rs) that dictates additional steps to perform during the
migration.  Each option and its behavior is as follows:
  * `new_admin_address`: If provided as a valid bech32 address, the contract's internal admin account will be changed to
      match this value.

#### Emitted Attributes
* `asset_event_type`: This value will always be populated as `migrate_contract`.

* `asset_new_value`: This value will always match the version property of the [Cargo.toml](Cargo.toml) in the build used to store
the wasm bytecode for the new contract instance.

* `asset_additional_metadata`: If any values were provided as [MigrationOptions](src/core/msg.rs), they will be included
in this attribute using a key/value system.  If no options were provided, this attribute will be omitted.

#### Request Sample With Options
```json
{
  "contract_upgrade": {
    "options": {
      "new_admin_address": "tp1ps3750ga04lp3yw3n3uydm2sw6rn832wszcpkz"
    }
  }
}
```

#### Request Sample Without Options
```json
{
  "contract_upgrade": {}
}
```

### [Execution Routes](src/execute)

The contract exposes various execute routes by which interaction is possible.  All execution route enum variants are
defined in the [ExecuteMsg Enum](src/core/msg.rs).   The json schema for sending a contract execution transaction message
is defined in the [Execute Schema Json](schema/execute_msg.json).

#### [Onboard Asset](src/execute/onboard_asset.rs)

This route is the primary interaction point for most consumers.  It consumes an asset uuid or scope address, the type of
asset corresponding to that scope (heloc, mortgage, payable, etc), and, if all checks pass, attaches an attribute to the
provided scope that stages the scope for verification of authenticity by the specified verifier in the request.  The
attribute is attached based on the `base_contract_name` specified in the contract, combined with the specified asset type
in the request.  Ex: if `base_contract_name` is "asset" and the asset type is "myasset", the attribute would be assigned
under the name of "myasset.asset".  All available asset types are queryable, and stored in the contract as [AssetDefinitionV3](src/core/types/asset_definition.rs)
values.  After onboarding is completed, an [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs) will be
stored on the scope with an [AssetOnboardingStatus](src/core/types/asset_onboarding_status.rs) of `Pending`, indicating
that the asset has been onboarded to the contract but is awaiting verification.

Note: The account that invokes the `OnboardAsset` execution route must be the owner of the scope referenced in the
request.

##### Request Parameters

* `identifier`: A serialized version of an [AssetIdentifier](src/core/types/asset_identifier.rs) enum.  Indicates the
scope to onboard.  The following json is an example of what this might look like in a request:
```json
{"identifier": {"type": "asset_uuid", "value": "8f9cea0a-d6e7-11ec-be71-dbbe1d4d92be"}}
```
OR
```json
{"identifier": {"type": "scope_address", "value": "scope1qzj8tjp76mn3rmyvz49c5738k2asm824ga"}}
```

* `asset_type`: A name that must directly match one of the contract's internal [AssetDefinitionV3](src/core/types/asset_definition.rs)
names.  Any request with a specified type not matching an asset definition will be rejected outright.

* `verifier_address`: The bech32 address of a Verifier Account associated with the targeted [AssetDefinitionV3](src/core/types/asset_definition.rs),
within its nested vector of [VerifierDetailV2](src/core/types/verifier_detail.rs)s.

* `access_routes`: An optional parameter that allows the specification of a location to get the underlying asset data
for the specified scope.  The [AccessRoute](src/core/types/access_route.rs) struct is very generic in its composition
for the purpose of allowing various different solutions to fetching asset data.  If the verification process requires
generic lookups for each onboarded asset, access routes on the scope's [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs)
can be leveraged to easily determine the source of the underlying data.  If these values are omitted at first, but later needed,
they can always be added by using the `UpdateAccessRoutes` execution route.  Note: Access routes can specify a `name`
parameter, as well, to indicate the reason for the route, but this is entirely optional.

* `add_os_gateway_permission`: An optional parameter that will cause the emitted events to include values that signal
to any [Object Store Gateway](https://github.com/FigureTechnologies/object-store-gateway) watching the events that the
selected verifier has permission to inspect the identified scope's records via fetch routes. This will only cause a
gateway to grant permissions to a scope to which the gateway itself already has read permissions.  This essentially
means that a key held by a gateway instance must have been used to store the scope's records in [Provenance Object Store](https://github.com/provenance-io/object-store).
This behavior defaults to TRUE if not explicitly provided in the json payload.

##### Emitted Attributes
* `asset_event_type`: This value will always be populated as `onboard_asset`.

* `asset_type`: This value will correspond to the value of the `asset_type` parameter passed into the request.

* `asset_scope_address`: This value will be the bech32 address of the scope used during onboarding.

* `asset_verifier_address`: This value will be the bech32 address included in the `verifier_address` parameter of this
execution route.

* `asset_scope_owner_address`: This value will be the bech32 address of the owner of the scope processed in the request.
As the request will be rejected unless it is made by the scope owner, this address should match the sender of the message
as well.

* `object_store_gateway_event_type`: This value is only emitted when `add_os_gateway_permission` is omitted or explicitly
specified as `true`.  It will always have a value of `access_grant` and indicates to the Object Store Gateway that the
verifier should receive permissions to inspect the records included in the scope referred to by `asset_scope_address`.

* `object_store_gateway_scope_address`: This value is only emitted when `add_os_gateway_permission` is omitted or
explicitly specified as `true`.  It will always have the same value as `asset_scope_address`, and indicates the bech32
scope identifier to use for access grants.

* `object_store_gateway_target_account_address`: This value is only emitted when `add_os_gateway_permission` is omitted
or explicitly specified as `true`.  It will always have the same value as `asset_verifier_address`, and indicates the
bech32 account identifier of the verifier, ensuring that the verifier receives a grant to inspect scope records.

* `object_store_gateway_access_grant_id`: This value is only emitted when `add_os_gateway_permission` is omitted or
explicitly specified as `true`.  It is a concatenation of the `asset_type` and `asset_scope_address` values, creating
a unique identifier for an asset's verification.  This allows multiple asset type verifications to occur for the same
scope address, working in tandem with the fact that the `verify_asset` functionality will revoke access grants from the
verifier based on the same grant id as they are processed.  This will ensure that the verifier can only inspect scope
data for as long as the verification process is active.

##### Request Sample
```json
{
  "onboard_asset": {
    "identifier": {
      "type": "asset_uuid",
      "value": "417556d2-d6ec-11ec-88d8-8be6d7728b01"
    },
    "asset_type": "payable",
    "verifier_address": "tp1v5j3mlmkdyfyjuwp4ux7066s7knjzaq30f3re0",
    "access_routes": [
      {
        "route": "https://www.mycoolaccessserver.website/api/download-asset/417556d2-d6ec-11ec-88d8-8be6d7728b01",
        "name": "Web Api"
      },
      {
        "route": "grpc://mycoolgrpcserver.website",
        "name": "GRPC Access"
      }
    ],
    "add_os_gateway_permission": false
  }
}
```

#### [Verify Asset](src/execute/verify_asset.rs)

This route is specifically designed to allow a Verifier specified in the [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs)
of a [Provenance Metadata Scope](https://docs.provenance.io/modules/metadata-module#scope-data-structures) to indicate to
the owner of the scope whether or not the content within the scope was valid or not.  The Verifier Account, after determining
validity of the underlying data, will either mark the classification as a success or failure.  This route will reject
all invokers except for Verifiers linked to a scope by the scope attribute, ensuring that only the verifier requested
has the permission needed to classify an asset.  In this way, the process for verification ensures that all involved
parties' requirements for security are satisfied.  In addition, the verifier used in the process is stored on the scope
attribute after the fact, ensuring that external inspectors of the generated attribute can choose which verifications to
acknowledge and which to disregard.

It is important to note that this route emits event attributes automatically that are interpreted by
[Object Store Gateway](https://github.com/FigureTechnologies/object-store-gateway).  However, if the values indicate to
the gateway that it should remove a permission that was never at first created, then the event will be ignored and take
no negative actions.  In order to avoid the contract triggering an impact in the gateway, simply provide a value of
`"add_os_gateway_permission": false` when using the `onboard_asset` route.

##### Request Parameters

* `identifier`: A serialized version of an [AssetIdentifier](src/core/types/asset_identifier.rs) enum.  Indicates the
scope being verified.  The following json is an example of what this might look like in a request:
```json
{"identifier": {"type": "asset_uuid", "value": "8f9cea0a-d6e7-11ec-be71-dbbe1d4d92be"}}
```
OR
```json
{"identifier": {"type": "scope_address", "value": "scope1qzj8tjp76mn3rmyvz49c5738k2asm824ga"}}
```

* `success`: A boolean indicating whether or not verification was successful.  A value of `false` either indicates that
the underlying data was fetched and it did not meet the requirements for a classified asset, or that a failure occurred
during the verification process.  Note: Verifiers should be wary of returning false immediately on a code failure, as
this incurs additional cost to the onboarding account.  Instead, it is recommended that verification implement some
process that retries logic when exceptions or other code execution issues cause a failed verification.  A value of `true`
will move the [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs) to the `Approved` status, which indicates a classified asset.

* `message`: An optional string describing the result of the verification process.  If omitted, a standard message
describing success or failure based on the value of `success` will be displayed in the [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs).

* `access_routes`: Like in the `OnboardAsset` message, this parameter allows the verifier to provide access routes for
the assets that it has successfully fetched from the underlying scope data.  This allows for the verifier to define its
own subset of [AccessRoute](src/core/types/access_route.rs) values to allow actors with permission to easily fetch asset
data from a new location, potentially without any Provenance Blockchain interaction, facilitating the process of data
interaction.

##### Emitted Attributes
* `asset_event_type`: This value will always be populated as `verify_asset`.

* `asset_type`: This value will correspond to `asset_type` parameter stored in the [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs)
attached to the scope that was previously onboarded before verification.

* `asset_scope_address`: This value will be the bech32 address of the scope modified during verification.

* `asset_verifier_address`: This value will be the bech32 address of the verifier invoking the execution route.

* `object_store_gateway_event_type`: This value will always have a value of `access_revoke` and indicates to the Object
Store Gateway that the verifier should have its permissions to inspect the records included in the scope referred to by
`asset_scope_address` removed.

* `object_store_gateway_scope_address`: This value will always have the same value as `asset_scope_address`, and
indicates the bech32 scope identifier to target an existing access grant.

* `object_store_gateway_target_account_address`: It will always have the same value as `asset_verifier_address`, and
indicates the bech32 account identifier of the verifier, ensuring that the verifier has its grant to inspect scope
records revoked.

* `object_store_gateway_access_grant_id`: It is a concatenation of the `asset_type` and `asset_scope_address` values,
creating a unique identifier for an asset's verification.  This allows multiple asset type verifications to occur for
the same scope address, working in tandem with the fact that the `verify_asset` functionality will revoke access grants
from the verifier based on the same grant id as they are processed.  This will ensure that the verifier can only inspect
scope data for as long as the verification process is active.

##### Request Sample
```json
{
  "verify_asset": {
    "identifier": {
      "type": "asset_uuid",
      "value": "417556d2-d6ec-11ec-88d8-8be6d7728b01"
    },
    "success": "true",
    "message": "Verification completed successfully after downloading Payable Asset and inspecting its data",
    "access_routes": [
      {
        "route": "https://www.myverifierhost.verifier/api/v2/asset/417556d2-d6ec-11ec-88d8-8be6d7728b01"
      }
    ],
    "remove_os_gateway_permission": false
  }
}
```

#### [Add Asset Definition](src/execute/add_asset_definition.rs)

__This route is only accessible to the contract's admin address.__  This route allows a new [AssetDefinitionV3](src/core/types/asset_definition.rs)
value to be added to the contract's internal storage.  These asset definitions dictate which asset types are allowed to
be onboarded, as well as which verifiers are tied to each asset type.  Each added asset definition must be unique in
two criteria:
* Its `asset_type` value must not yet be registered in a different asset definition.

##### Request Parameters

* `asset_definition`: An [AssetDefinitionInputV3](src/core/types/asset_definition.rs) value defining all of the new
[AssetDefinitionV3](src/core/types/asset_definition.rs)'s values.  The execution route converts the incoming value to an
asset definition.

##### Emitted Attributes
* `asset_event_type`: This value will always be populated as `add_asset_definition`.

* `asset_type`: This value will be the `asset_type` value stored in the added [AssetDefinitionV3](src/core/types/asset_definition.rs).

##### Request Sample
```json
{
  "add_asset_definition": {
    "asset_definition": {
      "asset_type": "car",
      "verifiers": [
        {
          "address": "tp1eqfg2lxlwqs23m320arhuk3dad47ha6tvzu5n5",
          "onboarding_cost": "1000000000",
          "onboarding_denom:": "nhash",
          "fee_destinations": [
            {
              "address": "tp1s735l5tmh7sngyvvn6rf4l7e9e9qq8uz93z9ky",
              "fee_amount": "500000000",
              "entity_detail": {
                "name": "Car Detailer",
                "descriptions": "Details the cars and charges a fee to do it"
              }
            },
            {
              "address": "tp1k5m8lkshpupuf44p02r4mwgjdawd2kta4n9ry0",
              "fee_amount": "500000000"
            }
          ],
          "entity_detail": {
            "name": "My Company's Verifier",
            "description": "Verifies car assets better than any other car verifier you can think of",
            "home_url": "https://www.mycompany.wesellcars/info",
            "source_url": "https://www.github.com/CarCompanyCodePlace/car-verifier-application"
          },
          "retry_cost": {
            "onboarding_cost": "1000000",
            "fee_destinations": [
              {
                "address": "tp1s735l5tmh7sngyvvn6rf4l7e9e9qq8uz93z9ky",
                "fee_amount": "5000",
                "entity_detail": {
                  "name": "Car Detailer",
                  "descriptions": "Details the cars and charges a fee to do it"
                }
              }
            ]
          },
          "subsequent_classification_detail": {
            "cost": {
              "onboarding_cost": "3000000",
              "fee_destinations": [
                {
                  "address": "tp1s735l5tmh7sngyvvn6rf4l7e9e9qq8uz93z9ky",
                  "fee_amount": "1000",
                  "entity_detail": {
                    "name": "Car Detailer",
                    "descriptions": "Details the cars and charges a fee to do it"
                  }
                }
              ]
            },
            "applicable_asset_types": ["vehicle", "owned"]
          }
        }
      ],
      "enabled": true,
      "bind_name": true
    }
  }
}
```

#### [Update Asset Definition](src/execute/update_asset_definition.rs)
__This route is only accessible to the contract's admin address.__ This route allows an existing [AssetDefinitionV3](src/core/types/asset_definition.rs)
value to be updated.  It works by matching the input's `asset_type` to an existing asset definition and overwriting the
existing values.  If no asset definition exists for the given type, the request will be rejected.

##### Request Parameters

* `asset_definition`: An [AssetDefinitionInputV3](src/core/types/asset_definition.rs) value defining all of the updated
  [AssetDefinitionV3](src/core/types/asset_definition.rs)'s values.  The execution route converts the incoming value to an
  asset definition.

##### Emitted Attributes
* `asset_event_type`: This value will always be populated as `update_asset_definition`.

* `asset_type`: This value will be the `asset_type` value stored in the updated [AssetDefinitionV3](src/core/types/asset_definition.rs).

##### Request Sample
```json
{
  "update_asset_definition": {
    "asset_definition": {
      "asset_type": "car",
      "verifiers": [
        {
          "address": "tp1uvnpfg9hmeyuf0t3a6l9xhegx8ewhtk9z683x4",
          "onboarding_cost": "15000000000",
          "onboarding_denom:": "carcoins",
          "fee_destinations": [
            {
              "address": "tp1s735l5tmh7sngyvvn6rf4l7e9e9qq8uz93z9ky",
              "fee_amount": "20000",
              "entity_detail": {
                "name": "Car Theft Prevention Services",
                "description": "Installs loud alarms in the cars to make sure if people steal them, everyone knows"
              }
            }
          ],
          "entity_detail": {
            "name": "My Company's Verifier",
            "description": "Verifies car assets better than any other car verifier you can even FATHOM",
            "home_url": "https://www.mycompany.wesellcars/info",
            "source_url": "https://www.github.com/CarCompanyCodePlace/car-verifier-application"
          },
          "retry_cost": {
            "onboarding_cost": "1000000",
            "fee_destinations": [
              {
                "address": "tp1s735l5tmh7sngyvvn6rf4l7e9e9qq8uz93z9ky",
                "fee_amount": "5000",
                "entity_detail": {
                  "name": "Car Theft Prevention Services",
                  "description": "Installs loud alarms in the cars to make sure if people steal them, everyone knows"
                }
              }
            ]
          },
          "subsequent_classification_detail": {
            "cost": {
              "onboarding_cost": "3000000",
              "fee_destinations": [
                {
                  "address": "tp1s735l5tmh7sngyvvn6rf4l7e9e9qq8uz93z9ky",
                  "fee_amount": "1000",
                  "entity_detail": {
                    "name": "Car Theft Prevention Services",
                    "description": "Installs loud alarms in the cars to make sure if people steal them, everyone knows"
                  }
                }
              ]
            },
            "applicable_asset_types": ["vehicle", "owned"]
          }
        }
      ],
      "enabled": true
    }
  }
}
```

#### [Toggle Asset Definition](src/execute/toggle_asset_definition.rs)
__This route is only accessible to the contract's admin address.__ This route toggles an existing [AssetDefinitionV3](src/core/types/asset_definition.rs)
from enabled to disabled, or disabled to enabled.  When disabled, an asset definition will no longer allow new assets to
be onboarded to the contract.  Existing assets already onboarded to the contract and in pending status will still be
allowed to be verified, but new values will be rejected.  This same functionality could be achieved with an invocation of
the `UpdateAssetDefinition` route but swapping the `enabled` value on the `asset_definition` parameter, but this route
is significantly simpler and prevents accidental data mutation due to it not requiring the entirety of the definition's
values.

##### Request Parameters

* `asset_type`: The type of asset for which the definition's `enabled` value will be toggled.  As the asset type value
on each asset definition is guaranteed to be unique, this key is all that is needed to find the target definition.

* `expected_result`: The value of `enabled` after the toggle takes place.  This value is required to ensure that
multiple toggles executed in succession (either by accident or by various unrelated callers) will only be honored if
the asset definition is in the intended state during the execution of the route.

##### Emitted Attributes
* `asset_event_type`: This value will always be populated as `toggle_asset_definition`.

* `asset_type`: This value will be the `asset_type` value stored in the modified [AssetDefinitionV3](src/core/types/asset_definition.rs).

* `asset_new_value`: This value will be the new status of the asset definition's `enabled` property, after the toggle occurs (true/false).

##### Request Sample
```json
{
  "toggle_asset_definition": {
    "asset_type": "airplane",
    "expected_result": false
  }
}
```

#### [Delete Asset Definition](src/execute/delete_asset_definition.rs)
__This route is only accessible to the contract's admin address.__  This route facilitates the removal of bad data.

__IMPORTANT__: If an asset definition is completely removed, all contract references to it will fail to function.  This
can cause assets currently in the onboarding process for a deleted type to have failures when interactions occur with
them.  This functionality should only be used for an unused type!

##### Request Parameters

* `asset_type`: The asset type to delete.

##### Emitted Attributes
* `asset_event_type`: This value will always be populated as `delete_asset_definition`.

* `asset_type`: This value will be populated with the [asset_type](src/core/types/asset_definition.rs) property of the
deleted [asset definition](src/core/types/asset_definition.rs).

##### Request Sample
```json
{
  "delete_asset_definition": {
    "asset_type": "widget"
  }
}
```

#### [Add Asset Verifier](src/execute/add_asset_verifier.rs)
__This route is only accessible to the contract's admin address.__ This route adds a new [VerifierDetailV2](src/core/types/verifier_detail.rs)
to an existing [AssetDefinitionV3](src/core/types/asset_definition.rs).  This route is intended to register new verifiers
without the bulky requirements of the `UpdateAssetDefinition` execution route.  This route will reject verifiers added
with addresses that match any other verifiers on the target asset definition.

##### Request Parameters

* `asset_type`: The type of asset for which the new [VerifierDetailV2](src/core/types/verifier_detail.rs) will be added.
This must refer to an existing [AssetDefinitionV3](src/core/types/asset_definition.rs)'s `asset_type` value, or the request
will be rejected.

* `verifier`: The new [VerifierDetailV2](src/core/types/verifier_detail.rs) to be added to the asset definition, with all
of its required values.  No verifiers within the existing asset definition must have the same `address` value of this
parameter, or the request will be rejected.

##### Emitted Attributes
* `asset_event_type`: This value will always be populated as `add_asset_verifier`.

* `asset_type`: This value will be the `asset_type` value stored in the modified [AssetDefinitionV3](src/core/types/asset_definition.rs).

* `asset_verifier_address`: This value will be the bech32 address stored in the `address` property of the new [VerifierDetailV2](src/core/types/verifier_detail.rs).

##### Request Sample
```json
{
  "add_asset_verifier": {
    "asset_type": "train",
    "verifier": {
      "address": "tp149832nekuva7lxtzcezlwhzqjc32vu8lsaqcuv",
      "onboarding_cost": "250",
      "onboarding_denom": "traintoken",
      "fee_destinations": [
        {
          "address": "tp1lmp2qmntl090whuftym0wthzwjc8xv0v79cu6l",
          "fee_amount": "250",
          "entity_detail": {
            "name": "Conductor Fee Collector",
            "description": "Train conductors need to eat, too!",
            "home_url": "www.trainconductorshomesite.gov/how-do-conductors-make-money-from-the-blockchain"
          }
        }
      ],
      "entity_detail": {
        "name": "Train Verifier",
        "description": "Verifies that trains can successfully fit on the tracks",
        "home_url": "https://www.trainsrus.edu.gov.com.org.eduagain",
        "source_url": "https://github.com/samuelmarina/is-even"
      },

    }
  }
}
```

#### [Update Asset Verifier](src/execute/update_asset_verifier.rs)
__This route is only accessible to the contract's admin address or the address of the verifier being updated.__ This route updates an existing [VerifierDetailV2](src/core/types/verifier_detail.rs)
in an existing [AssetDefinitionV3](src/core/types/asset_definition.rs).  This route is intended to be used when the values
of a single verifier detail need to change, but not the entire asset definition.  The request will be rejected if the
referenced asset definition is not present within the contract, or if a verifier does not exist within the asset
definition that matches the address of the provided verifier data.

##### Request Parameters

* `asset_type`: The type of asset for which the [VerifierDetailV2](src/core/types/verifier_detail.rs) will be updated. This
must refer to an existing [AssetDefinitionV3](src/core/types/asset_definition.rs)'s `asset_type` value, or the request
will be rejected.

* `verifier`: The updated [VerifierDetailV2](src/core/types/verifier_detail.rs) to be modified in the asset definition.
An existing verifier detail within the target asset definition must have a matching `address` value, or the request will
be rejected.

##### Emitted Attributes
* `asset_event_type`: This value will always be populated as `update_asset_verifier`.

* `asset_type`: This value will be the `asset_type` value stored in the modified [AssetDefinitionV3](src/core/types/asset_definition.rs).

* `asset_verifier_address`: This value will be the bech32 address stored in the `address` property of the updated [VerifierDetailV2](src/core/types/verifier_detail.rs).

##### Request Sample
```json
{
  "update_asset_verifier": {
    "asset_type": "widget",
    "verifier": {
      "address": "tp15n6as7tytrza9692anawwc52kyg5pv86lpeyhu",
      "onboarding_cost": "200",
      "onboarding_denom": "widgetdollar",
      "fee_destinations": [],
      "entity_detail": {
        "name": "Widget Verifier Inc.",
        "description": "We do generic verification of things and have no clear reason for what we do",
        "home_url": "https://www.widgetwebsite.squirrel/horse/rabbit.jpg",
        "source_url": "https://www.github.com/widgetsquirrels/horse-rabbit-project"
      }
    }
  }
}
```

#### [Update Access Routes](src/execute/update_access_routes.rs)
__This route is only accessible to the contract's admin address OR to the owner of the access routes being updated.__
This route will swap all existing access routes for a specific owner for a specific scope to the provided values. These
access routes either correspond to those created during the onboarding process, or those created during the verification
process.

##### Request Parameters

* `identifier`: A serialized version of an [AssetIdentifier](src/core/types/asset_identifier.rs) enum.  Indicates the
scope to have its access routes swapped.  The following json is an example of what this might look like in a request:
```json
{"identifier": {"type": "asset_uuid", "value": "8f9cea0a-d6e7-11ec-be71-dbbe1d4d92be"}}
```
OR
```json
{"identifier": {"type": "scope_address", "value": "scope1qzj8tjp76mn3rmyvz49c5738k2asm824ga"}}
```
* `owner_address`: Corresponds to the bech32 address of the account that originally created the [AccessRoute](src/core/types/access_route.rs)s.
These values can be found in the [AccessDefinition](src/core/types/access_definition.rs) of the [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs)
tied to a scope after the onboarding process occurs.

* `access_routes`: An array of [AccessRoute](src/core/types/access_route.rs) to be used instead of the existing routes.
If other existing routes need to be maintained and the updated is intended to simply add a new route, then the existing
routes need to be included in the request alongside the new route(s).

##### Emitted Attributes
* `asset_event_type`: This value will always be populated as `update_access_routes`.

* `asset_type`: This value will be the `asset_type` value stored in the modified [AssetDefinitionV3](src/core/types/asset_definition.rs).

* `asset_scope_address`: This value will be the bech32 address of the [Provenance Blockchain Metadata Scope](https://docs.provenance.io/modules/metadata-module#metadata-scope)
referred to by the `identifier` parameter passed into the execution message.

##### Request Sample
```json
{
  "update_access_routes": {
    "identifier": {
      "type": "asset_uuid",
      "value": "93ad940c-d6f9-11ec-91fd-af096c6cf471"
    },
    "owner_address": "tp1mpa626v8kntgpweespkyf4vfnvsj73ejwalec2",
    "access_routes": [
      {
        "route": "https://www.mywebsite.websiteplace/api/asset/request/too/many/slashes/93ad940c-d6f9-11ec-91fd-af096c6cf471",
        "name": "Unnecessarily Long URL"
      }
    ]
  }
}
```

### [Query Routes](src/query)

The contract exposes various query routes by which data retrieval is possible.  All query route enum variants are
defined in the [QueryMsg Enum](src/core/msg.rs).  The json schema for sending a contract query message is defined in the
[Query Schema Json](schema/query_msg.json).

#### [Query Asset Definition](src/query/query_asset_definition.rs)

This route can be used to retrieve a specific [AssetDefinitionV3](src/core/types/asset_definition.rs) from the contract's
internal storage for inspection of its verifies and other properties.  If the requested value is not found, a null
response will be returned.

##### Request Parameters

* `asset_type`: The asset type to fetch.

##### Request Sample
```json
{
  "query_asset_definition": {
    "asset_type": "dog"
  }
}
```

##### Response Sample
```json
{
  "data": {
    "asset_type": "dog",
    "verifiers": [
      {
        "address": "tp1935mawrmyuzwuryg8wya3g6uh2vpwvapq50kvq",
        "onboarding_cost": "1000000000",
        "onboarding_denom": "nhash",
        "fee_destinations": [
          {
            "address": "tp126lrty2c0h78mdtjyzzf7mtsge427trccq8lta",
            "fee_amount": "1000",
            "entity_detail": {
              "name": "Totally Not a Dog Collecting Fees",
              "description": "Bark! Bark! Uh... I mean, I'm an administrator of the website and totally a human",
              "home_url": "https://www.totallynotadog.truth/i-am-not-a-dog"
            }
          },
          {
            "address": "tp1gkg4a8zrtadlz5c5u56fuhcf5gq3xy4v8pgaef",
            "fee_amount": "234",
            "entity_detail": {
              "description": "All the fields for entity detail are optional, so this one only provides a description!"
            }
          }
        ],
        "entity_detail": {
          "name": "Dog Verifier",
          "description": "Ensures that each dog has a clean coat and knows how to play fetch",
          "home_url": "https://www.website.web.site/website",
          "source_url": "https://www.github.com/dogorg/dog-verifier"
        }
      }
    ],
    "enabled": true
  }
}
```

#### [Query Asset Definitions](src/query/query_asset_definitions.rs)

This route can be used to retrieve all asset definitions stored in the contract.  This response payload can be quite
large if many complex definitions are stored, so it should only used in circumstances where all asset definitions need
to be inspected or displayed.  The query asset definition route is much more efficient.

##### Request Parameters

No parameters are used for the `QueryAssetDefinitions` route.

##### Request Sample
```json
{
  "query_asset_definitions": {}
}
```

##### Response Sample
```json
{
  "data": {
    "asset_definitions": [
      {
        "asset_type": "ferret",
        "verifiers": [
          {
            "address": "tp1935mawrmyuzwuryg8wya3g6uh2vpwvapq50kvq",
            "onboarding_cost": "250",
            "onboarding_denom": "ferretcoin",
            "fee_destinations": [
              {
                "address": "tp126lrty2c0h78mdtjyzzf7mtsge427trccq8lta",
                "fee_amount": "250",
                "entity_detail": {
                  "description": "All the ferret charges go to this account"
                }
              }
            ],
            "entity_detail": {
              "name": "Ferret Verifier",
              "description": "Ensures that each ferret smells terrible because they generally do",
              "home_url": "https://www.website.web.site/website",
              "source_url": "https://www.github.com/ferretorg/ferret-verifier"
            }
          }
        ],
        "enabled": true
      }
    ]
  }
}
```

#### [Query Asset Scope Attribute](src/query/query_asset_scope_attribute.rs)

This route can be used to retrieve an existing [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs) that has
been added to a [Provenance Metadata Scope](https://docs.provenance.io/modules/metadata-module#metadata-scope) by this
contract.  This route will return a null if the scope has never had a scope attribute added to it by the contract.
This is a useful route for external consumers of the contract's data to determine if a scope (aka asset) has been
successfully classified by a verifier.

##### Request Parameters

* `identifier`: A serialized version of an [AssetIdentifier](src/core/types/asset_identifier.rs) enum.  Indicates the
target scope for the search.  The following json is an example of what this might look like in a request:
```json
{"identifier": {"type": "asset_uuid", "value": "8f9cea0a-d6e7-11ec-be71-dbbe1d4d92be"}}
```
OR
```json
{"identifier": {"type": "scope_address", "value": "scope1qzj8tjp76mn3rmyvz49c5738k2asm824ga"}}
```

##### Request Sample
```json
{
  "query_asset_scope_attribute": {
    "identifier": {
      "type": "asset_uuid",
      "value": "67b4e0b4-d706-11ec-9542-9f84339d2300"
    }
  }
}
```

##### Response Sample
```json
{
  "data": {
    "asset_uuid": "67b4e0b4-d706-11ec-9542-9f84339d2300",
    "scope_address": "scope1qpnmfc956urprmy4g20cgvuayvqqpa98dj",
    "asset_type": "heloc",
    "requestor_address": "tp18lscdretne93g0wk8ukknxp92jj9y7hmcecvf0",
    "verifier_address": "tp1un7l6rm0n2ualsrnnuvqakxr63e39gaa5h3am6",
    "onboarding_status": "approved",
    "latest_verification_result": {
      "message": "Heloc was successfully verified",
      "success": true
    },
    "access_definitions": [
      {
        "owner_address": "tp18lscdretne93g0wk8ukknxp92jj9y7hmcecvf0",
        "access_routes": [
          {
            "route": "https://www.helocplace.internet/heloc",
            "name": "download"
          }
        ],
        "definition_type": "Requestor"
      },
      {
        "owner_address": "tp1un7l6rm0n2ualsrnnuvqakxr63e39gaa5h3am6",
        "access_routes": [
          {
            "route": "https://www.internal.helocplace.internet/helocs",
            "name": "download"
          }
        ],
        "definition_type": "Verifier"
      }
    ]
  }
}
```

#### [Query Fee Payments](src/query/query_fee_payments.rs)

This route can be used to retrieve an existing [FeePaymentDetail](src/core/types/fee_payment_detail.rs) that has been
stored from a [VerifierDetailV2](src/core/types/verifier_detail.rs) during the [OnboardAsset](src/execute/onboard_asset.rs)
execution route's processes.  This route is useful in showing the expected fees to be paid when the
[VerifyAsset](src/execute/verify_asset.rs) route is executed.

##### Request Parameters

* `identifier`: A serialized version of an [AssetIdentifier](src/core/types/asset_identifier.rs) enum.  Indicates the
  target scope for the search.  The following json is an example of what this might look like in a request:
```json
{"identifier": {"type": "asset_uuid", "value": "8f9cea0a-d6e7-11ec-be71-dbbe1d4d92be"}}
```
OR
```json
{"identifier": {"type": "scope_address", "value": "scope1qzj8tjp76mn3rmyvz49c5738k2asm824ga"}}
```

##### Request Sample
```json
{
  "query_asset_scope_attribute": {
    "identifier": {
      "type": "scope_address",
      "value": "scope1qrr0argjp7p3rmv96xh62x8e8tksaue3we"
    }
  }
}
```

##### Response Sample
```json
{
  "data": {
    "scope_address": "scope1qrr0argjp7p3rmv96xh62x8e8tksaue3we",
    "payments": [
      {
        "amount": {
          "amount": "150",
          "denom": "nhash"
        },
        "name": "Fee for Contract Admin",
        "recipient": "tp1ren9rf5yshqen6zp598ux3sl2pyrzamgpua790"
      },
      {
        "amount": {
          "amount": "220",
          "denom": "nhash"
        },
        "name": "Ferret Inc. Verifier Fee",
        "recipient": "tp1zf2lct9m90nm5hrffhs2dhp3v8vr4ll4dfw3kr"
      }
    ]
  }
}
```

#### [Query State](src/query/query_state.rs)

This route can be used to retrieve the internal contract state values.  These are core configurations that denote how
the contract behaves.  They reflect the values created at instantiation and potentially modified during migration.  It
responds with a [StateV2](src/core/state.rs) struct value.

##### Request Parameters

No parameters are used for the `QueryState` route.

##### Request Sample
```json
{
  "query_state": {}
}
```

##### Response Sample
```json
{
  "data": {
    "base_contract_name": "testassets.pb",
    "admin": "tp17ryu7zepmk467s3mg5p4hnfu6k3xyh4trcn5ss",
    "is_test": true
  }
}
```

#### [Query Version](src/query/query_version.rs)

This route can be used to retrieve the internal contract version information.  It elucidates the current version of the
contract that was derived through instantiation or the most recent code migration.  It responds with a [VersionInfoV1](src/migrate/version_info.rs)
struct value.

##### Request Parameters

No parameters are used for the `QueryVersion` route.

##### Request Sample
```json
{
  "query_version": {}
}
```

##### Response Sample
```json
{
  "data": {
    "contract": "asset_classification_smart_contract",
    "version": "1.0.0"
  }
}
```

## Local Deployment

The following steps will show you how to locally run the contract with a local Provenance Blockchain instance.

1. Download and run a Provenance Blockchain localnet.   The remaining commands in this tutorial are assumed to be run
   from the provenance directory.If you already have the provenance repository cloned locally, this step can be skipped.

```shell
git clone https://github.com/provenance-io/provenance.git
git checkout main
make clean
make localnet-start
```

2. The contract needs an administrator account.  In a testnet or mainnet environment, this account is very
   important, as it controls various aspects of the contract, like maintaining asset definitions.  If you already have
   an account, this step can be skipped.  To generate an account for use locally, run the following command:

```shell
# Add the class-admin account to the local keys in build/node0.  Don't forget to hold on to the mnemonic for recovery later!
provenanced keys add class-admin --home build/node0 -t --hd-path "44'/1'/0'/0/0'" --output json | jq
# Store the account's address in a variable for easy re-use
export class_admin=$(provenanced keys show -a class-admin --home build/node0 --testnet)
# Display the value for confirmation
echo $class_admin
```

3. A new account won't do any good for storing and instantiate contracts, though.  New accounts always start out with no
   hash!  Fortunately, we can fix that by leveraging the `node0` account generated locally to send funds to the new
   account.  Run the following:

```shell
# Extracts node0's address from saved keys.  It is stored in build/node0 by default
export node0=$(provenanced keys show -a node0 --home build/node0 --testnet)
# Send the classification admin some funds from the node0 account
provenanced tx bank send \
"$node0" \
"$class_admin" \
200000000000000nhash \
--from "node0" \
--home build/node0 \
--chain-id chain-local \
--gas auto \
--gas-prices 1905nhash \
--gas-adjustment 1.2 \
--broadcast-mode block \
--testnet \
--yes \
--output json | jq
# Verify that the transfer was successful
provenanced q bank balances "$class_admin" --testnet
```

4. Now let's instantiate the contract!  Run the following, making sure to use the correct location of the wasm file
   that should exist in the `artifacts` directory of the asset-classification-smart-contract repository:

```shell
provenanced tx wasm store <your-path-here>/asset-classification-smart-contract/artifacts/asset_classification_smart_contract.wasm \ (testfiguretech/onboarding)
--instantiate-only-address "$class_admin" \
--from class-admin \
--home build/node0 \
--chain-id chain-local \
--gas auto \
--gas-prices 1905nhash \
--gas-adjustment 1.05 \
--broadcast-mode block \
--testnet \
--output json \
--yes | jq
```

5. Find the `code_id` output from the previous command.  If you're following this guide from a fresh install, the value
   should just be 1.  Let's assume it is for this next command.  Time to instantiate the contract!  Note: In some localnet
   environments, `pio` is a restricted root name and `pb` is unrestricted.  If this command fails due to a restricted
   name issue, try using `"base_contract_name": "asset.pb"` instead.

```shell
provenanced tx wasm instantiate 1 \
'{"base_contract_name": "asset.pio", "bind_base_name": true, "asset_definitions": [], "is_test": true}' \
--admin "$class_admin" \
--from class-admin \
--home build/node0 \
--label assetss \
--chain-id chain-local \
--gas auto \
--gas-prices 1905nhash \
--gas-adjustment 1.2 \
--broadcast-mode block \
--testnet \
--output json \
--yes | jq
```

Success!  The contract is now deployed locally!!
