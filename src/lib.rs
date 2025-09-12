mod config;
mod merkle;

use crate::config::Config;
use near_sdk::store::LookupSet;
use near_sdk::{
    borsh, env, near, require, AccountId, BorshStorageKey, CryptoHash, NearToken,
    PanicOnDefault, Promise,
};

/// Raw type for balance in yocto NEAR.
pub type Balance = u128;

#[derive(BorshStorageKey)]
#[near]
enum StorageKeys {
    Claims,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers=[borsh])]
struct MerkleTreeData {
    account: String,
    amount: Balance,
}

// Define the contract structure
#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct MerkleClaim {
    config: Config,
    /// A map of accounts who have claimed or have yet to claim their NEAR rewards
    claims: LookupSet<AccountId>,
}

// Implement the contract structure
#[near(serializers=[borsh])]
impl MerkleClaim {
    /// Initializes the contract with the given configuration.
    #[init]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            claims: LookupSet::new(StorageKeys::Claims),
        }
    }

    pub fn claim(&mut self, amount: near_sdk::json_types::U128, merkle_proof: Vec<CryptoHash>) {
        let user_account_id = env::predecessor_account_id();
        // Check claim parameters
        require!(amount.0 > 0, "Amount must not be zero");
        require!(
            self.claims.contains(&user_account_id) == false,
            "Already claimed rewards"
        );
        require!(merkle_proof.len() > 0, "Merkle proof supplied is empty");
        require!(
            env::block_timestamp() < self.config.claim_end.into(),
            "Claim period has concluded"
        );

        // Calculate leaf to be checked alongside provided proof
        let data = MerkleTreeData {
            account: user_account_id.to_string(),
            amount: amount.0,
        };

        let serialized_data: Vec<u8> = borsh::to_vec(&data).expect("Failed to serialize data");
        let leaf = env::keccak256_array(&serialized_data);

        require!(
            Self::verify_proof(leaf, merkle_proof, self.config.merkle_root),
            "Invalid Proof"
        );

        // Mark as claimed and send NEAR to account
        self.claims.insert(env::predecessor_account_id());
        Promise::new(env::predecessor_account_id()).transfer(NearToken::from_yoctonear(amount.0));
    }

    pub fn withdraw(&mut self) {
        let caller = env::predecessor_account_id();

        require!(
            env::block_timestamp() > self.config.claim_end.into(),
            "Claim period has not finished"
        );

        require!(
            caller == self.config.owner_account_id,
            "Caller must be the owner of the claims contract"
        );

        // Send total balance to the owner
        Promise::new(caller).transfer(env::account_balance());
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_default_greeting() {
        let contract = Contract::default();
        // this test did not call set_greeting so should return the default "Hello" greeting
        assert_eq!(contract.get_greeting(), "Hello");
    }

    #[test]
    fn set_then_get_greeting() {
        let mut contract = Contract::default();
        contract.set_greeting("howdy".to_string());
        assert_eq!(contract.get_greeting(), "howdy");
    }
}
*/
