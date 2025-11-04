use crate::*;
use near_sdk::{AccountId, NearToken};

#[derive(Debug, Clone)]
#[near(serializers=[borsh, json])]
pub struct Config {
    /// The owner of the contract who can withdraw the remaining token balance
    pub owner_account_id: AccountId,

    /// The minimum amount in NEAR required for storage
    pub min_storage_deposit: NearToken,
}

#[near]
impl MerkleClaim {
    /// Returns the current contract configuration.
    pub fn get_config(&self) -> &Config {
        &self.config
    }
}
