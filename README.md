# Asset Classification Smart Contract

This contract analyzes Provenance Blockchain [scope](https://docs.provenance.io/modules/metadata-module#scope-data-structures)
metadata to verify that basic portions of newly-generated asset structures are correct, and then allows third parties
to verify the contents of the asset by granting permission to them.

## Status
[![Latest Release][release-badge]][release-latest]
[![Apache 2.0 License][license-badge]][license-url]
[![LOC][loc-badge]][loc-report]

[license-badge]: https://img.shields.io/github/license/provenance-io/asset-classification-smart-contract.svg
[license-url]: https://github.com/provenance-io/asset-classification-smart-contract/blob/main/LICENSE
[release-badge]: https://img.shields.io/github/tag/provenance-io/asset-classification-smart-contract.svg
[release-latest]: https://github.com/provenance-io/asset-classification-smart-contract/releases/latest
[loc-badge]: https://tokei.rs/b1/github/provenance-io/asset-classification-smart-contract
[loc-report]: https://github.com/provenance-io/asset-classification-smart-contract

## Build

To build the smart contract, simply run the following:
```shell
make optimize
```

## Descriptions / Configuration

### Processes

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

### Account Roles

The contract interacts with various Provenance Blockchain accounts throughout its various execution processes.  This is
meant to be an exhaustive list of all different types of accounts that may have interactions:

* __Admin Account__: This account is used to store and instantiate the smart contract.  When the contract is instantiated,
the sender address is automatically used as the admin.  This account is required to execute many of the
contract execution endpoints.  This address can later be changed when running a contract migration.
* __Verifier Account__: This account type is used in the contract's [AssetDefinition](src/core/types/asset_definition.rs)'s
[VerifierDetail](src/core/types/verifier_detail.rs).  It indicates an account that will inspect the events emitted by
the contract, receive all or a portion of the fees sent to the contract during the onboarding process, and perform
verification of an underlying scope.  Eventually, this account is tasked with calling into the [Verify Asset](src/execute/verify_asset.rs)
execution route to specify whether or not an onboarded scope is valid and therefore verified.
* __Onboarding Account__: This account is not stored in the contract, and can be any Provenance Blockchain address.  It
is the primary consumer of the contract's functionality, and calls into the [Onboard Asset](src/execute/onboard_asset.rs)
execution route, specifying a Verifier Account and paying the fees required by the verifier's [VerifierDetail](src/core/types/verifier_detail.rs).
* __Fee Account__: This account is an optional specification in a [VerifierDetail](src/core/types/verifier_detail.rs) and,
when specified, indicates that some or all of the fees provided during the onboarding process should be sent to this address.
The fee account is specified directly in a [FeeDestination](src/core/types/fee_destination.rs), nested within the [VerifierDetail](src/core/types/verifier_detail.rs).
There can be multiple Fee Accounts for a single Verifier Account, ensuring that any amount of fee division can occur.

### Execution Routes

The contract exposes various execute routes by which interaction is possible.  All execution route enum variants are
defined in the [ExecuteMsg Enum](src/core/msg.rs).   The json schema for sending a contract execution transation message
is defined in the [Execute Schema Json](schema/execute_msg.json).

#### Onboard Asset

This route is the primary interaction point for most consumers.  It consumes an asset uuid or scope address, the type of
asset corresponding to that scope (heloc, mortgage, payable, etc), and, if all checks pass, attaches an attribute to the
provided scope that stages the scope for verification of authenticity by the specified verifier in the request.  The
attribute is attached based on the `base_contract_name` specified in the contract, combined with the specified asset type
in the request.  Ex: if `base_contract_name` is "asset" and the asset type is "myasset", the attribute would be assigned
under the name of "myasset.asset".  All available asset types are queryable, and stored in the contract as [AssetDefinition](src/core/types/asset_definition.rs)
values.  After onboarding is completed, an [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs) will be
stored on the scope with an [AssetOnboardingStatus](src/core/types/asset_onboarding_status.rs) of `Pending`, indicating
that the asset has been onboarded to the contract but is awaiting verification.

Note: The account that invokes the `OnboardAsset` execution route must be the owner of the scope referenced in the
request.

The various parameters for the `OnboardAsset` execution route are as follows:

* `identifier`: A serialized version of an [AssetIdentifier](src/core/types/asset_identifier.rs) enum.  Indicates the
scope to onboard.  The following json is an example of what this might look like in a request:
```json
{"identifier": {"type": "asset_uuid", "value": "8f9cea0a-d6e7-11ec-be71-dbbe1d4d92be"}}
```
OR
```json
{"identifier": {"type": "scope_address", "value": "scope1qzj8tjp76mn3rmyvz49c5738k2asm824ga"}}
```
* `asset_type`: A name that must directly match one of the contract's internal [AssetDefinition](src/core/types/asset_definition.rs)
names.  Any request with a specified type not matching an asset definition will be rejected outright.
* `verifier_address`: The bech32 address of a Verifier Account associated with the targeted [AssetDefinition](src/core/types/asset_definition.rs),
within its nested vector of [VerifierDetail](src/core/types/verifier_detail.rs)s.
* `access_routes`: An optional parameter that allows the specification of a location to get the underlying asset data
for the specified scope.  The [AccessRoute](src/core/types/access_route.rs) struct is very generic in its composition
for the purpose of allowing various different solutions to fetching asset data.  If the verification process requires
generic lookups for each onboarded asset, access routes on the scope's [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs)
can be leveraged to easily determine the source of the underlying data.  If these values are omitted at first, but later needed,
they can always be added by using the `UpdateAccessRoutes` execution route.  Note: Access routes can specify a `name`
parameter, as well, to indicate the reason for the route, but this is entirely optional.

__Full Request Sample__:
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
    ]
  }
}
```

#### Verify Asset

This route is specifically designed to allow a Verifier specified in the [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs)
of a [Provenance Metadata Scope](https://docs.provenance.io/modules/metadata-module#scope-data-structures) to indicate to
the owner of the scope whether or not the content within the scope was valid or not.  The Verifier Account, after determining
validity of the underlying data, will either mark the classification as a success or failure.  This route will reject
all invokers except for Verifiers linked to a scope by the scope attribute, ensuring that only the verifier requested
has the permission needed to classify an asset.  In this way, the process for verification ensures that all involved
parties' requirements for security are satisfied.  In addition, the verifier used in the process is stored on the scope
attribute after the fact, ensuring that external inspectors of the generated attribute can choose which verifications to
acknowledge and which to disregard.

The various parameters for the `VerifyAsset` execution route are as follows:

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
process that retries logic when exceptions or other code execution issues cause a failued verification.
* `message`: An optional string describing the result of the verification process.  If omitted, a standard message
describing success or failure based on the value of `success` will be displayed in the [AssetScopeAttribute](src/core/types/asset_scope_attribute.rs).
* `access_routes`: Like in the `OnboardAsset` message, this parameter allows the verifier to provide access routes for
the assets that it has successfully fetched from the underlying scope data.  This allows for the verifier to define its
own subset of [AccessRoute](src/core/types/access_route.rs) values to allow actors with permission to easily fetch asset
data from a new location, potentially without any Provenance Blockchain interaction, facilitating the process of data
interaction.

__Full Request Sample__:
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
    ]
  }
}
```

