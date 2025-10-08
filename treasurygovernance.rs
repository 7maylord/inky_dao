use crate::errors::{Error, Result};
use crate::types::*;

use ink::storage::Mapping;
use ink::prelude::string::String;
use ink::prelude::vec::Vec;
use ink::primitives::H160;

#[ink::contract]
pub mod treasury_governance {
    use super::*;

    /// Events
    #[ink(event)]
    pub struct ProposalCreated {
        #[ink(topic)]
        proposal_id: u32,
        #[ink(topic)]
        proposer: H160,
        title: String,
    }

    #[ink(event)]
    pub struct VoteCast {
        #[ink(topic)]
        proposal_id: u32,
        #[ink(topic)]
        voter: H160,
        option_index: u32,
        option_text: String,
        weight: u128,
    }

    #[ink(event)]
    pub struct ProposalExecuted {
        #[ink(topic)]
        proposal_id: u32,
        status: ProposalStatus,
    }

    /// Defines the storage of your contract.
    #[ink(storage)]
    pub struct TreasuryGovernance {
        /// Mapping from proposal ID to Proposal
        proposals: Mapping<u32, Proposal>,
        /// Mapping from (proposal_id, voter) to Vote
        votes: Mapping<(u32, H160), Vote>,
        /// Mapping from voter address to registration status
        registered_voters: Mapping<H160, bool>,
        /// Next proposal ID
        next_proposal_id: u32,
        /// Total number of proposals
        proposal_count: u32,
        /// Total number of voters(for quorum calculation)
        total_voters: u32,
        /// contract owner
        owner: H160,
    }

