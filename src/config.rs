use crate::*;
use near_sdk::json_types::U64;
use near_sdk::store::{LookupMap, Vector};
use near_sdk::json_types::Base58CryptoHash;
use near_sdk::AccountId;


#[derive(Debug, Clone)]
#[near(serializers=[borsh, json])]
pub struct Config {
    /// The owner of the contract who can withdraw the remaining token balance
    pub owner_account_id: AccountId,

    /// A map of accounts who have claimed or have yet to claim their NEAR rewards
    pub claims: U64,

    /// The merkle root of the tree containing the rewards for each account_id
    pub merkle_root: Base58CryptoHash,

    /// The timestamp for when the claim period has concluded
    pub claim_end: U64
}

#[near]
impl Contract {
    /// Returns the current contract configuration.
    pub fn get_config(&self) -> &Config {
        &self.config
    }
}
