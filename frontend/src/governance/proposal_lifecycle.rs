use crate::governance::proposal::ProposalStatus;

// Implement conversion between ProposalState and ProposalStatus
impl From<ProposalState> for ProposalStatus {
    fn from(state: ProposalState) -> Self {
        match state {
            ProposalState::Draft => ProposalStatus::Draft,
            ProposalState::OpenForFeedback => ProposalStatus::Deliberation,
            ProposalState::Voting => ProposalStatus::Voting,
            ProposalState::Executed => ProposalStatus::Executed,
            ProposalState::Rejected => ProposalStatus::Rejected,
            ProposalState::Expired => ProposalStatus::Expired,
        }
    }
} 