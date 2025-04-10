use crate::federation::messages::{FederatedProposal, FederatedVote, ProposalScope};
use crate::storage::traits::StorageExtensions;
use crate::storage::errors::{StorageResult, StorageError};
use crate::identity::Identity;
use serde::{Serialize, Deserialize};
use log::{debug, info, warn, error};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const FEDERATION_PROPOSAL_PREFIX: &str = "federation/proposals/";
const FEDERATION_VOTES_PREFIX: &str = "federation/votes/";

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
    
    /// Save a proposal to storage and cache
    pub fn save_proposal<S: StorageExtensions>(
        &self, 
        storage: &mut S, 
        proposal: FederatedProposal
    ) -> StorageResult<()> {
        // Create the storage key
        let key = format!("{}{}", FEDERATION_PROPOSAL_PREFIX, proposal.proposal_id);
        
        // Store in the backend
        storage.set_json(None, &proposal.namespace, &key, &proposal)?;
        
        // Add to the cache
        let mut cache = self.cache.lock().unwrap();
        cache.proposals.insert(proposal.proposal_id.clone(), proposal);
        
        info!("Saved federation proposal to storage and cache");
        Ok(())
    }
    
    /// Add a vote to a proposal
    pub fn save_vote<S: StorageExtensions>(
        &self,
        storage: &mut S,
        vote: FederatedVote,
        voter_identity: Option<&Identity>
    ) -> StorageResult<()> {
        // First, get the proposal to check scope-based eligibility
        let proposal = self.get_proposal(storage, &vote.proposal_id)?;
        
        // Check eligibility based on proposal scope if we have voter identity information
        if let Some(identity) = voter_identity {
            let is_eligible = match &proposal.scope {
                ProposalScope::SingleCoop(coop_id) => {
                    // Only members of this specific coop can vote
                    if !identity.belongs_to(coop_id) {
                        warn!("Vote rejected: Voter {} not a member of cooperative {}", 
                            vote.voter, coop_id);
                        return Err(StorageError::Other { 
                            details: format!("Voter not a member of eligible cooperative {}", coop_id) 
                        });
                    }
                    true
                },
                ProposalScope::MultiCoop(coop_ids) => {
                    // Check if the voter belongs to any of the eligible coops
                    let belongs = coop_ids.iter().any(|coop_id| identity.belongs_to(coop_id));
                    if !belongs {
                        warn!("Vote rejected: Voter {} not a member of any eligible cooperatives", 
                            vote.voter);
                        return Err(StorageError::Other { 
                            details: "Voter not a member of any eligible cooperatives".to_string() 
                        });
                    }
                    true
                },
                ProposalScope::GlobalFederation => {
                    // All federation members can vote
                    true
                },
            };
            
            if !is_eligible {
                return Err(StorageError::Other { 
                    details: "Voter not eligible for this proposal scope".to_string() 
                });
            }
        } else {
            // Without identity information, we can't enforce eligibility
            debug!("No voter identity provided, skipping eligibility check");
        }
        
        // Create the storage key - we'll store votes as a list under the proposal
        let key = format!("{}{}", FEDERATION_VOTES_PREFIX, vote.proposal_id);
        
        // First try to get existing votes
        let mut votes: Vec<FederatedVote> = match storage.get_json(None, "votes", &key) {
            Ok(existing_votes) => existing_votes,
            Err(_) => Vec::new(),
        };
        
        // Add the new vote
        votes.push(vote.clone());
        
        // Store the updated votes list
        storage.set_json(None, "votes", &key, &votes)?;
        
        // Update the cache
        let mut cache = self.cache.lock().unwrap();
        cache.votes
            .entry(vote.proposal_id.clone())
            .or_insert_with(Vec::new)
            .push(vote);
            
        info!("Saved federation vote to storage and cache");
        Ok(())
    }
    
    /// Get a proposal by ID
    pub fn get_proposal<S: StorageExtensions>(
        &self,
        storage: &S,
        proposal_id: &str
    ) -> StorageResult<FederatedProposal> {
        // First check the cache
        {
            let cache = self.cache.lock().unwrap();
            if let Some(proposal) = cache.proposals.get(proposal_id) {
                return Ok(proposal.clone());
            }
        }
        
        // If not in cache, check storage
        let key = format!("{}{}", FEDERATION_PROPOSAL_PREFIX, proposal_id);
        let namespace = "federation"; // Default namespace if not known
        storage.get_json(None, namespace, &key)
    }
    
    /// Get all votes for a proposal
    pub fn get_votes<S: StorageExtensions>(
        &self,
        storage: &S,
        proposal_id: &str
    ) -> StorageResult<Vec<FederatedVote>> {
        // First check the cache
        {
            let cache = self.cache.lock().unwrap();
            if let Some(votes) = cache.votes.get(proposal_id) {
                return Ok(votes.clone());
            }
        }
        
        // If not in cache, check storage
        let key = format!("{}{}", FEDERATION_VOTES_PREFIX, proposal_id);
        storage.get_json(None, "votes", &key)
    }
    
    /// Convert votes to a format suitable for the ranked vote algorithm
    pub fn prepare_ranked_ballots(&self, votes: &[FederatedVote], option_count: usize) -> Vec<Vec<f64>> {
        votes.iter()
            .map(|vote| vote.ranked_choices.clone())
            .collect()
    }
} 