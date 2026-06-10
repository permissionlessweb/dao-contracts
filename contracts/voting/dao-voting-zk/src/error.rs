use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Snapshot already exists for proposal {proposal_id}")]
    SnapshotAlreadyExists { proposal_id: u64 },

    #[error("No snapshot found for proposal {proposal_id}")]
    NoSnapshot { proposal_id: u64 },

    #[error("No poll found for poll_id {poll_id}")]
    NoPoll { poll_id: String },

    #[error("Invalid Merkle root: {reason}")]
    InvalidMerkleRoot { reason: String },

    #[error("Total power must be greater than zero")]
    ZeroTotalPower {},

    #[error("PollRegistry query failed: {reason}")]
    PollRegistryQuery { reason: String },

    #[error("PollRegistry execution failed: {reason}")]
    PollRegistryExecute { reason: String },
}

impl PartialEq for ContractError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ContractError::Std(_), ContractError::Std(_)) => true,
            (ContractError::Unauthorized {}, ContractError::Unauthorized {}) => true,
            (ContractError::SnapshotAlreadyExists { proposal_id: a }, ContractError::SnapshotAlreadyExists { proposal_id: b }) => a == b,
            (ContractError::NoSnapshot { proposal_id: a }, ContractError::NoSnapshot { proposal_id: b }) => a == b,
            (ContractError::NoPoll { poll_id: a }, ContractError::NoPoll { poll_id: b }) => a == b,
            (ContractError::InvalidMerkleRoot { reason: a }, ContractError::InvalidMerkleRoot { reason: b }) => a == b,
            (ContractError::ZeroTotalPower {}, ContractError::ZeroTotalPower {}) => true,
            (ContractError::PollRegistryQuery { reason: a }, ContractError::PollRegistryQuery { reason: b }) => a == b,
            (ContractError::PollRegistryExecute { reason: a }, ContractError::PollRegistryExecute { reason: b }) => a == b,
            _ => false,
        }
    }
}