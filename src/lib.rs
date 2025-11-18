mod config;
mod merkle;

use crate::config::Config;
use near_sdk::json_types::U64;
use near_sdk::store::{LookupMap, LookupSet};
use near_sdk::{
    borsh, env, near, require, serde_json, AccountId, BorshStorageKey, CryptoHash, NearToken,
    PanicOnDefault, Promise,
};

use near_sdk::serde::Serialize;

/// Raw type for balance in yocto NEAR.
pub type Balance = u128;
/// Raw type for unique identifier for campaigns
pub type CampaignId = u32;

#[derive(BorshStorageKey)]
#[near]
enum StorageKeys {
    Claims,
    Campaigns,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone)]
#[near(serializers=[borsh])]
struct MerkleTreeData {
    account: String,
    lockup: String,
    amount: Balance,
}

#[derive(Clone)]
#[near(serializers=[borsh,json])]
pub struct RewardCampaign {
    /// The unique identifier of the campaign, this is generated automatically.
    pub id: CampaignId,
    /// The timestamp that starts the claim period, this is generated automatically
    pub claim_start: U64,
    /// The timestamp for when the claim period has concluded
    pub claim_end: U64,
    /// The merkle root of the tree containing the rewards for each account_id
    pub merkle_root: CryptoHash,
}

