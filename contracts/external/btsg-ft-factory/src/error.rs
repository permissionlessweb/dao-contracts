use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, )]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("An unknown reply ID was received.")]
    UnknownReplyID {},
}

impl PartialEq for ContractError {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}