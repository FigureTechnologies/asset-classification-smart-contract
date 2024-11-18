#!/bin/sh

if ! command -v jq >/dev/null 2>&1; then
  echo "jq must be installed for this script to work"
  exit 1
fi

if [ -z "$1" ] || [ -z "$2" ]; then
  printf "Arguments: [Provenance home] [Path to contract WASM]\nExample: %s ~/git/provenance/build/node0 ./artifacts/asset_classification_smart_contract.wasm" "$0"
  exit 1
fi
PROVENANCE_HOME=$(realpath "$1")

GAS_PRICES="1905nhash"
GAS_ADJUSTMENT="1.5"
CONTRACT_LABEL="asset-classification-demo"

###
### Creates Provenance keys and returns the corresponding address
###
create_keys() {
  provenanced keys add "$1" -t --home "$PROVENANCE_HOME" --keyring-backend test --hd-path "44'/1'/0'/0/0" --output json | jq >/dev/null 2>&1
  provenanced keys show -a "$1" -t --home "$PROVENANCE_HOME" --keyring-backend test
}

### Create an address which will act as the administrator of the smart contract
ADMIN_ACCOUNT="$(create_keys contract-admin)"
export ADMIN_ACCOUNT

### Create an originator address which will onboard assets to the smart contract
ORIGINATOR_ACCOUNT="$(create_keys loan-originator)"
export ORIGINATOR_ACCOUNT

### Create a validator address which will act as a third-party reviewer that fulfills requests for classification in the smart contract
VERIFIER_ACCOUNT="$(create_keys loan-verifier)"
export VERIFIER_ACCOUNT

### Get the address of a node that can be used to fund addresses — you may need to adjust this command depending on the local Provenance setup you're using
NODE0="$(provenanced keys show -a node0 -t --home "$PROVENANCE_HOME" --keyring-backend test)"

###
### Creates an account for a given address by giving it hash
###
create_account() {
  if [ -z "$1" ] || [ -z "$2" ]; then
    echo "Must supply account address and description"
    return 1
  fi

  ### Transfer hash from the node to the given address
  provenanced tx bank send \
    "$NODE0" \
    "$1" \
    350000000000nhash \
    --from "$NODE0" \
    --testnet \
    --keyring-backend test \
    --home "$PROVENANCE_HOME" \
    --chain-id chain-local \
    --gas auto \
    --gas-prices "$GAS_PRICES" \
    --gas-adjustment "$GAS_ADJUSTMENT" \
    --yes \
    --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json | jq '{ txhash, raw_log }' || exit 1

  ### Verify that the account has been created & funded
  BALANCE=$(provenanced query bank balances "$1" -t --home "$PROVENANCE_HOME" -o json | jq -re '.balances[] | select(.denom=="nhash") | .amount' 2>/dev/null)
  if [ -z "$BALANCE" ] || [ "$BALANCE" -le 0 ]; then
    echo "Failed to fund account for $2"
    return 1
  else
    echo "Successfully created & funded account for $2 ($1)"
    return 0
  fi
}

### Create some accounts that will be used as the parties involved in the contract
create_account "$ADMIN_ACCOUNT" "contract administrator" || exit 1
create_account "$ORIGINATOR_ACCOUNT" "loan originator" || exit 1
create_account "$VERIFIER_ACCOUNT" "loan verifier" || exit 1

### Create an unrestricted name that we will bind the address of the smart contract to
provenanced tx name bind \
  "sc" \
  "$NODE0" \
  "pb" \
  --unrestrict \
  --from "$NODE0" \
  --testnet \
  --keyring-backend test \
  --home "$PROVENANCE_HOME" \
  --chain-id chain-local \
  --gas auto \
  --gas-prices "$GAS_PRICES" \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --yes \
  --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json | jq '{ txhash, raw_log }' || exit 1

echo "Created unrestricted name for contract to bind to"

PATH_TO_CONTRACT_WASM=$(realpath "$2")

### Store the optimized contract WASM file to the chain
WASM_STORE=$(
  provenanced tx wasm store "$PATH_TO_CONTRACT_WASM" \
    --instantiate-anyof-addresses "$ADMIN_ACCOUNT" \
    --from "$ADMIN_ACCOUNT" \
    --testnet \
    --keyring-backend test \
    --home "$PROVENANCE_HOME" \
    --chain-id chain-local \
    --gas auto \
    --gas-prices "$GAS_PRICES" \
    --gas-adjustment "$GAS_ADJUSTMENT" \
    --yes \
    --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json || exit 1
)

