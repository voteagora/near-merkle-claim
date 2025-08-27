mod config;
mod merkle;

use crate::config::Config;
use near_sdk::{log, env, near, require, Promise, CryptoHash, BorshStorageKey, AccountId, PanicOnDefault};
use near_sdk::store::{LookupSet, Vector};

/// Raw type for balance in yocto NEAR.
pub type Balance = u128;

#[derive(BorshStorageKey)]
#[near]
enum StorageKeys {
    Claims,
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
#[near]
impl MerkleClaim {
    /// Initializes the contract with the given configuration.
    #[init]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            claims: LookupSet::new(StorageKeys::Claims),
        }
    }

    pub fn claim(&mut self, amount: Balance, merkle_proof: Vec<CryptoHash>) {
        let user_account_id = env::predecessor_account_id();

        require!(
            amount > 0,
            "Amount must not be zero"
        );

        require!(
            self.claims.contains(&user_account_id) == false,
            "Already claimed rewards"
        );

        require!(
            merkle_proof.len() > 0,
            "Merkle proof supplied is empty"
        );

        require!(
            env::block_timestamp() <self.config.claim_end.into(),
            "Claim period has concluded"
        );

        // Calculate leaf to be checked alongside provided proof
        //      verify_proof(leaf, merkle_proof, merkle_root)
        //
        // Mark as claimed and send NEAR to account
        self.claims.insert(user_account_id);
    }

    pub fn withdraw(&mut self) {
        let caller = env::predecessor_account_id();

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
