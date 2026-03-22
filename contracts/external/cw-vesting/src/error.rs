use cosmwasm_std::{StdError, Uint256};
use cw_denom::DenomError;
use cw_ownable::OwnershipError;
use cw_utils::PaymentError;
use thiserror::Error;
use wynd_utils::CurveError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Curve(#[from] CurveError),

    #[error(transparent)]
    Denom(#[from] DenomError),

    #[error(transparent)]
    Ownable(#[from] OwnershipError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("vesting curve values be in [0, total]`. got [{min}, {max}]")]
    VestRange { min: Uint256, max: Uint256 },

    #[error("vesting contract vests ({expected}) tokens, funded with ({sent})")]
    WrongFundAmount { sent: Uint256, expected: Uint256 },

    #[error("sent wrong cw20")]
    WrongCw20,

    #[error("total amount to vest must be non-zero")]
    ZeroVest,

    #[error("this vesting contract would complete instantly")]
    Instavest,

    #[error("can not vest a constant amount, specifiy two or more points")]
    ConstantVest,

    #[error("payment is cancelled")]
    Cancelled,

    #[error("payment is not cancelled")]
    NotCancelled,

    #[error("vesting contract is not distributing funds")]
    NotFunded,

    #[error("it should not be possible for a slash to occur in the unfunded state")]
    UnfundedSlash,

    #[error("vesting contract has already been funded")]
    Funded,

    #[error("only the vest receiver may perform this action")]
    NotReceiver,

    #[error("vesting denom may not be staked")]
    NotStakeable,

    #[error("no delegation to validator {0}")]
    NoDelegation(String),

    #[error("slash amount can not be zero")]
    NoSlash,

    #[error("can't set wihtdraw address to vesting contract")]
    SelfWithdraw,

    #[error("can't redelegate funds that are not immediately redelegatable. max: ({max})")]
    NonImmediateRedelegate { max: Uint256 },

    #[error("request must be <= claimable and > 0. !(0 < {request} <= {claimable})")]
    InvalidWithdrawal {
        request: Uint256,
        claimable: Uint256,
    },

    #[error("can't register a slash event occuring in the future")]
    FutureSlash,
}

impl PartialEq for ContractError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
