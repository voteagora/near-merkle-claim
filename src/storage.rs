use crate::*;
use near_sdk::Promise;

#[near]
impl MerkleClaim {
    #[payable]
    pub fn storage_deposit(&mut self) {
        self.assert_owner();
        let amount = env::attached_deposit();

        let min_balance = self.config.min_storage_deposit;

        if amount < min_balance {
            env::panic_str("The attached deposit is less than the minimum storage balance");
        }

        // Send more than the min deposit back to the owner
        let refund = amount.saturating_sub(min_balance);

        if refund > NearToken::from_near(0) {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }
    }
}
