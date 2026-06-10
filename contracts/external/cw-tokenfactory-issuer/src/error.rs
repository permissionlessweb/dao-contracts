use cosmwasm_std::Uint256;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    Ownership(#[from] cw_ownable::OwnershipError),

    #[error(transparent)]
    ConversionOverflowError(#[from] cosmwasm_std::ConversionOverflowError),

    #[error("BeforeSendHook not set. Features requiring it are disabled.")]
    BeforeSendHookFeaturesDisabled {},

    #[error("The address '{address}' is denied transfer abilities")]
    Denied { address: String },

    #[error("Cannot denylist the issuer contract itself")]
    CannotDenylistSelf {},

    #[error("The contract is frozen for denom {denom:?}. Addresses need to be added to the allowlist to enable transfers to or from an account.")]
    ContractFrozen { denom: String },

    #[error("Invalid subdenom: {subdenom:?}")]
    InvalidSubdenom { subdenom: String },

    #[error("Invalid denom: {denom:?} {message:?}")]
    InvalidDenom { denom: String, message: String },

    #[error("Not enough {denom:?} ({funds:?}) in funds. {needed:?} {denom:?} needed")]
    NotEnoughFunds {
        denom: String,
        funds: u128,
        needed: u128,
    },

    #[error("Not enough {action} allowance: attempted to {action} {amount}, but remaining allowance is {allowance}")]
    NotEnoughAllowance {
        action: String,
        amount: Uint256,
        allowance: Uint256,
    },

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Got a submessage reply with unknown id: {id}")]
    UnknownReplyId { id: u64 },

    #[error("amount was zero, must be positive")]
    ZeroAmount {},
}

impl ContractError {
    pub fn not_enough_mint_allowance(
        amount: impl Into<Uint256>,
        allowance: impl Into<Uint256>,
    ) -> ContractError {
        ContractError::NotEnoughAllowance {
            action: "mint".to_string(),
            amount: amount.into(),
            allowance: allowance.into(),
        }
    }

    pub fn not_enough_burn_allowance(
        amount: impl Into<Uint256>,
        allowance: impl Into<Uint256>,
    ) -> ContractError {
        ContractError::NotEnoughAllowance {
            action: "burn".to_string(),
            amount: amount.into(),
            allowance: allowance.into(),
        }
    }
}
