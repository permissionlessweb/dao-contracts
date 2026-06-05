use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VotingError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("min_voting_period and max_voting_period must have the same units (height or time)")]
    DurationUnitsConflict {},

    #[error("Min voting period must be less than or equal to max voting period")]
    InvalidMinVotingPeriod {},
}

impl PartialEq for VotingError {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}