echo "$WASM_STORE" | jq '{ txhash, raw_log }'

echo "Stored the smart contract code"

### Verify that the code was stored
STORED_CODE=$(provenanced query wasm list-code --home "$PROVENANCE_HOME" -o json)

echo "$STORED_CODE" | jq

### Gets the code ID for our contract from the above output
AC_CODE_ID=$(echo "$STORED_CODE" | jq -r "[.code_infos[] | select(.creator == \"$ADMIN_ACCOUNT\")][0] | .code_id")

CURRENT_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
INSTANTIATION_MESSAGE_PATH="$CURRENT_DIR/example_instantiation_message.json"

### Instantiate the contract
provenanced tx wasm instantiate "$AC_CODE_ID" \
  "$(cat "$INSTANTIATION_MESSAGE_PATH")" \
  --admin "$ADMIN_ACCOUNT" \
  --label "$CONTRACT_LABEL" \
  --from "$ADMIN_ACCOUNT" \
  --testnet \
  --keyring-backend test \
  --home "$PROVENANCE_HOME" \
  --chain-id chain-local \
  --gas auto \
  --gas-prices "$GAS_PRICES" \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --yes \
  --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json | jq '{ txhash, raw_log }' || exit 1

echo "Instantiated the smart contract"

### Verify that the contract can be queried by code ID
provenanced query wasm list-contract-by-code "$AC_CODE_ID" --home "$PROVENANCE_HOME" -t -o json | jq || exit 1

### Store the address of the contract for convenience
AC_CONTRACT_ADDRESS=$(provenanced query wasm list-contract-by-code "$AC_CODE_ID" --home "$PROVENANCE_HOME" -t -o json | jq -r '.contracts[0]') # Adjust this jq filter if you used the same admin account to instantiate the contract a second time
export AC_CONTRACT_ADDRESS

if [ -z "$AC_CONTRACT_ADDRESS" ] || [ "$AC_CONTRACT_ADDRESS" = "null" ]; then
  echo "Failed to set up the contract"
  exit 1
fi

### Ensure that querying the contract works
provenanced query wasm contract-state smart "$AC_CONTRACT_ADDRESS" '{"query_state":{}}' --home "$PROVENANCE_HOME" -t -o json | jq '.data' && echo "The contract has been set up with address $AC_CONTRACT_ADDRESS" || exit 1

### Add an asset definition

provenanced tx wasm execute "$AC_CONTRACT_ADDRESS" \
  "{
    \"add_asset_definition\": {
      \"asset_definition\": {
        \"asset_type\": \"mortgage\",
        \"display_name\": \"Mortgage\",
        \"verifiers\": [
          {
            \"address\": \"$VERIFIER_ACCOUNT\",
            \"onboarding_cost\": \"30000000000\",
            \"onboarding_denom\": \"nhash\",
            \"fee_destinations\": [
              {
                \"address\": \"$VERIFIER_ACCOUNT\",
                \"fee_amount\": \"30000000000\"
              }
            ]
          }
        ],
        \"enabled\": true
      }
    }
  }" \
  --from "$ADMIN_ACCOUNT" \
  --testnet \
  --keyring-backend test \
  --home "$PROVENANCE_HOME" \
  --chain-id chain-local \
  --gas auto \
  --gas-prices "$GAS_PRICES" \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --yes \
  --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json | jq '{ txhash, raw_log }' || exit 1

provenanced query wasm contract-state smart "$AC_CONTRACT_ADDRESS" '{"query_asset_definitions":{}}' --home "$PROVENANCE_HOME" -t -o json | jq '.data[]' && echo "Successfully created a new asset definition" || exit 1

### Create a contract specification so that we can create a scope specification

CONTRACT_SPEC_ADDRESS="contractspec1qdcgh2qltruylry66e4qwwe99tls9k9yca"

provenanced tx metadata write-contract-specification "$CONTRACT_SPEC_ADDRESS" "$ORIGINATOR_ACCOUNT" "owner" "hashvalue" "SomeClassName" \
  --from "$ORIGINATOR_ACCOUNT" \
  --testnet \
  --keyring-backend test \
  --home "$PROVENANCE_HOME" \
  --chain-id chain-local \
  --gas auto \
  --gas-prices "$GAS_PRICES" \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --yes \
  --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json | jq '{ txhash, raw_log }' || exit 1

provenanced query metadata contractspec "$CONTRACT_SPEC_ADDRESS" --home "$PROVENANCE_HOME" -t -o json | jq >/dev/null 2>&1 && echo "Successfully created a new contract specification" || exit 1