// Define the contract structure
#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct MerkleClaim {
    config: Config,
    /// A set of accounts who have claimed or have yet to claim their NEAR rewards where the key is
    /// a hash of the campaign_id & account_id
    claims: LookupSet<CryptoHash>,
    /// A map all the reward campaings
    campaigns: LookupMap<CampaignId, RewardCampaign>,
    /// The last campaign_id generated
    last_campaign_id: CampaignId,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct CampaignCreatedEvent {
    pub campaign_id: CampaignId,
    pub merkle_root: CryptoHash,
    pub claim_end: U64,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ClaimEvent {
    pub campaign_id: CampaignId,
    pub account_id: AccountId,
    pub lockup_contract: AccountId,
    pub amount: Balance,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct WithdrawEvent {
    pub balance: NearToken,
    pub withdrawn: NearToken,
}

// Implement the contract structure
#[near(serializers=[borsh])]
impl MerkleClaim {
    /// Initializes the contract with the given configuration.
    #[init]
    #[payable]
    pub fn new(config: Config) -> Self {
        let amount = env::attached_deposit();

        let min_balance = config.min_storage_deposit;

        if amount < min_balance {
            env::panic_str("The attached deposit is less than the minimum storage balance");
        }

        // Send more than the min deposit back to the owner
        let refund = amount.saturating_sub(min_balance);

        if refund > NearToken::from_near(0) {
            Promise::new(env::predecessor_account_id()).transfer(refund);
        }

        Self {
            config,
            claims: LookupSet::new(StorageKeys::Claims),
            campaigns: LookupMap::new(StorageKeys::Campaigns),
            last_campaign_id: 0,
        }
    }

    pub fn assert_owner(&self) {
        require!(
            env::predecessor_account_id() == self.config.owner_account_id,
            "Only the owner can call this method"
        );
    }

    pub fn create_campaign(&mut self, merkle_root: CryptoHash, claim_end: U64) {
        self.assert_owner();

        require!(
            env::block_timestamp() < claim_end.into(),
            "Claim end timestamp must be some time in the future"
        );

        let campaign_id = self.last_campaign_id + 1;

        let campaign = RewardCampaign {
            id: campaign_id,
            claim_start: env::block_timestamp().into(),
            claim_end,
            merkle_root,
        };

        self.campaigns.insert(campaign_id, campaign.into());
        self.last_campaign_id = self
            .last_campaign_id
            .checked_add(1)
            .expect("Campaign id value overflows");

        let create = CampaignCreatedEvent {
            campaign_id,
            merkle_root,
            claim_end,
        };

        env::log_str(&serde_json::to_string(&create).unwrap());
    }

    pub fn claim(
        &mut self,
        amount: near_sdk::json_types::U128,
        merkle_proof: Vec<CryptoHash>,
        campaign_id: CampaignId,
        lockup_contract: AccountId,
    ) {
        let user_account_id = env::predecessor_account_id();
        let key = env::keccak256_array(
            &[
                user_account_id.as_bytes().to_vec(),
                campaign_id.to_ne_bytes().to_vec(),
            ]
            .concat(),
        );

        // Check claim parameters
        require!(amount.0 > 0, "Amount must not be zero");
        require!(
            self.campaigns.contains_key(&campaign_id) == true,
            "Campaign does not exist"
        );
        require!(!self.claims.contains(&key), "Already claimed rewards");

        require!(merkle_proof.len() > 0, "Merkle proof supplied is empty");

        let selected_campaign = self.campaigns.get(&campaign_id).unwrap();

        require!(
            env::block_timestamp() < selected_campaign.claim_end.into(),
            "Claim period has concluded"
        );

        // Calculate leaf to be checked alongside provided proof
        let data = MerkleTreeData {
            account: user_account_id.to_string(),
            lockup: lockup_contract.to_string(),
            amount: amount.0,
        };

        let serialized_data: Vec<u8> = borsh::to_vec(&data).expect("Failed to serialize data");
        let leaf = env::keccak256_array(&serialized_data);

        require!(
            Self::verify_proof(leaf, merkle_proof, selected_campaign.merkle_root),
            "Invalid Proof"
        );

        // Mark as claimed and send NEAR to account
        self.claims.insert(key);
        Promise::new(lockup_contract.clone()).transfer(NearToken::from_yoctonear(amount.0));

        let claim = ClaimEvent {
            campaign_id,
            account_id: user_account_id,
            lockup_contract,
            amount: amount.0,
        };

        env::log_str(&serde_json::to_string(&claim).unwrap());
    }

    pub fn withdraw(&mut self) {
        self.assert_owner();
        let available_balance =
            env::account_balance().saturating_sub(self.config.min_storage_deposit);

        if available_balance > NearToken::from_near(0) {
            Promise::new(env::predecessor_account_id()).transfer(available_balance);

            let withdraw = WithdrawEvent {
                balance: env::account_balance(),
                withdrawn: available_balance,
            };

            env::log_str(&serde_json::to_string(&withdraw).unwrap());
        } else {
            env::panic_str("The remaining balance is required for contract storage");
        }
    }

    pub fn get_campaign(&self, campaign_id: CampaignId) -> Option<RewardCampaign> {
        self.campaigns.get(&campaign_id).cloned()
    }

    pub fn has_claimed(&self, campaign_id: CampaignId, account_id: AccountId) -> bool {
        let key = env::keccak256_array(
            &[
                account_id.as_bytes().to_vec(),
                campaign_id.to_ne_bytes().to_vec(),
            ]
            .concat(),
        );

        self.claims.contains(&key)
    }

    pub fn get_last_campaign_id(&self) -> CampaignId {
        self.last_campaign_id
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use near_sdk::{json_types, testing_env, AccountId, NearToken, VMContext};
    use std::convert::TryInto;
    use std::str::FromStr;
    use test_utils::*;

    use super::*;

    mod test_utils;

    const MIN_STORAGE_DEPOSIT: NearToken = NearToken::from_yoctonear(1000);
    const FAKE_MERKLE_PROOF: [[u8; 32]; 2] = [
        [
            94, 143, 161, 184, 186, 17, 223, 110, 197, 156, 168, 41, 145, 20, 196, 193, 228, 159,
            17, 221, 180, 108, 13, 100, 7, 247, 235, 212, 6, 162, 245, 2,
        ],
        [
            236, 8, 150, 63, 117, 228, 0, 201, 219, 131, 238, 26, 123, 68, 157, 32, 157, 222, 174,
            86, 151, 188, 49, 178, 51, 171, 115, 173, 101, 12, 43, 15,
        ],
    ];

    fn basic_context() -> VMContext {
        get_context(system_account(), to_ts(GENESIS_TIME_IN_DAYS))
    }

    fn claims_contract_setup() -> (VMContext, MerkleClaim) {
        let context = basic_context();
        testing_env!(context.clone());

        let config = Config {
            owner_account_id: account_owner(),
            min_storage_deposit: MIN_STORAGE_DEPOSIT,
        };

        let contract = MerkleClaim::new(config);

        (context, contract)
    }

    fn build_mock_campaign() -> (u32, CryptoHash, U64) {
        let data = MerkleTreeData {
            account: account_owner().to_string(),
            lockup: system_account().to_string(),
            amount: 0,
        };

        let serialized_data: Vec<u8> = borsh::to_vec(&data).expect("Failed to serialize data");
        let root = env::keccak256_array(&serialized_data);
        let end = json_types::U64(to_ts(GENESIS_TIME_IN_DAYS + 30u64));
        let campaign_id = 1u32;

        (campaign_id, root, end)
    }

    #[test]
    fn test_campaign_creation_success() {
        let (mut context, mut contract) = claims_contract_setup();

        context.predecessor_account_id = account_owner();
        context.signer_account_id = account_owner();
        context.signer_account_pk = public_key(1).try_into().unwrap();
        testing_env!(context.clone());

        let mock_campaign = build_mock_campaign();

        contract.create_campaign(mock_campaign.1, mock_campaign.2);

        let current_campaign = contract.get_campaign(mock_campaign.0).unwrap();

        assert_eq!(current_campaign.id, mock_campaign.0);
        assert_eq!(current_campaign.merkle_root, mock_campaign.1);
        assert_eq!(
            current_campaign.claim_start,
            json_types::U64(to_ts(GENESIS_TIME_IN_DAYS))
        );
        assert_eq!(current_campaign.claim_end, mock_campaign.2);
    }

    #[test]
    #[should_panic(expected = "Only the owner can call this method")]
    fn test_campaign_creation_failure_non_owner() {
        let (mut context, mut contract) = claims_contract_setup();
        context.predecessor_account_id = non_owner();
        context.signer_account_id = non_owner();
        context.signer_account_pk = public_key(2).try_into().unwrap();
        context.attached_deposit = NearToken::from_yoctonear(1);

        testing_env!(context.clone());
        let mock_campaign = build_mock_campaign();

        contract.create_campaign(mock_campaign.1, mock_campaign.2);
    }

    #[test]
    #[should_panic(expected = "Claim end timestamp must be some time in the future")]
    fn test_campaign_creation_failure_claim_end() {
        let (mut context, mut contract) = claims_contract_setup();
        context.predecessor_account_id = account_owner();
        context.signer_account_id = account_owner();
        context.signer_account_pk = public_key(1).try_into().unwrap();

        let mock_campaign = build_mock_campaign();
        // Change the block timestamp to be the claim end period
        context.block_timestamp = mock_campaign.2.into();

        testing_env!(context.clone());
        contract.create_campaign(mock_campaign.1, mock_campaign.2);
    }

    #[test]
    #[should_panic(expected = "Invalid Proof")]
    fn test_claim_invalid_proof() {
        let (mut context, mut contract) = claims_contract_setup();

        context.predecessor_account_id = account_owner();
        context.signer_account_id = account_owner();
        context.signer_account_pk = public_key(1).try_into().unwrap();
        testing_env!(context.clone());
        let end = json_types::U64(to_ts(GENESIS_TIME_IN_DAYS + 30u64));

        contract.create_campaign(
            [
                158, 236, 219, 170, 25, 1, 253, 172, 46, 71, 82, 30, 201, 181, 15, 59, 58, 254,
                170, 207, 59, 87, 184, 46, 81, 28, 122, 202, 227, 92, 92, 128,
            ],
            end,
        );

        context.predecessor_account_id = claimant();
        context.signer_account_id = claimant();
        context.signer_account_pk = public_key(123).try_into().unwrap();
        testing_env!(context.clone());

        contract.claim(
            json_types::U128(1000u128),
            FAKE_MERKLE_PROOF.to_vec(),
            1u32,
            AccountId::from_str("lockup-contract").unwrap(),
        );
    }

    #[test]
    #[should_panic(expected = "Amount must not be zero")]
    fn test_claim_amount_failure() {
        let (mut context, mut contract) = claims_contract_setup();

        context.predecessor_account_id = account_owner();
        context.signer_account_id = account_owner();
        context.signer_account_pk = public_key(1).try_into().unwrap();
        testing_env!(context.clone());

        let mock_campaign = build_mock_campaign();

        contract.create_campaign(mock_campaign.1, mock_campaign.2);

        context.predecessor_account_id = claimant();
        context.signer_account_id = claimant();
        context.signer_account_pk = public_key(123).try_into().unwrap();
        testing_env!(context.clone());

        contract.claim(
            json_types::U128(0u128),
            FAKE_MERKLE_PROOF.to_vec(),
            1u32,
            AccountId::from_str("lockup-contract").unwrap(),
        );
    }

    #[test]
    #[should_panic(expected = "Campaign does not exist")]
    fn test_claim_campaign_failure() {
        let (mut context, mut contract) = claims_contract_setup();

        context.predecessor_account_id = account_owner();
        context.signer_account_id = account_owner();
        context.signer_account_pk = public_key(1).try_into().unwrap();
        testing_env!(context.clone());

        let mock_campaign = build_mock_campaign();

        contract.create_campaign(mock_campaign.1, mock_campaign.2);

        context.predecessor_account_id = claimant();
        context.signer_account_id = claimant();
        context.signer_account_pk = public_key(123).try_into().unwrap();
        testing_env!(context.clone());

        contract.claim(
            json_types::U128(1000u128),
            FAKE_MERKLE_PROOF.to_vec(),
            2u32,
            AccountId::from_str("lockup-contract").unwrap(),
        );
    }

    #[test]
    #[should_panic(expected = "Merkle proof supplied is empty")]
    fn test_claim_proof_empty_failure() {
        let (mut context, mut contract) = claims_contract_setup();

        context.predecessor_account_id = account_owner();
        context.signer_account_id = account_owner();
        context.signer_account_pk = public_key(1).try_into().unwrap();
        testing_env!(context.clone());

        let mock_campaign = build_mock_campaign();

        contract.create_campaign(mock_campaign.1, mock_campaign.2);

        context.predecessor_account_id = claimant();
        context.signer_account_id = claimant();
        context.signer_account_pk = public_key(123).try_into().unwrap();
        testing_env!(context.clone());

        contract.claim(
            json_types::U128(1000u128),
            [].to_vec(),
            1u32,
            AccountId::from_str("lockup-contract").unwrap(),
        );
    }

    #[test]
    #[should_panic(expected = "Claim period has concluded")]
    fn test_claim_end_failure() {
        let (mut context, mut contract) = claims_contract_setup();

        context.predecessor_account_id = account_owner();
        context.signer_account_id = account_owner();
        context.signer_account_pk = public_key(1).try_into().unwrap();
        testing_env!(context.clone());

        let mock_campaign = build_mock_campaign();

        contract.create_campaign(mock_campaign.1, mock_campaign.2);

        context.predecessor_account_id = claimant();
        context.signer_account_id = claimant();
        context.signer_account_pk = public_key(123).try_into().unwrap();
        context.block_timestamp = to_ts(GENESIS_TIME_IN_DAYS + 40u64);
        testing_env!(context.clone());

        contract.claim(
            json_types::U128(1000u128),
            FAKE_MERKLE_PROOF.to_vec(),
            1u32,
            AccountId::from_str("lockup-contract").unwrap(),
        );
    }

    #[test]
    #[should_panic(expected = "Only the owner can call this method")]
    fn test_withdraw_owner_failure() {
        let (mut context, mut contract) = claims_contract_setup();

        context.predecessor_account_id = non_owner();
        context.signer_account_id = non_owner();
        context.signer_account_pk = public_key(1).try_into().unwrap();
        testing_env!(context.clone());

        contract.withdraw();
    }
}
