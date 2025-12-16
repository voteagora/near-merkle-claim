# near-merkle-claim

This is an implementation of a Merkle claim contract that utilizes keccack256 hashes to prove balances for a given accountId. This can be used to distribute NEAR
tokens to these respective users given a predefined list of recipients. These contracts use the accumulator pattern to show the inclusion of a given hash
in the merkle tree set.

## How It Works

1. Owner funds the contract and supplies a merkle root, and claim end period for each campaign
2. Users(accountId) who are recipients can supply a proof along with the expected balance to the `claim` method. Once the proof is verified, NEAR is sent.
3. After the claim period ends, the owner can then withdraw the remaining unclaimed balance.

## How to Build Locally?

Install [`cargo-near`](https://github.com/near/cargo-near) and run:

```bash
cargo near build
```

## How to Test Locally?

```bash
cargo test
```

### Building release candidate

Check the release tags for [latest](https://github.com/voteagora/near-merkle-claim/releases/tag/v1.0.0)

Before building or verifying code hashes run:
```
git fetch --tags
git checkout v1.0.0

cargo near build
-> reproducible-wasm
```

## How to Deploy?

### Configuration

A JSON configuration needs to be provided to initialize the contract using the `new()` method. These values cannot be changed at a later time once the contract is deployed. Furthermore, it is important that the owner / or some party funds the contract with the appropiate balance to allow users to withdraw. 

`owner_account_id: AccountId` - This user can withdraw the total amount of funds once the claim period ends.
`min_storage_deposit: NearToken` - When initializing the contract ensure to deposit NEAR that exceeds this value, it is used for storage.

### Creating a Campaign

Once the trie has been generated the merkle root must be published along with a claim end timestamp:

```
{"merkle_root": [...], "claim_end": "1789228321000000000"}
```

Deployment is automated with GitHub Actions CI/CD pipeline.
To deploy manually, install [`cargo-near`](https://github.com/near/cargo-near) and run:

If you deploy for debugging purposes:
```bash
cargo near deploy build-non-reproducible-wasm <account-id>
```

If you deploy production ready smart contract:
```bash
cargo near deploy build-reproducible-wasm <account-id>
```

### On/offchain data model

The merkle root for every campaign is on-chain and is only indexable through the `create_campaign` events. The proof artifacts the user supplies, as well as the
entire merkle tree generated from the CSV is computed off-chain. A provider will obtain the data to populate the CSV with users account ids, lockup contract account,
and total reward amount accrued. This data is indexed and assembled from the `venear.dao` contract on mainnet.