#### Add Asset Definition

__This route is only accessible to the contract's admin address.__  This route allows a new [AssetDefinition](src/core/types/asset_definition.rs)
value to be added to the contract's internal storage.  These asset definitions dictate which asset types are allowed to
be onboarded, as well as which verifiers are tied to each asset type.  Each added asset definition must be unique in
two criteria:
* Its `asset_type` value must not yet be registrered in a different asset definition.
* Its `scope_spec_address` (entered as a [ScopeSpecIdentifier](src/core/types/scope_spec_identifier.rs)) must also be
unique across asset definitions.
Additionally, all added asset definitions must refer to an existing [Provenance Metadata Scope Specification](https://docs.provenance.io/modules/metadata-module#scope-specification).

The various parameters for the `AddAssetDefinition` execution route are as follows:

* `asset_definition`: An [AssetDefinitionInput](src/core/types/asset_definition.rs) value defining all of the new
[AssetDefinition](src/core/types/asset_definition.rs)'s values.  The execution route converts the incoming value to an
asset definition.

__Full Request Sample__:
```json
{
  "add_asset_definition": {
    "asset_definition": {
      "asset_type": "car",
      "scope_spec_identifier": {
        "type": "address",
        "value": "scopespec1qjhsfqsj6meprmyrvmhn64pquchqmu0qaw"
      },
      "verifiers": [
        {
          "address": "tp1eqfg2lxlwqs23m320arhuk3dad47ha6tvzu5n5",
          "onboarding_cost": "1000000000",
          "onboarding_denom:": "nhash",
          "fee_percent": "0.5",
          "fee_destinations": [
            {
              "address": "tp1s735l5tmh7sngyvvn6rf4l7e9e9qq8uz93z9ky",
              "fee_percent": "0.5"
            },
            {
              "address": "tp1k5m8lkshpupuf44p02r4mwgjdawd2kta4n9ry0",
              "fee_percent": "0.5"
            }
          ],
          "entity_detail": {
            "name": "My Company's Verifier",
            "description": "Verifies car assets better than any other car verifier you can think of",
            "home_url": "https://www.mycompany.wesellcars/info",
            "source_url": "https://www.github.com/CarCompanyCodePlace/car-verifier-application"
          }
        }
      ],
      "enabled": "true",
      "bind_name": "true"
    }
  }
}
```

#### Update Asset Definition
__This route is only accessible to the contract's admin address.__ This route allows an existing [AssetDefinition](src/core/types/asset_definition.rs)
value to be updated.  It works by matching the input's `asset_type` to an existing asset definition and overwriting the
existing values.  If no asset definition exists for the given type, the request will be rejected.  Contract validation
ensures that after the update, all scope specification addresses contained in asset definitions remain unique, as well.

The various parameters for the `UpdateAssetDefinition` execution route are as follows:

* `asset_definition`: An [AssetDefinitionInput](src/core/types/asset_definition.rs) value defining all of the updated
  [AssetDefinition](src/core/types/asset_definition.rs)'s values.  The execution route converts the incoming value to an
  asset definition.

__Full Request Sample__:
```json
{
  "update_asset_definition": {
    "asset_definition": {
      "asset_type": "car",
      "scope_spec_identifier": {
        "type": "address",
        "value": "scopespec1qjm3wkwc6me3rmyde635xcmqnq8q8yecd9"
      },
      "verifiers": [
        {
          "address": "tp1uvnpfg9hmeyuf0t3a6l9xhegx8ewhtk9z683x4",
          "onboarding_cost": "15000000000",
          "onboarding_denom:": "carcoins",
          "fee_percent": "0.25",
          "fee_destinations": [
            {
              "address": "tp1s735l5tmh7sngyvvn6rf4l7e9e9qq8uz93z9ky",
              "fee_percent": "1.0"
            }
          ],
          "entity_detail": {
            "name": "My Company's Verifier",
            "description": "Verifies car assets better than any other car verifier you can even FATHOM",
            "home_url": "https://www.mycompany.wesellcars/info",
            "source_url": "https://www.github.com/CarCompanyCodePlace/car-verifier-application"
          }
        }
      ],
      "enabled": "true"
    }
  }
}
```

#### Toggle Asset Definition
__This route is only accessible to the contract's admin address.__ This route toggles an existing [AssetDefinition](src/core/types/asset_definition.rs)
from enabled to disabled, or disabled to enabled.  When disabled, an asset definition will no longer allow new assets to
be onboarded to the contract.  Existing assets already onboarded to the contract and in pending status will still be
allowed to be verified, but new values will be rejected.  This same functionality could be achieved with an invocation of
the `UpdateAssetDefinition` route but swapping the `enabled` value on the `asset_definition` parameter, but this route
is significantly simpler and prevents accidental data mutation due to it not requiring the entirety of the definition's
values.

The various parameters for the `ToggleAssetDefinition` execution route are as follows:

* `asset_type`: The type of asset for which the definition's `enabled` value will be toggled.  As the asset type value
on each asset definition is guaranteed to be unique, this key is all that is needed to find the target definition.
* `expected_result`: The value of `enabled` after the toggle takes place.  This value is required to ensure that
multiple toggles executed in succession (either by accident or by various unrelated callers) will only be honored if
the asset definition is in the intended state during the execution of the route.

__Full Request Sample__:
```json
{
  "toggle_asset_definition": {
    "asset_type": "airplane",
    "expected_result": "false"
  }
}
```

#### Add Asset Verifier
__This route is only accessible to the contract's admin address.__ This route adds a new [VerifierDetail](src/core/types/verifier_detail.rs)
to an existing [AssetDefinition](src/core/types/asset_definition.rs).  This route is intended to register new verifiers
without the bulky requirements of the `UpdateAssetDefinition` execution route.  This route will reject verifiers added
with addresses that match any other verifiers on the target asset definition.

The various parameters for the `AddAssetVerfifier` execution route are as follows:

* `asset_type`: The type of asset for which the new [VerifierDetail](src/core/types/verifier_detail.rs) will be added.
This must refer to an existing [AssetDefinition](src/core/types/asset_definition.rs)'s `asset_type` value, or the request
will be rejected.
* `verifier`: The new [VerifierDetail](src/core/types/verifier_detail.rs) to be added to the asset definition, with all
of its required values.  No verifiers within the existing asset definition must have the same `address` value of this
parameter, or the request will be rejected.

__Full Request Sample__:
```json
{
  "add_asset_verifier": {
    "asset_type": "train",
    "verifier": {
      "address": "tp149832nekuva7lxtzcezlwhzqjc32vu8lsaqcuv",
      "onboarding_cost": "250",
      "onboarding_denom": "traintoken",
      "fee_percent": "1.0",
      "fee_destinations": [
        {
          "address": "tp1lmp2qmntl090whuftym0wthzwjc8xv0v79cu6l",
          "fee_percent": "1.0"
        }
      ],
      "entity_detail": {
        "name": "Train Verifier",
        "description": "Verifies that trains can successfully fit on the tracks",
        "home_url": "https://www.trainsrus.edu.gov.com.org.eduagain",
        "source_url": "https://github.com/samuelmarina/is-even"
      }
    }
  }
}
```

#### Update Asset Verifier
__This route is only accessible to the contract's admin address.__ This route updates an existing [VerifierDetail](src/core/types/verifier_detail.rs)
in an existing [AssetDefinition](src/core/types/asset_definition.rs).  This route is intended to be used when the values
of a single verifier detail need to change, but not the entire asset definition.  The request will be rejected if the
referenced asset definition is not present within the contract, or if a verifier does not exist within the asset
definition that matches the address of the provided verifier data.

The various parameters for the `UpdateAssetVerifier` execution route are as follows:

* `asset_type`: The type of asset for which the [VerifierDetail](src/core/types/verifier_detail.rs) will be updated. This
must refer to an existing [AssetDefinition](src/core/types/asset_definition.rs)'s `asset_type` value, or the request
will be rejected.
* `verifier`: The updated [VerifierDetail](src/core/types/verifier_detail.rs) to be modified in the asset definition.
An existing verifier detail within the target asset definition must have a matching `address` value, or the request will
be rejected.

__Full Request Sample__:
```json
{
  "update_asset_verifier": {
    "asset_type": "widget",
    "verifier": {
      "address": "tp15n6as7tytrza9692anawwc52kyg5pv86lpeyhu",
      "onboarding_cost": "200",
      "onboarding_denom": "widgetdollar",
      "fee_percent": "0",
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

#### Update Access Routes
__This route is only accessible to the contract's admin address OR to the owner of the access routes being updated.__
This route will swap all existing access routes for a specific owner for a specific scope to the provided values. These
access routes either correspond to those created during the onboarding process, or those created during the verification
process.

The various parameters for the `UpdateAccessRoutes` execution route are as follows:

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

__Full Request Sample__:
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

#### Bind Contract Alias
__This route is only accessible to the contract's admin address.__ The [Provenance Name Module](https://docs.provenance.io/modules/name-module)
offers a very elegant method of lookup for addresses when a name has been bound to an address.  This execution route
allows for a name to be bound directly to the contract within the contract itself.  Due to the nature of how the name
module works, public names can only be bound by the requesting account (in this case, the contract) or by the name
owner.  In most cases, users won't have access to the root name owner of an unrestricted name, but will want to bind a
name to the contract in order to facilitate lookups.  This allows any unrestricted name to be bound to the contract with
ease.  This route will fail execution if a name is provided that stems from a restricted parent.

The various parameters for the `BindContractAlias` execution route are as follows:

* `alias_name`: The name to bind to the contract.  Ex: `assetclassificationalias.pb`.

__Full Request Sample__:
```json
{
  "bind_contract_alias": {
    "alias_name": "assetclassificationalias.pb"
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
   should just be 1.  Let's assume it is for this next command.  Time to instantiate the contract!

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