### Create a scope so that we can onboard something to the contract

SCOPE_SPEC_ADDRESS="scopespec1qngj7w3uwns5xgv0uhjnx5ryznvshg0aht"

provenanced tx metadata write-scope-specification "$SCOPE_SPEC_ADDRESS" "$ORIGINATOR_ACCOUNT" "owner" "$CONTRACT_SPEC_ADDRESS" \
  --from "$ORIGINATOR_ACCOUNT" \
  --testnet \
  --keyring-backend test \
  --home "$PROVENANCE_HOME" \
  --chain-id chain-local \
  --gas auto \
  --gas-prices "$GAS_PRICES" \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --yes \
  --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json | jq '{ txhash, raw_log }' || exit 1

provenanced query metadata scopespec "$SCOPE_SPEC_ADDRESS" --home "$PROVENANCE_HOME" -t -o json | jq >/dev/null 2>&1 && echo "Successfully created a new scope specification" || exit 1

SCOPE_UUID="c5bc3231-39ed-4573-b442-01dae6bb00ae"
SCOPE_ADDRESS="scope1qrzmcv3388k52ua5ggqa4e4mqzhqqplxue"

provenanced tx metadata write-scope "$SCOPE_ADDRESS" "$SCOPE_SPEC_ADDRESS" "$ORIGINATOR_ACCOUNT" "$ORIGINATOR_ACCOUNT" "$ORIGINATOR_ACCOUNT" \
  --from "$ORIGINATOR_ACCOUNT" \
  --testnet \
  --keyring-backend test \
  --home "$PROVENANCE_HOME" \
  --chain-id chain-local \
  --gas auto \
  --gas-prices "$GAS_PRICES" \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --yes \
  --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json | jq '{ txhash, raw_log }' || exit 1

provenanced query metadata scope "$SCOPE_ADDRESS" --home "$PROVENANCE_HOME" -t -o json | jq >/dev/null 2>&1 && echo "Successfully created a new scope" || exit 1

### Create a record in the scope so that the scope is qualified for onboarding to the contract

RECORD_SPEC_ADDRESS="recspec1q4cgh2qltruylry66e4qwwe99tl5w2alzjfrutnualv99xp9csq7sw828k9"
RECORD_META_ADDRESS="record1qf3n2cnrxverxvfdxvuk2epdxs6nwvedvg6rgv3dxqckgct9xe3xyvpsv9jkcmmpdcjga2tx"

provenanced tx metadata write-record-specification \
  "$RECORD_SPEC_ADDRESS" \
  "loan" \
  "loanBuilder,org.example.loan.model,someChecksum" \
  "init,org.example.loan.other,$RECORD_META_ADDRESS" \
  "record_list" \
  "owner" \
  --from "$ORIGINATOR_ACCOUNT" \
  --testnet \
  --keyring-backend test \
  --home "$PROVENANCE_HOME" \
  --chain-id chain-local \
  --gas auto \
  --gas-prices "$GAS_PRICES" \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --yes \
  --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json | jq '{ txhash, raw_log }' || exit 1

provenanced query metadata recordspec "$RECORD_SPEC_ADDRESS" --home "$PROVENANCE_HOME" -t -o json | jq >/dev/null 2>&1 && echo "Successfully created a new record specification" || exit 1

provenanced tx metadata write-record \
  "$SCOPE_ADDRESS" \
  "$RECORD_SPEC_ADDRESS" \
  "loan" \
  "someProcess,anotherChecksum,org.example.loan.model" \
  "loanBuilder,inputHashValue,org.example.loan.model,proposed" \
  "newHashValue,pass" \
  "$ORIGINATOR_ACCOUNT,owner" \
  "$CONTRACT_SPEC_ADDRESS" \
  --from "$ORIGINATOR_ACCOUNT" \
  --testnet \
  --keyring-backend test \
  --home "$PROVENANCE_HOME" \
  --chain-id chain-local \
  --gas auto \
  --gas-prices "$GAS_PRICES" \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --yes \
  --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json | jq '{ txhash, raw_log }' || exit 1

RECORDS_COUNT=$(
  provenanced query metadata scope "$SCOPE_ADDRESS" \
    --include-records \
    --home "$PROVENANCE_HOME" \
    --testnet \
    --output json | jq '.records | length'
)

if [ -z "$RECORDS_COUNT" ] || [ "$RECORDS_COUNT" -le 0 ]; then
  echo "Failed to add record to scope"
  exit 1
