mod config;
mod merkle;

use crate::config::Config;
use near_sdk::json_types::U64;
use near_sdk::store::LookupMap;
use near_sdk::{
    borsh, env, near, require, AccountId, BorshStorageKey, CryptoHash, NearToken, PanicOnDefault,
    Promise,
};

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

#[near(serializers=[borsh])]
struct RewardCampaign {
    /// The unique identifier of the campaign, this is generated automatically.
    id: CampaignId,
    /// The timestamp that starts the claim period, this is generated automatically
    claim_start: U64,
    /// The timestamp for when the claim period has concluded
    claim_end: U64,
    /// The merkle root of the tree containing the rewards for each account_id
    merkle_root: CryptoHash,
}

// Define the contract structure
#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct MerkleClaim {
    config: Config,
    /// A map of accounts who have claimed or have yet to claim their NEAR rewards
    claims: LookupMap<CampaignId, AccountId>,
    /// A map all the reward campaings
    campaigns: LookupMap<CampaignId, RewardCampaign>,
    /// The last campaign_id generated
    last_campaign_id: CampaignId,
}

// Implement the contract structure
#[near(serializers=[borsh])]
impl MerkleClaim {
    /// Initializes the contract with the given configuration.
    #[init]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            claims: LookupMap::new(StorageKeys::Claims),
            campaigns: LookupMap::new(StorageKeys::Campaigns),
            last_campaign_id: 0,
        }
    }

    pub fn create_campaign(&mut self, merkle_root: CryptoHash, claim_end: U64) {
        require!(
            env::predecessor_account_id() == self.config.owner_account_id,
            "Account must be the owner"
        );
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
        self.last_campaign_id += 1;
    }

    pub fn claim(
        &mut self,
        amount: near_sdk::json_types::U128,
        merkle_proof: Vec<CryptoHash>,
        campaign_id: CampaignId,
        lockup_contract: AccountId,
    ) {
        let user_account_id = env::predecessor_account_id();
        // Check claim parameters
        require!(amount.0 > 0, "Amount must not be zero");
        require!(
            self.campaigns.contains_key(&campaign_id) == true,
            "Campaign does not exist"
        );
        require!(
            self.claims.get(&campaign_id) != None,
            "Already claimed rewards"
        );
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
        self.claims
            .insert(campaign_id, env::predecessor_account_id());
        Promise::new(lockup_contract).transfer(NearToken::from_yoctonear(amount.0));
    }

    pub fn withdraw(&mut self) {
        let caller = env::predecessor_account_id();

        require!(
            caller == self.config.owner_account_id,
            "Caller must be the owner of the claims contract"
        );

        // Send total balance to the owner
        Promise::new(caller).transfer(env::account_balance());
    }
}
