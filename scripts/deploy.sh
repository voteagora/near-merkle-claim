#!/usr/bin/env bash
set -e

ROOT_ACCOUNT_ID=$1
TARGET=$2

# Fail if the root account ID is not set
if [ -z "$ROOT_ACCOUNT_ID" ]; then
  echo "Usage: $0 root_account_id"
  echo "Please set the root account ID."
  exit 1
fi

: "${CHAIN_ID:=mainnet}"
: ${STORAGE_DEPOSIT:="0.1 NEAR"}
# 0.1 NEAR (enough for 10000 bytes)
: ${MIN_STORAGE_DEPOSIT:="100000000000000000000000"}

export ROOT_ACCOUNT_ID="$ROOT_ACCOUNT_ID"
export CLAIMS_ACCOUNT_ID="maskc.$ROOT_ACCOUNT_ID"

echo "Creating account $CLAIMS_ACCOUNT_ID"
near --quiet account create-account fund-myself $CLAIMS_ACCOUNT_ID '1.0 NEAR' autogenerate-new-keypair save-to-keychain sign-as $ROOT_ACCOUNT_ID network-config $CHAIN_ID sign-with-keychain send

echo "Deploying and initializing Rewards contract"
near --quiet contract deploy $CLAIMS_ACCOUNT_ID use-file $TARGET with-init-call new json-args '{
  "config": {
    "owner_account_id": "'$ROOT_ACCOUNT_ID'",
    "min_storage_deposit": "'$MIN_STORAGE_DEPOSIT'",
  }
}' prepaid-gas '10.0 Tgas' attached-deposit "'$STORAGE_DEPOSIT'" network-config $CHAIN_ID sign-with-keychain send
