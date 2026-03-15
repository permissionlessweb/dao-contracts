use cosmwasm_std::StdError;
use cw_hooks::HookError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    HookError(#[from] HookError),

    #[error("unauthorized")]
    Unauthorized {},

    #[error("no such event ({id})")]
    NoSuchEvent { id: u64 },

    #[error("no such group ({id})")]
    NoSuchGroup { id: String },

    #[error("invalid time range: start must be before end")]
    InvalidTimeRange {},

    #[error("event ({id}) is not in upcoming status")]
    EventNotUpcoming { id: u64 },

    #[error("event ({id}) is not in active status")]
    EventNotActive { id: u64 },

    #[error("event ({id}) has already been triggered")]
    AlreadyTriggered { id: u64 },

    #[error("group ({id}) already exists")]
    GroupAlreadyExists { id: String },

    #[error("invalid supplier type")]
    InvalidSupplierType {},

    #[error("received a failed event hook reply with an invalid hook index: ({idx})")]
    InvalidHookIndex { idx: u64 },

    #[error("received a reply failure with an invalid ID: ({id})")]
    InvalidReplyID { id: u64 },
}
