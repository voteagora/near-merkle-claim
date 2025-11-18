use near_sdk::{AccountId, Gas, NearToken, PublicKey, VMContext};
use std::str::FromStr;

pub const GENESIS_TIME_IN_DAYS: u64 = 500;
pub const DEFAULT_BALANCE_YOCTO: u128 = 20000;

pub fn system_account() -> AccountId {
    AccountId::from_str("system_account").unwrap()
}

pub fn account_owner() -> AccountId {
    AccountId::from_str("account_owner").unwrap()
}

pub fn non_owner() -> AccountId {
    AccountId::from_str("non_owner").unwrap()
}

pub fn claimant() -> AccountId {
    AccountId::from_str("claimant").unwrap()
}

pub fn to_nanos(num_days: u64) -> u64 {
    num_days * 86400_000_000_000
}

pub fn to_ts(num_days: u64) -> u64 {
    // 2018-08-01 UTC in nanoseconds
    1533081600_000_000_000 + to_nanos(num_days)
}

pub fn get_context(predecessor_account_id: AccountId, block_timestamp: u64) -> VMContext {
    VMContext {
        current_account_id: account_owner(),
        signer_account_id: predecessor_account_id.clone(),
        signer_account_pk: public_key(123),
        predecessor_account_id,
        input: vec![],
        block_index: 1,
        block_timestamp,
        epoch_height: 1,
        account_balance: NearToken::from_yoctonear(DEFAULT_BALANCE_YOCTO),
        account_locked_balance: NearToken::from_yoctonear(DEFAULT_BALANCE_YOCTO),
        storage_usage: 10u64.pow(6),
        attached_deposit: NearToken::from_yoctonear(1000),
        prepaid_gas: Gas::from_gas(10u64.pow(15)),
        random_seed: [37u8; 32],
        output_data_receivers: vec![],
        view_config: None,
    }
}

pub fn public_key(byte_val: u8) -> PublicKey {
    let mut pk = vec![byte_val; 33];
    pk[0] = 0;
    PublicKey::try_from(pk).unwrap()
}
