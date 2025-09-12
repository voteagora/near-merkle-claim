use crate::*;
use near_sdk::env::keccak256_array;
use near_sdk::CryptoHash;

impl MerkleClaim {
    pub fn verify_proof(
        leaf: CryptoHash,
        merkle_proof: Vec<CryptoHash>,
        merkle_root: CryptoHash,
    ) -> bool {
        let mut computed_hash = leaf;

        for hash in merkle_proof {
            computed_hash = keccak256_array(&Self::commutative_keccak256(&computed_hash, &hash));
        }

        merkle_root == computed_hash
    }

    fn commutative_keccak256(a: &CryptoHash, b: &CryptoHash) -> Vec<u8> {
        if a < b {
            [a.as_slice(), b.as_slice()].concat()
        } else {
            [b.as_slice(), a.as_slice()].concat()
        }
    }
}
