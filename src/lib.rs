mod config;

use crate::config::Config;
use near_sdk::{log, near, require, AccountId, PanicOnDefault};


// Define the contract structure
#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct Contract {
    config: Config
}

// Implement the contract structure
#[near]
impl Contract {
    /// Initializes the contract with the given configuration.
    #[init]
    pub fn new(config: Config) -> Self {
        Self {
            config
        }
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
