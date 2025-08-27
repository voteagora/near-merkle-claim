use crate::*;
use near_sdk::{assert_one_yocto, near, require};
use near_sdk::CryptoHash;


impl MerkleClaim {
    pub fn verify_proof(&self) {
        assert_one_yocto();
    }
}
