
#[derive(Debug, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum Error {
    ProposalNotFound,
    ProposalNotActive,
    VotingPeriodEnded,
    AlreadyVoted,
    NotAuthorized,
    ProposalNotReadyForExecution,
    InvalidProposal,
    AlreadyRegistered,
}

pub type Result<T> = core::result::Result<T, Error>;