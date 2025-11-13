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
