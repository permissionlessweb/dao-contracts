use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownership(#[from] cw_ownable::OwnershipError),

    #[error("Invalid Cw20")]
    InvalidCw20 {},

    #[error("Invalid Staking Contract")]
    InvalidStakingContract {},

    #[error("Zero eligible rewards")]
    ZeroRewards {},

    #[error("Rewards have already been distributed for this block")]
    RewardsDistributedForBlock {},

    #[error("can not migrate. current version is up to date")]
    AlreadyMigrated {},
}

impl PartialEq for ContractError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
