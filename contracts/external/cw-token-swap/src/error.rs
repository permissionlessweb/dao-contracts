use cosmwasm_std::{StdError, Uint256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Counterparties must have different addresses")]
    NonDistinctCounterparties {},

    #[error("Can not provide funds more than once")]
    AlreadyProvided {},

    #[error("Escrow funds have already been sent")]
    Complete {},

    #[error("Must provide funds before withdrawing")]
    NoProvision {},

    #[error("Can not create an escrow for zero tokens")]
    ZeroTokens {},

    #[error("Provided funds do not match promised funds")]
    InvalidFunds {},

    #[error("Invalid amount. Expected ({expected}), got ({actual})")]
    InvalidAmount { expected: Uint256, actual: Uint256 },
}

impl PartialEq for ContractError {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}