    impl TreasuryGovernance {
        /// Constructor that initializes the treasury governance contract.
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                proposals: Mapping::new(),
                votes: Mapping::new(),
                registered_voters: Mapping::new(),
                next_proposal_id: 1,
                proposal_count: 0,
                total_voters: 0,
                owner: caller,
            }
        }

        /// Create a new proposal
        #[ink(message)]
        pub fn create_proposal(&mut self, title: String, description: String, proposal_type: ProposalType, governance_params: GovernanceParameters, voting_options: VotingOptions) -> Result<u32> {
            // Validate voting options
            if voting_options.options.is_empty() || voting_options.options.len() > 10 {
                return Err(Error::InvalidProposal);
            }
           
            // Validate that all voting options are non-empty strings
            for option in &voting_options.options {
                if option.trim().is_empty() {
                    return Err(Error::InvalidProposal);
                }
            }
            
            let proposal_id = self.next_proposal_id;
            let caller = self.env().caller();
            
            // Calculate voting end time based on governance parameters
            let current_time = self.env().block_timestamp() as u32;
            let voting_duration = match governance_params.voting_period {
                VotingPeriod::ThreeDays => 3 * 24 * 60 * 60, // 3 days in seconds
                VotingPeriod::SevenDays => 7 * 24 * 60 * 60,
                VotingPeriod::FourteenDays => 14 * 24 * 60 * 60,
                VotingPeriod::ThirtyDays => 30 * 24 * 60 * 60,
            };
            
            // Calculate execution time based on execution delay
            let execution_delay = match governance_params.execution_delay {
                ExecutionDelay::Immediately => 0,
                ExecutionDelay::OneDay => 24 * 60 * 60,
                ExecutionDelay::TwoDays => 2 * 24 * 60 * 60,
                ExecutionDelay::SevenDays => 7 * 24 * 60 * 60,
            };
            
            let voting_end = current_time.checked_add(voting_duration)
                .ok_or(Error::InvalidProposal)?;
            
            let execution_time = voting_end.checked_add(execution_delay)
                .ok_or(Error::InvalidProposal)?;
            
            let mut vote_counts = Vec::new();
            for _ in 0..voting_options.options.len() {
                vote_counts.push(0);
            }
            
            let proposal = Proposal {
                id: proposal_id,
                title: title.clone(),
                description,
                proposal_type,
                governance_params,
                voting_options: voting_options.clone(),
                proposer: caller,
                created_at: current_time,
                voting_end,
                execution_time,
                status: ProposalStatus::Active,
                vote_counts,
                total_voters: 0,
            };
            
            // Store proposal
            self.proposals.insert(proposal_id, &proposal);
            self.next_proposal_id += 1;
            self.proposal_count += 1;
            
            // Emit event
            self.env().emit_event(ProposalCreated {
                proposal_id,
                proposer: caller,
                title,
            });
            
            Ok(proposal_id)
        }

        /// Vote on a proposal
        #[ink(message)]
        pub fn vote(&mut self, proposal_id: u32, choice: VoteChoice) -> Result<()> {
            let caller = self.env().caller();
            let current_time = self.env().block_timestamp() as u32;
            
            // Get the proposal
            let mut proposal = self.proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;
            
            // Validate proposal is active
            if proposal.status != ProposalStatus::Active {
                return Err(Error::ProposalNotActive);
            }
            
            // Validate voting period has not ended
            if current_time > proposal.voting_end {
                return Err(Error::VotingPeriodEnded);
            }
            
            // Check if user is registered as a voter
            if !self.is_voter_registered(caller) {
                return Err(Error::NotAuthorized);
            }
            
            // Prevent double voting
            if self.votes.contains((proposal_id, caller)) {
                return Err(Error::AlreadyVoted);
            }
            
            // Validate option index
            if choice.option_index as usize >= proposal.voting_options.options.len() {
                return Err(Error::InvalidProposal);
            }
            
            // Create vote record
            let vote = Vote {
                voter: caller,
                choice: choice.clone(),
                timestamp: current_time,
                weight: 1, // Default weight of 1, can be extended for weighted voting
            };
            
            // Store vote record
            self.votes.insert((proposal_id, caller), &vote);
            
            // Update vote counts
            if let Some(vote_count) = proposal.vote_counts.get_mut(choice.option_index as usize) {
                *vote_count += 1;
            }
            
            // Update total voters
            proposal.total_voters += 1;
            
            // Update proposal in storage
            self.proposals.insert(proposal_id, &proposal);
            
            // Emit vote event
            self.env().emit_event(VoteCast {
                proposal_id,
                voter: caller,
                option_index: choice.option_index,
                option_text: choice.option_text,
                weight: 1,
            });
            
            Ok(())
        }

       

        /// Update proposal status based on voting results and quorum
        #[ink(message)]
        pub fn update_proposal_status(&mut self, proposal_id: u32) -> Result<ProposalStatus> {
            let current_time = self.env().block_timestamp() as u32;
            
            // Get the proposal
            let mut proposal = self.proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;
            
            // Only update if proposal is still active
            if proposal.status != ProposalStatus::Active {
                return Ok(proposal.status);
            }
            
            // Check if voting period has ended
            if current_time <= proposal.voting_end {
                return Err(Error::ProposalNotReadyForExecution);
            }
            
            // Calculate quorum requirements
            let quorum_percentage = match proposal.governance_params.quorum_threshold {
                QuorumThreshold::Five => 5,
                QuorumThreshold::Ten => 10,
                QuorumThreshold::Twenty => 20,
                QuorumThreshold::TwentyFive => 25,
            };
            
            // Calculate required votes for quorum
            let required_votes = (self.total_voters * quorum_percentage) / 100;
            
            // Check if quorum is met
            if proposal.total_voters < required_votes {
                proposal.status = ProposalStatus::Rejected;
                self.proposals.insert(proposal_id, &proposal);
                
                self.env().emit_event(ProposalExecuted {
                    proposal_id,
                    status: ProposalStatus::Rejected,
                });
                
                return Ok(ProposalStatus::Rejected);
            }
            
            // Find the winning option (highest vote count)
            let mut max_votes = 0;
            let mut tie_count = 0;
            
            for &vote_count in &proposal.vote_counts {
                if vote_count > max_votes {
                    max_votes = vote_count;
                    tie_count = 1;
                } else if vote_count == max_votes && vote_count > 0 {
                    tie_count += 1;
                }
            }
            
            // Handle ties - if there's a tie for the highest vote count, mark as rejected
            if tie_count > 1 {
                proposal.status = ProposalStatus::Rejected;
                self.proposals.insert(proposal_id, &proposal);
                
                self.env().emit_event(ProposalExecuted {
                    proposal_id,
                    status: ProposalStatus::Rejected,
                });
                
                return Ok(ProposalStatus::Rejected);
            }
            
            // If we have a clear winner and quorum is met, mark as passed
            if max_votes > 0 {
                proposal.status = ProposalStatus::Passed;
                self.proposals.insert(proposal_id, &proposal);
                
                self.env().emit_event(ProposalExecuted {
                    proposal_id,
                    status: ProposalStatus::Passed,
                });
                
                return Ok(ProposalStatus::Passed);
            }
            
            // If no votes were cast, mark as rejected
            proposal.status = ProposalStatus::Rejected;
            self.proposals.insert(proposal_id, &proposal);
            
            self.env().emit_event(ProposalExecuted {
                proposal_id,
                status: ProposalStatus::Rejected,
            });
            
            Ok(ProposalStatus::Rejected)
        }

        /// Execute a passed proposal 
        #[ink(message)]
        pub fn execute_proposal(&mut self, proposal_id: u32) -> Result<()> {
            let current_time = self.env().block_timestamp() as u32;
            
            // Get the proposal
            let mut proposal = self.proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;
            
            // Check if proposal is passed and ready for execution
            if proposal.status != ProposalStatus::Passed {
                return Err(Error::ProposalNotReadyForExecution);
            }
            
            // Check if execution time has been reached
            if current_time < proposal.execution_time {
                return Err(Error::ProposalNotReadyForExecution);
            }
            
            // Mark as executed
            proposal.status = ProposalStatus::Executed;
            self.proposals.insert(proposal_id, &proposal);
            
            self.env().emit_event(ProposalExecuted {
                proposal_id,
                status: ProposalStatus::Executed,
            });
            
            Ok(())
        }

        /// Register a user as a global voter
        #[ink(message)]
        pub fn register_voter(&mut self) -> Result<()> {
            let caller = self.env().caller();
            
            // Check if user is already registered
            if self.is_voter_registered(caller) {
                return Err(Error::AlreadyRegistered);
            }
            
            // Register the voter globally
            self.registered_voters.insert(caller, &true);
            
            // Increment total voter count
            self.total_voters += 1;
            
            Ok(())
        }

        /// Check if a user is registered as a voter
        #[ink(message)]
        pub fn is_voter_registered(&self, user: H160) -> bool {
            self.registered_voters.get(user).unwrap_or(false)
        }


        /// Get a proposal by ID
        #[ink(message)]
        pub fn get_proposal(&self, proposal_id: u32) -> Option<Proposal> {
            self.proposals.get(proposal_id)
        }

        /// Get the total number of proposals
        #[ink(message)]
        pub fn get_proposal_count(&self) -> u32 {
            self.proposal_count
        }

        /// Get user's vote on a proposal
        #[ink(message)]
        pub fn get_user_vote(&self, proposal_id: u32, user: H160) -> Option<Vote> {
            self.votes.get((proposal_id, user))
        }

        /// Get contract statistics (total, active, executed proposals)
        #[ink(message)]
        pub fn get_stats(&self) -> (u32, u32, u32) {
            let mut active_count = 0;
            let mut executed_count = 0;
            
            // Count active and executed proposals
            for i in 1..self.next_proposal_id {
                if let Some(proposal) = self.proposals.get(i) {
                    match proposal.status {
                        ProposalStatus::Active => active_count += 1,
                        ProposalStatus::Executed => executed_count += 1,
                        _ => {}
                    }
                }
            }
            
            (self.proposal_count, active_count, executed_count)
        }

        /// Get the total number of registered voters
        #[ink(message)]
        pub fn get_total_voters(&self) -> u32 {
            self.total_voters
        }

        /// Check if proposal has reached quorum
        #[ink(message)]
        pub fn has_reached_quorum(&self, proposal_id: u32) -> Result<bool> {
            let proposal = self.proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;
            
            let quorum_percentage = match proposal.governance_params.quorum_threshold {
                QuorumThreshold::Five => 5,
                QuorumThreshold::Ten => 10,
                QuorumThreshold::Twenty => 20,
                QuorumThreshold::TwentyFive => 25,
            };
            
            let required_votes = (self.total_voters * quorum_percentage) / 100;
            Ok(proposal.total_voters >= required_votes)
        }

        /// Get proposal results (vote counts and quorum status)
        #[ink(message)]
        pub fn get_proposal_results(&self, proposal_id: u32) -> Result<(Vec<u128>, bool, u32, u32)> {
            let proposal = self.proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;
            
            let quorum_percentage = match proposal.governance_params.quorum_threshold {
                QuorumThreshold::Five => 5,
                QuorumThreshold::Ten => 10,
                QuorumThreshold::Twenty => 20,
                QuorumThreshold::TwentyFive => 25,
            };
            
            let required_votes = (self.total_voters * quorum_percentage) / 100;
            let has_quorum = proposal.total_voters >= required_votes;
            
            Ok((proposal.vote_counts, has_quorum, proposal.total_voters, required_votes))
        }

        /// Get voting options for a proposal
        #[ink(message)]
        pub fn get_voting_options(&self, proposal_id: u32) -> Result<Vec<String>> {
            let proposal = self.proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;
            Ok(proposal.voting_options.options)
        }

        /// Get detailed results with option names
        #[ink(message)]
        pub fn get_detailed_results(&self, proposal_id: u32) -> Result<Vec<(String, u128)>> {
            let proposal = self.proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;
            
            let mut results = Vec::new();
            for (index, &vote_count) in proposal.vote_counts.iter().enumerate() {
                if let Some(option_text) = proposal.voting_options.options.get(index) {
                    results.push((option_text.clone(), vote_count));
                }
            }
            
            Ok(results)
        }

        /// Get the winning option and vote count
        #[ink(message)]
        pub fn get_winning_option(&self, proposal_id: u32) -> Result<Option<(String, u128)>> {
            let proposal = self.proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;
            
            let mut max_votes = 0;
            let mut winning_index = None;
            let mut tie_count = 0;
            
            for (index, &vote_count) in proposal.vote_counts.iter().enumerate() {
                if vote_count > max_votes {
                    max_votes = vote_count;
                    winning_index = Some(index);
                    tie_count = 1;
                } else if vote_count == max_votes && vote_count > 0 {
                    tie_count += 1;
                }
            }
            
            // If there's a tie or no votes, return None
            if tie_count > 1 || max_votes == 0 {
                return Ok(None);
            }
            
            // Return the winning option
            if let Some(index) = winning_index {
                if let Some(option_text) = proposal.voting_options.options.get(index) {
                    return Ok(Some((option_text.clone(), max_votes)));
                }
            }
            
            Ok(None)
        }

        /// Get the next proposal ID
        #[ink(message)]
        pub fn get_next_proposal_id(&self) -> u32 {
            self.next_proposal_id
        }

        

        
    }

}
