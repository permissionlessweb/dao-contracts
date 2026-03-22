use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;
use thiserror::Error;

pub use cw_ownable::OwnershipError;
pub use cw_utils::PaymentError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error("ReplyParseError: {err}")]
    ReplyParseError { err: String },

    #[error(transparent)]
    Ownership(#[from] OwnershipError),

    #[error(transparent)]
    ParseReply(#[from] ParseReplyError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("JSON serialization error: {err}")]
    JsonSerialization { err: String },

    #[error("Unknown reply ID: {id}")]
    UnknownReplyID { id: u64 },

    #[error("Missing protobuf registry")]
    MissingProtobufRegistry {},
}
impl PartialEq for ContractError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
