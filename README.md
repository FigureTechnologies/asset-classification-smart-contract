# Asset Classification Smart Contract

This contract analyzes Provenance Blockchain [scope](https://docs.provenance.io/modules/metadata-module#scope-data-structures)
metadata to verify that basic portions of newly-generated asset structures are correct, and then allows third parties
to verify the contents of the asset by granting permission to them.

## Status
[![Latest Release][release-badge]][release-latest]
[![Apache 2.0 License][license-badge]][license-url]
[![LOC][loc-badge]][loc-report]

## Build
[license-badge]: https://img.shields.io/github/license/provenance-io/asset-classification-smart-contract.svg
[license-url]: https://github.com/provenance-io/asset-classification-smart-contract/blob/main/LICENSE
[release-badge]: https://img.shields.io/github/tag/provenance-io/asset-classification-smart-contract.svg
[release-latest]: https://github.com/provenance-io/asset-classification-smart-contract/releases/latest
[loc-badge]: https://tokei.rs/b1/github/provenance-io/asset-classification-smart-contract
[loc-report]: https://github.com/provenance-io/asset-classification-smart-contract

To build the smart contract, simply run the following:
```shell
make optimize
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

Success!  The contract is now deployed locally.
