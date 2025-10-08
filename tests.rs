#![cfg(test)]

use ink::env::test::{default_accounts, advance_block, set_block_timestamp, set_caller};

use crate::treasurygovernance::treasury_governance::TreasuryGovernance;
use crate::types::*;

fn create_test_proposal_params() -> (String, String, ProposalType, GovernanceParameters, VotingOptions) {
    let title = "Test Proposal".to_string();
    let description = "This is a test proposal".to_string();
    let proposal_type = ProposalType::Treasury;
    let governance_params = GovernanceParameters {
        voting_period: VotingPeriod::SevenDays,
        quorum_threshold: QuorumThreshold::Ten,
        execution_delay: ExecutionDelay::OneDay,
    };
    let voting_options = VotingOptions {
        options: vec!["Yes".to_string(), "No".to_string()],
    };
    (title, description, proposal_type, governance_params, voting_options)
}

mod tests {
    use super::*;

    #[ink::test]
    fn valid_proposal_creation() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        let (title, description, proposal_type, governance_params, voting_options) = create_test_proposal_params();
        
        let result = contract.create_proposal(title.clone(), description, proposal_type, governance_params, voting_options);
        assert!(result.is_ok());
        
        let proposal_id = result.unwrap();
        assert_eq!(proposal_id, 1);
        
