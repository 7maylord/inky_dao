use crate::errors::{Error, Result};
use crate::types::{ProposalType, VotingPeriod, QuorumThreshold, ExecutionDelay, GovernanceParameters, VotingOptions, VoteChoice, Vote, ProposalStatus, Proposal};

use ink::storage::Mapping;
use ink::prelude::string::{String, ToString};
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
        proposer: AccountId,
        title: String,
    }

    #[ink(event)]
    pub struct VoteCast {
        #[ink(topic)]
        proposal_id: u32,
        #[ink(topic)]
        voter: AccountId,
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
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct TreasuryGovernance {
        /// Stores a single `bool` value on the storage.
        value: bool,
    }

    impl TreasuryGovernance {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self { value: false }
        }

        /// Create a new proposal
        #[ink(message)]
        pub fn create_proposal(&mut self, title: String, description: String, proposal_type: ProposalType, governance_params: GovernanceParameters, voting_options: VotingOptions) -> Result<()> {
            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test a simple use case of our contract.
        #[ink::test]
        fn it_works() {
            let mut dao_governance = DaoGovernance::new(false);
            assert_eq!(dao_governance.get(), false);
            dao_governance.flip();
            assert_eq!(dao_governance.get(), true);
        }
    }

}
