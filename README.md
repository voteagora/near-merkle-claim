# near-merkle-claim

This contract implements a Merkle-based claim mechanism that uses keccak256 hashes to verify an account’s allocated balance. It enables the distribution of NEAR tokens to a predefined list of recipients by allowing each user to cryptographically prove their eligibility. The contract uses an accumulator pattern to verify that a given hash is included in the Merkle tree.

## How It Works

1. The owner initializes each campaign by funding the contract and providing a Merkle root along with a claim end date.
2. Eligible users (by accountId) submit a claim by calling the claim method with their Merkle proof and expected balance. If the proof is valid, the contract transfers the corresponding amount of NEAR to the user.
3. After the claim period expires, the owner may withdraw any remaining unclaimed NEAR from the contract.

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

`owner_account_id: AccountId` - This user can withdraw remaining funds once the the claim period ends.
`min_storage_deposit: NearToken` - When initializing the contract ensure to deposit NEAR that exceeds this value, it is used for storage.

### Creating a Campaign

Once the trie has been generated the Merkle root must be published along with a claim end timestamp:

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

### On/offchain Data Model

For each campaign, the Merkle root is stored on-chain and can only be discovered by indexing the `create_campaign` events. The Merkle proofs submitted by users, as well as the full Merkle tree derived from the CSV, are generated off-chain.

A data provider collects the information needed to build the CSV—including user account IDs, lockup contract accounts, and total accrued rewards. This data is indexed and aggregated from the `venear.dao` contract on mainnet.
