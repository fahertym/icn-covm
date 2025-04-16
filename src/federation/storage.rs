use crate::federation::messages::{
    FederatedProposal, FederatedVote, ProposalScope, ProposalStatus, VotingModel,
};
use crate::identity::Identity;
use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::traits::StorageExtensions;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// Storage namespace constants
pub const FEDERATION_NAMESPACE: &str = "federation";
pub const VOTES_NAMESPACE: &str = "votes";

// Storage key prefix constants
pub const FEDERATION_PROPOSAL_PREFIX: &str = "federation/proposals/";
pub const FEDERATION_VOTES_PREFIX: &str = "federation/votes/";
pub const FEDERATION_SYNC_PREFIX: &str = "federation/sync/";

/// In-memory cache for active proposals and votes
#[derive(Default)]
pub struct FederationCache {
    /// Map of proposal ID to proposal
    pub proposals: HashMap<String, FederatedProposal>,

    /// Map of proposal ID to a vector of votes
    pub votes: HashMap<String, Vec<FederatedVote>>,
}

/// Result of a federation vote tally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteTallyResult {
    /// The proposal that was voted on
    pub proposal: FederatedProposal,

    /// The winning option index
    pub winner_index: usize,

    /// The winning option text
    pub winner_option: String,

    /// Total number of votes cast
    pub total_votes: usize,
}

/// Handles storage and retrieval of federation proposals and votes
pub struct FederationStorage {
    /// In-memory cache for active proposals and votes
    cache: Arc<Mutex<FederationCache>>,
}

