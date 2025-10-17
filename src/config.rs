use crate::*;
use near_sdk::AccountId;

#[derive(Debug, Clone)]
#[near(serializers=[borsh, json])]
pub struct Config {
    /// The owner of the contract who can withdraw the remaining token balance
    pub owner_account_id: AccountId,
}

#[near]
impl MerkleClaim {
    /// Returns the current contract configuration.
    pub fn get_config(&self) -> &Config {
        &self.config
    }
}