else
  echo "Successfully added a record to the scope"
fi

### Onboard the scope to the contract

get_nhash_balance() {
  provenanced query bank balance "$1" nhash --testnet --home "$PROVENANCE_HOME" -o json | jq -r '.balance.amount'
}

echo "Loan originator has $(get_nhash_balance "$ORIGINATOR_ACCOUNT") nhash before executing the asset onboarding"

provenanced tx wasm execute "$AC_CONTRACT_ADDRESS" \
  "{
    \"onboard_asset\": {
      \"identifier\": {
        \"type\": \"asset_uuid\",
        \"value\": \"$SCOPE_UUID\"
      },
      \"asset_type\": \"mortgage\",
      \"verifier_address\": \"$VERIFIER_ACCOUNT\"
    }
  }" \
  --fees 35000000000nhash \
  --from "$ORIGINATOR_ACCOUNT" \
  --testnet \
  --keyring-backend test \
  --home "$PROVENANCE_HOME" \
  --chain-id chain-local \
  --gas auto \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --yes \
  --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json | jq '{ txhash, raw_log }'

echo "Loan originator has $(get_nhash_balance "$ORIGINATOR_ACCOUNT") nhash after executing the asset onboarding"

### Verify that the scope was successfully onboarded

SCOPE_ATTRIBUTES=$(
  provenanced query wasm contract-state smart "$AC_CONTRACT_ADDRESS" \
    "{
      \"query_asset_scope_attributes\": {
        \"identifier\": {
          \"type\": \"scope_address\",
          \"value\": \"$SCOPE_ADDRESS\"
        }
      }
    }" \
    --home "$PROVENANCE_HOME" \
    --testnet \
    --output json | jq '.data'
)

printf "Scope attributes on chain: "
provenanced query attribute list "$SCOPE_ADDRESS" --home "$PROVENANCE_HOME" --testnet -o json | jq '.attributes[] | .value |= (@base64d | fromjson)'

if [ -z "$SCOPE_ATTRIBUTES" ] || [ "$SCOPE_ATTRIBUTES" = "null" ]; then
  echo "Failed to onboard the scope to the contract, check the raw_log above for more information"
  exit 1
else
  printf "Scope state in contract: "
  echo "$SCOPE_ATTRIBUTES" | jq
  echo "Successfully onboarded a scope to the contract"
fi

### Perform verification of the scope

provenanced tx wasm execute "$AC_CONTRACT_ADDRESS" \
  "{
    \"verify_asset\": {
      \"identifier\": {
        \"type\": \"scope_address\",
        \"value\": \"$SCOPE_ADDRESS\"
      },
      \"asset_type\": \"mortgage\",
      \"success\": true,
      \"message\": \"Successfully verified! :D\"
    }
  }" \
  --from "$VERIFIER_ACCOUNT" \
  --testnet \
  --keyring-backend test \
  --home "$PROVENANCE_HOME" \
  --chain-id chain-local \
  --gas auto \
  --gas-prices "$GAS_PRICES" \
  --gas-adjustment "$GAS_ADJUSTMENT" \
  --yes \
  --output json | provenanced query wait-tx --home "$PROVENANCE_HOME" -o json | jq '{ txhash, raw_log }'

### Verify that the scope was successfully verified

UPDATED_SCOPE_ATTRIBUTE=$(
  provenanced query wasm contract-state smart "$AC_CONTRACT_ADDRESS" \
    "{
      \"query_asset_scope_attributes\": {
        \"identifier\": {
          \"type\": \"scope_address\",
          \"value\": \"$SCOPE_ADDRESS\"
        }
      }
    }" \
    --home "$PROVENANCE_HOME" \
    --testnet \
    --output json | jq '.data'
)

printf "Scope attributes on chain: "
provenanced query attribute list "$SCOPE_ADDRESS" --home "$PROVENANCE_HOME" --testnet -o json | jq '.attributes[] | .value |= (@base64d | fromjson)'

if [ -z "$UPDATED_SCOPE_ATTRIBUTE" ] || [ "$UPDATED_SCOPE_ATTRIBUTE" = "null" ] || [ "$(echo "$UPDATED_SCOPE_ATTRIBUTE" | jq -r '.[0] | .onboarding_status')" != "approved" ]; then
  echo "Failed to onboard the scope to the contract, check the raw_log above for more information"
  exit 1
else
  printf "Scope state in contract: "
  echo "$UPDATED_SCOPE_ATTRIBUTE" | jq
  echo "Successfully verified the scope"
fi