        let proposal = contract.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.title, title);
        assert_eq!(proposal.status, ProposalStatus::Active);
        assert_eq!(proposal.vote_counts.len(), 2);
        assert_eq!(proposal.vote_counts[0], 0);
        assert_eq!(proposal.vote_counts[1], 0);
    }

    #[ink::test]
    fn invalid_voting_options_empty() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        let (title, description, proposal_type, governance_params, _) = create_test_proposal_params();
        let voting_options = VotingOptions {
            options: vec![],
        };
        
        let result = contract.create_proposal(title, description, proposal_type, governance_params, voting_options);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::errors::Error::InvalidProposal);
    }

    #[ink::test]
    fn invalid_voting_options_too_many() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        let (title, description, proposal_type, governance_params, _) = create_test_proposal_params();
        let voting_options = VotingOptions {
            options: (1..=11).map(|i| format!("Option {}", i)).collect(),
        };
        
        let result = contract.create_proposal(title, description, proposal_type, governance_params, voting_options);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::errors::Error::InvalidProposal);
    }

    #[ink::test]
    fn invalid_voting_options_empty_strings() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        let (title, description, proposal_type, governance_params, _) = create_test_proposal_params();
        let voting_options = VotingOptions {
            options: vec!["Valid Option".to_string(), "".to_string()],
        };
        
        let result = contract.create_proposal(title, description, proposal_type, governance_params, voting_options);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::errors::Error::InvalidProposal);
    }

    #[ink::test]
    fn time_calculations_voting_period() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        let (title, description, proposal_type, mut governance_params, voting_options) = create_test_proposal_params();
        
        // Test different voting periods
        governance_params.voting_period = VotingPeriod::ThreeDays;
        let result = contract.create_proposal(title.clone(), description.clone(), proposal_type.clone(), governance_params.clone(), voting_options.clone());
        assert!(result.is_ok());
        
        let proposal = contract.get_proposal(1).unwrap();
        let expected_end = proposal.created_at + (3 * 24 * 60 * 60);
        assert_eq!(proposal.voting_end, expected_end);
    }

    #[ink::test]
    fn time_calculations_execution_delay() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        let (title, description, proposal_type, mut governance_params, voting_options) = create_test_proposal_params();
        
        governance_params.execution_delay = ExecutionDelay::TwoDays;
        let result = contract.create_proposal(title, description, proposal_type, governance_params, voting_options);
        assert!(result.is_ok());
        
        let proposal = contract.get_proposal(1).unwrap();
        let expected_execution = proposal.voting_end + (2 * 24 * 60 * 60);
        assert_eq!(proposal.execution_time, expected_execution);
    }


    #[ink::test]
    fn successful_voting() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Register voter
        contract.register_voter().unwrap();
        
        // Create proposal
        let (title, description, proposal_type, governance_params, voting_options) = create_test_proposal_params();
        let proposal_id = contract.create_proposal(title, description, proposal_type, governance_params, voting_options).unwrap();
        
        // Vote
        let vote_choice = VoteChoice {
            option_index: 0,
            option_text: "Yes".to_string(),
        };
        
        let result = contract.vote(proposal_id, vote_choice);
        assert!(result.is_ok());
        
        // Check vote was recorded
        let user_vote = contract.get_user_vote(proposal_id, accounts.alice);
        assert!(user_vote.is_some());
        assert_eq!(user_vote.unwrap().choice.option_index, 0);
        
        // Check vote counts updated
        let proposal = contract.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.vote_counts[0], 1);
        assert_eq!(proposal.vote_counts[1], 0);
        assert_eq!(proposal.total_voters, 1);
    }

    #[ink::test]
    fn double_voting_prevention() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Register voter
        contract.register_voter().unwrap();
        
        // Create proposal
        let (title, description, proposal_type, governance_params, voting_options) = create_test_proposal_params();
        let proposal_id = contract.create_proposal(title, description, proposal_type, governance_params, voting_options).unwrap();
        
        // First vote
        let vote_choice = VoteChoice {
            option_index: 0,
            option_text: "Yes".to_string(),
        };
        contract.vote(proposal_id, vote_choice.clone()).unwrap();
        
        // Second vote should fail
        let result = contract.vote(proposal_id, vote_choice);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::errors::Error::AlreadyVoted);
    }

    #[ink::test]
    fn invalid_option_index() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Register voter
        contract.register_voter().unwrap();
        
        // Create proposal
        let (title, description, proposal_type, governance_params, voting_options) = create_test_proposal_params();
        let proposal_id = contract.create_proposal(title, description, proposal_type, governance_params, voting_options).unwrap();
        
        // Vote with invalid option index
        let vote_choice = VoteChoice {
            option_index: 5, // Invalid index
            option_text: "Invalid".to_string(),
        };
        
        let result = contract.vote(proposal_id, vote_choice);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::errors::Error::InvalidProposal);
    }

    #[ink::test]
    fn voting_period_validation() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Register voter
        contract.register_voter().unwrap();
        
        // Create proposal
        let (title, description, proposal_type, governance_params, voting_options) = create_test_proposal_params();
        let proposal_id = contract.create_proposal(title, description, proposal_type, governance_params, voting_options).unwrap();
        
        // Advance time past voting period
        let proposal = contract.get_proposal(proposal_id).unwrap();
        set_block_timestamp::<ink::env::DefaultEnvironment>((proposal.voting_end + 1) as u64);
        
        // Vote should fail
        let vote_choice = VoteChoice {
            option_index: 0,
            option_text: "Yes".to_string(),
        };
        
        let result = contract.vote(proposal_id, vote_choice);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::errors::Error::VotingPeriodEnded);
    }

    #[ink::test]
    fn unregistered_voter_cannot_vote() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Create proposal without registering
        let (title, description, proposal_type, governance_params, voting_options) = create_test_proposal_params();
        let proposal_id = contract.create_proposal(title, description, proposal_type, governance_params, voting_options).unwrap();
        
        // Vote should fail
        let vote_choice = VoteChoice {
            option_index: 0,
            option_text: "Yes".to_string(),
        };
        
        let result = contract.vote(proposal_id, vote_choice);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::errors::Error::NotAuthorized);
    }


    #[ink::test]
    fn automatic_status_updates_quorum_met() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Register multiple voters
        contract.register_voter().unwrap();
        set_caller(accounts.bob);
        contract.register_voter().unwrap();
        set_caller(accounts.charlie);
        contract.register_voter().unwrap();
        
        // Create proposal
        set_caller(accounts.alice);
        let (title, description, proposal_type, governance_params, voting_options) = create_test_proposal_params();
        let proposal_id = contract.create_proposal(title, description, proposal_type, governance_params, voting_options).unwrap();
        
        // Vote to meet quorum (10% of 3 voters = 1 vote needed)
        let vote_choice = VoteChoice {
            option_index: 0,
            option_text: "Yes".to_string(),
        };
        contract.vote(proposal_id, vote_choice).unwrap();
        
        // Advance time past voting period
        let proposal = contract.get_proposal(proposal_id).unwrap();
        set_block_timestamp::<ink::env::DefaultEnvironment>((proposal.voting_end + 1) as u64);
        
        // Update status
        let result = contract.update_proposal_status(proposal_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProposalStatus::Passed);
        
        // Check proposal status
        let proposal = contract.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Passed);
    }

    #[ink::test]
    fn automatic_status_updates_quorum_not_met() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Register only one voter
        contract.register_voter().unwrap();
        
        // Create proposal with higher quorum requirement
        let (title, description, proposal_type, mut governance_params, voting_options) = create_test_proposal_params();
        governance_params.quorum_threshold = QuorumThreshold::Twenty; // 20% quorum
        let proposal_id = contract.create_proposal(title, description, proposal_type, governance_params, voting_options).unwrap();
        
        // Don't vote (no votes cast)
        
        // Advance time past voting period
        let proposal = contract.get_proposal(proposal_id).unwrap();
        set_block_timestamp::<ink::env::DefaultEnvironment>((proposal.voting_end + 1) as u64);
        
        // Update status
        let result = contract.update_proposal_status(proposal_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProposalStatus::Rejected);
        
        // Check proposal status
        let proposal = contract.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Rejected);
    }

    #[ink::test]
    fn tie_handling() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Register voters
        contract.register_voter().unwrap();
        set_caller(accounts.bob);
        contract.register_voter().unwrap();
        
        // Create proposal
        set_caller(accounts.alice);
        let (title, description, proposal_type, governance_params, voting_options) = create_test_proposal_params();
        let proposal_id = contract.create_proposal(title, description, proposal_type, governance_params, voting_options).unwrap();
        
        // Create a tie (1 vote each)
        let vote_choice_1 = VoteChoice {
            option_index: 0,
            option_text: "Yes".to_string(),
        };
        contract.vote(proposal_id, vote_choice_1).unwrap();
        
        set_caller(accounts.bob);
        let vote_choice_2 = VoteChoice {
            option_index: 1,
            option_text: "No".to_string(),
        };
        contract.vote(proposal_id, vote_choice_2).unwrap();
        
        // Advance time past voting period
        let proposal = contract.get_proposal(proposal_id).unwrap();
        set_block_timestamp::<ink::env::DefaultEnvironment>((proposal.voting_end + 1) as u64);
        
        // Update status
        let result = contract.update_proposal_status(proposal_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ProposalStatus::Rejected);
        
        // Check proposal status
        let proposal = contract.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Rejected);
    }

    #[ink::test]
    fn execution_timing() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Register voter
        contract.register_voter().unwrap();
        
        // Create proposal
        let (title, description, proposal_type, governance_params, voting_options) = create_test_proposal_params();
        let proposal_id = contract.create_proposal(title, description, proposal_type, governance_params, voting_options).unwrap();
        
        // Vote and update status to passed
        let vote_choice = VoteChoice {
            option_index: 0,
            option_text: "Yes".to_string(),
        };
        contract.vote(proposal_id, vote_choice).unwrap();
        
        let proposal = contract.get_proposal(proposal_id).unwrap();
        set_block_timestamp::<ink::env::DefaultEnvironment>((proposal.voting_end + 1) as u64);
        contract.update_proposal_status(proposal_id).unwrap();
        
        // Try to execute before execution time
        let result = contract.execute_proposal(proposal_id);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::errors::Error::ProposalNotReadyForExecution);
        
        // Advance to execution time
        let proposal = contract.get_proposal(proposal_id).unwrap();
        set_block_timestamp::<ink::env::DefaultEnvironment>((proposal.execution_time + 1) as u64);
        
        // Execute should succeed
        let result = contract.execute_proposal(proposal_id);
        assert!(result.is_ok());
        
        // Check status
        let proposal = contract.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Executed);
    }

    #[ink::test]
    fn maximum_voting_options() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        let (title, description, proposal_type, governance_params, _) = create_test_proposal_params();
        let voting_options = VotingOptions {
            options: (1..=10).map(|i| format!("Option {}", i)).collect(),
        };
        
        let result = contract.create_proposal(title, description, proposal_type, governance_params, voting_options);
        assert!(result.is_ok());
        
        let proposal = contract.get_proposal(1).unwrap();
        assert_eq!(proposal.vote_counts.len(), 10);
    }

    #[ink::test]
    fn non_existent_proposal_handling() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Try to get non-existent proposal
        let proposal = contract.get_proposal(999);
        assert!(proposal.is_none());
        
        // Try to vote on non-existent proposal
        contract.register_voter().unwrap();
        let vote_choice = VoteChoice {
            option_index: 0,
            option_text: "Yes".to_string(),
        };
        
        let result = contract.vote(999, vote_choice);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::errors::Error::ProposalNotFound);
    }

    #[ink::test]
    fn integer_overflow_prevention() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        let (title, description, proposal_type, mut governance_params, voting_options) = create_test_proposal_params();
        
        // Set a very large voting period that could cause overflow
        governance_params.voting_period = VotingPeriod::ThirtyDays;
        governance_params.execution_delay = ExecutionDelay::SevenDays;
        
        // Set block timestamp near u32::MAX
        set_block_timestamp::<ink::env::DefaultEnvironment>(u32::MAX as u64 - 1000);
        
        let result = contract.create_proposal(title, description, proposal_type, governance_params, voting_options);
        // Should either succeed or fail gracefully with InvalidProposal
        if result.is_err() {
            assert_eq!(result.unwrap_err(), crate::errors::Error::InvalidProposal);
        }
    }

    #[ink::test]
    fn voter_registration_edge_cases() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Register voter
        let result = contract.register_voter();
        assert!(result.is_ok());
        assert_eq!(contract.get_total_voters(), 1);
        assert!(contract.is_voter_registered(accounts.alice));
        
        // Try to register again
        let result = contract.register_voter();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), crate::errors::Error::AlreadyRegistered);
        
        // Check non-existent voter
        assert!(!contract.is_voter_registered(accounts.bob));
    }

    #[ink::test]
    fn quorum_calculations_edge_cases() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Create proposal
        let (title, description, proposal_type, governance_params, voting_options) = create_test_proposal_params();
        let proposal_id = contract.create_proposal(title, description, proposal_type, governance_params, voting_options).unwrap();
        
        // Test quorum with no registered voters (0 votes needed, so 0 votes meets quorum)
        let has_quorum = contract.has_reached_quorum(proposal_id).unwrap();
        assert!(has_quorum);
        
        // Register one voter
        contract.register_voter().unwrap();
        
        // Test quorum with 10% threshold and 1 voter (should need 0.1 votes, rounded down to 0)
        let has_quorum = contract.has_reached_quorum(proposal_id).unwrap();
        assert!(has_quorum); // 0 votes needed, so any votes meet quorum
    }

    #[ink::test]
    fn winning_option_edge_cases() {
        let accounts = default_accounts();
        set_caller(accounts.alice);
        
        let mut contract = TreasuryGovernance::new();
        
        // Create proposal
        let (title, description, proposal_type, governance_params, voting_options) = create_test_proposal_params();
        let proposal_id = contract.create_proposal(title, description, proposal_type, governance_params, voting_options).unwrap();
        
        // Test with no votes
        let winner = contract.get_winning_option(proposal_id).unwrap();
        assert!(winner.is_none());
        
        // Register voter and vote
        contract.register_voter().unwrap();
        let vote_choice = VoteChoice {
            option_index: 0,
            option_text: "Yes".to_string(),
        };
        contract.vote(proposal_id, vote_choice).unwrap();
        
        // Test with clear winner
        let winner = contract.get_winning_option(proposal_id).unwrap();
        assert!(winner.is_some());
        let (option_text, vote_count) = winner.unwrap();
        assert_eq!(option_text, "Yes");
        assert_eq!(vote_count, 1);
    }

}