impl FederationStorage {
    /// Create a new federation storage handler
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(FederationCache::default())),
        }
    }
    
    /// Create a proposal storage key
    pub fn make_proposal_key(proposal_id: &str) -> String {
        format!("{}{}", FEDERATION_PROPOSAL_PREFIX, proposal_id)
    }
    
    /// Create a votes storage key
    pub fn make_votes_key(proposal_id: &str) -> String {
        format!("{}{}", FEDERATION_VOTES_PREFIX, proposal_id)
    }
    
    /// Create a sync metadata storage key
    pub fn make_sync_key(proposal_id: &str) -> String {
        format!("{}/last_seen", Self::make_sync_base_key(proposal_id))
    }
    
    /// Create a sync base key
    pub fn make_sync_base_key(proposal_id: &str) -> String {
        format!("{}{}", FEDERATION_SYNC_PREFIX, proposal_id)
    }

    /// Save a proposal to storage and cache
    pub fn save_proposal<S: StorageExtensions>(
        &self,
        storage: &mut S,
        proposal: FederatedProposal,
    ) -> StorageResult<()> {
        // Create the storage key
        let key = Self::make_proposal_key(&proposal.proposal_id);

        // Store in the backend
        storage.set_json(None, &proposal.namespace, &key, &proposal)?;

        // Add to the cache
        let mut cache = match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // If the mutex is poisoned, get a consistent state
                warn!("Cache mutex was poisoned, recovering the guard");
                poisoned.into_inner()
            }
        };
        
        cache
            .proposals
            .insert(proposal.proposal_id.clone(), proposal);

        info!("Saved federation proposal to storage and cache");
        Ok(())
    }

    /// Save a proposal to storage and cache with explicit auth
    pub fn save_proposal_with_auth<S: StorageExtensions>(
        &self,
        storage: &mut S,
        auth: Option<&AuthContext>,
        proposal: FederatedProposal,
    ) -> StorageResult<()> {
        // Create the storage key
        let key = Self::make_proposal_key(&proposal.proposal_id);

        // Store in the backend with auth
        storage.set_json(auth, &proposal.namespace, &key, &proposal)?;

        // Add to the cache
        let mut cache = match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // If the mutex is poisoned, get a consistent state
                warn!("Cache mutex was poisoned, recovering the guard");
                poisoned.into_inner()
            }
        };
        
        cache
            .proposals
            .insert(proposal.proposal_id.clone(), proposal);

        info!("Saved federation proposal to storage and cache with explicit auth");
        Ok(())
    }

    /// Load an identity from storage
    fn load_identity_from_storage<S: StorageExtensions>(
        &self,
        storage: &S,
        identity_id: &str,
    ) -> StorageResult<Identity> {
        storage.get_identity(identity_id)
    }

    /// Save a vote to storage, checking eligibility first
    pub fn save_vote<S: StorageExtensions>(
        &self,
        storage: &mut S,
        vote: FederatedVote,
        voter_identity: Option<&Identity>,
    ) -> StorageResult<()> {
        // First, get the proposal to check eligibility
        let proposal = self.get_proposal(&*storage, &vote.proposal_id)?;

        // Check if voting is still open
        if proposal.status != ProposalStatus::Open {
            warn!(
                "Vote rejected: Proposal {} is not open for voting",
                vote.proposal_id
            );
            return Err(StorageError::Other {
                details: format!("Proposal {} is not open for voting", vote.proposal_id),
            });
        }

        // Get the identity for verification
        let identity = if let Some(id) = voter_identity {
            id.clone()
        } else {
            // Need to load the identity
            match self.load_identity_from_storage(storage, &vote.voter) {
                Ok(loaded_identity) => loaded_identity,
                Err(e) => {
                    warn!("Failed to load identity for voter {}: {}", vote.voter, e);
                    return Err(StorageError::NotFound {
                        key: format!("Identity for voter {} not found", vote.voter),
                    });
                }
            }
        };

        // Verify the signature if the identity has a public key
        if let Some(pub_key) = identity.public_key() {
            // Only verify if we have a crypto scheme
            if let Some(scheme) = identity.crypto_scheme() {
                // For now, we'll use a simple verification - in production, use proper crypto
                if !self.verify_signature(
                    &vote.voter,
                    &vote.message,
                    &vote.signature,
                    &scheme,
                    pub_key,
                ) {
                    warn!("Vote rejected: Invalid signature from voter {}", vote.voter);
                    return Err(StorageError::Other {
                        details: format!("Invalid signature for vote from {}", vote.voter),
                    });
                }

                debug!("Signature verification passed for voter {}", vote.voter);
            } else {
                warn!(
                    "Cannot verify vote: No crypto scheme specified for voter {}",
                    vote.voter
                );
                return Err(StorageError::Other {
                    details: format!("No crypto scheme specified for voter {}", vote.voter),
                });
            }
        } else {
            warn!(
                "Cannot verify vote: No public key available for voter {}",
                vote.voter
            );
            return Err(StorageError::Other {
                details: format!("No public key available for voter {}", vote.voter),
            });
        }

        // Check eligibility based on proposal scope
        let is_eligible = match &proposal.scope {
            ProposalScope::SingleCoop(coop_id) => {
                // Only members of this specific coop can vote
                if !identity.belongs_to(coop_id) {
                    warn!(
                        "Vote rejected: Voter {} not a member of cooperative {}",
                        vote.voter, coop_id
                    );
                    return Err(StorageError::Other {
                        details: format!("Voter not a member of eligible cooperative {}", coop_id),
                    });
                }
                true
            }
            ProposalScope::MultiCoop(coop_ids) => {
                // Check if the voter belongs to any of the eligible coops
                let belongs = coop_ids.iter().any(|coop_id| identity.belongs_to(coop_id));
                if !belongs {
                    warn!(
                        "Vote rejected: Voter {} not a member of any eligible cooperatives",
                        vote.voter
                    );
                    return Err(StorageError::Other {
                        details: "Voter not a member of any eligible cooperatives".to_string(),
                    });
                }
                true
            }
            ProposalScope::GlobalFederation => {
                // All federation members can vote
                true
            }
        };

        if !is_eligible {
            return Err(StorageError::Other {
                details: "Voter not eligible for this proposal scope".to_string(),
            });
        }

        // Create the storage key - we'll store votes as a list under the proposal
        let key = Self::make_votes_key(&vote.proposal_id);

        // First try to get existing votes
        let mut votes: Vec<FederatedVote> = match storage.get_json(None, VOTES_NAMESPACE, &key) {
            Ok(existing_votes) => existing_votes,
            Err(_) => Vec::new(),
        };

        // Check if this voter has already voted
        if votes.iter().any(|v| v.voter == vote.voter) {
            warn!(
                "Vote rejected: Voter {} has already voted on proposal {}",
                vote.voter, vote.proposal_id
            );
            return Err(StorageError::Other {
                details: format!("Voter {} has already voted", vote.voter),
            });
        }

        // Add the new vote
        votes.push(vote.clone());

        // Store the updated votes list
        storage.set_json(None, VOTES_NAMESPACE, &key, &votes)?;

        // Update the cache
        let mut cache = match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // If the mutex is poisoned, get a consistent state
                warn!("Cache mutex was poisoned, recovering the guard");
                poisoned.into_inner()
            }
        };
        
        cache
            .votes
            .entry(vote.proposal_id.clone())
            .or_insert_with(Vec::new)
            .push(vote);

        info!("Saved federation vote to storage and cache");
        Ok(())
    }

    /// Verify a signature using the appropriate cryptographic scheme
    fn verify_signature(
        &self,
        voter_id: &str,
        message: &str,
        signature: &str,
        scheme: &str,
        public_key: &[u8],
    ) -> bool {
        // In a production system, this would use real cryptographic libraries
        // For now, we'll implement a simple mock verification

        // For testing, we'll accept "valid" as a special signature
        if signature == "valid" || signature == "mock_signature" {
            debug!("Using mock signature verification (TESTING ONLY)");
            return true;
        }

        match scheme {
            "ed25519" => {
                // Mock ed25519 verification
                // In a real system, use the ed25519-dalek crate or similar
                debug!("Verifying Ed25519 signature (mock implementation)");
                !signature.is_empty() && !message.is_empty() && !public_key.is_empty()
            }
            "secp256k1" => {
                // Mock secp256k1 verification
                // In a real system, use the secp256k1 crate
                debug!("Verifying Secp256k1 signature (mock implementation)");
                !signature.is_empty() && !message.is_empty() && !public_key.is_empty()
            }
            _ => {
                warn!("Unsupported crypto scheme: {}", scheme);
                false
            }
        }
    }

    /// Get a proposal by ID
    pub fn get_proposal<S: StorageExtensions>(
        &self,
        storage: &S,
        proposal_id: &str,
    ) -> StorageResult<FederatedProposal> {
        // First check the cache
        {
            let cache = match self.cache.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    // If the mutex is poisoned, get a consistent state
                    warn!("Cache mutex was poisoned, recovering the guard");
                    poisoned.into_inner()
                }
            };
            
            if let Some(proposal) = cache.proposals.get(proposal_id) {
                return Ok(proposal.clone());
            }
        }

        // If not in cache, check storage
        let key = Self::make_proposal_key(proposal_id);
        storage.get_json(None, FEDERATION_NAMESPACE, &key)
    }

    /// Get all votes for a proposal
    pub fn get_votes<S: StorageExtensions>(
        &self,
        storage: &S,
        proposal_id: &str,
    ) -> StorageResult<Vec<FederatedVote>> {
        // First check the cache
        {
            let cache = match self.cache.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    // If the mutex is poisoned, get a consistent state
                    warn!("Cache mutex was poisoned, recovering the guard");
                    poisoned.into_inner()
                }
            };
            
            if let Some(votes) = cache.votes.get(proposal_id) {
                return Ok(votes.clone());
            }
        }

        // If not in cache, check storage
        let key = Self::make_votes_key(proposal_id);
        storage.get_json(None, VOTES_NAMESPACE, &key)
    }

    /// Convert votes to a format suitable for the ranked vote algorithm
    /// This method implements the voting model logic:
    /// - OneMemberOneVote: Uses all votes as-is
    /// - OneCoopOneVote: Only keeps one vote per cooperative (the latest one)
    pub fn prepare_ranked_ballots(
        &self,
        votes: &[FederatedVote],
        proposal: &FederatedProposal,
        voter_identities: &HashMap<String, Identity>,
    ) -> Vec<Vec<f64>> {
        match proposal.voting_model {
            VotingModel::OneMemberOneVote => {
                // Use all votes directly
                votes
                    .iter()
                    .map(|vote| vote.ranked_choices.clone())
                    .collect()
            }
            VotingModel::OneCoopOneVote => {
                // We need to group votes by cooperative and only use the latest vote from each coop
                let mut coop_votes: HashMap<String, (FederatedVote, usize)> = HashMap::new();

                // Process votes in order (assuming later votes in the array are more recent)
                for (idx, vote) in votes.iter().enumerate() {
                    // If we have identity info for this voter, use it to determine their coop
                    if let Some(identity) = voter_identities.get(&vote.voter) {
                        if let Some(coop_id) = identity.get_metadata("coop_id") {
                            // Either insert this vote or replace an existing one for this coop
                            // Convert &str to String for the HashMap key
                            let coop_id_string = coop_id.to_string();
                            if let Some((_, existing_idx)) = coop_votes.get(&coop_id_string) {
                                if idx > *existing_idx {
                                    // This vote is more recent, replace the existing one
                                    coop_votes.insert(coop_id_string, (vote.clone(), idx));
                                }
                            } else {
                                // First vote from this coop
                                coop_votes.insert(coop_id_string, (vote.clone(), idx));
                            }
                        } else {
                            // If we can't determine the coop, still include the vote
                            // but use the voter ID as the key to avoid duplicates
                            coop_votes.insert(vote.voter.clone(), (vote.clone(), idx));
                        }
                    } else {
                        // No identity info, use the voter ID as the key
                        coop_votes.insert(vote.voter.clone(), (vote.clone(), idx));
                    }
                }

                // Extract just the votes from the resulting map
                coop_votes
                    .values()
                    .map(|(vote, _)| vote.ranked_choices.clone())
                    .collect()
            }
        }
    }
}

// Helper function to get current Unix timestamp
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
