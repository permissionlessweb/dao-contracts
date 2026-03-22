use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("ReplyParseError: {err}")]
    ReplyParseError { err: String },

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("An unknown reply ID was received.")]
    UnknownReplyID {},

    #[error("Expected contract address {expected} but instantiated {actual}.")]
    UnexpectedContractAddress { expected: String, actual: String },
}

impl PartialEq for ContractError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
