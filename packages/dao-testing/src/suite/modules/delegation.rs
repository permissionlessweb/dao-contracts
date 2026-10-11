//! Unified DAO delegation surface.
//!
//! Two related but distinct jobs live here so scripts can share one suite:
//! - **vote**: `DaoVoteDelegation` — members delegate *voting power* inside the DAO
//! - **native**: [`super::native_delegation`] — the DAO treasury delegates *stake*
//!   and sends gas to operators
//!
//! Append this to [`crate::DaoDaoSuite`] as `delegation`.

use cw_orch::prelude::*;
use dao_cw_orch::DaoVoteDelegation;

use super::native_delegation::{
    instantiate_subdao_msg, NativeDelegationPlan, NEW_VALIDATOR_ACCOUNT, NEW_VALIDATOR_OPERATOR,
};
use cosmwasm_std::{Coin, CosmosMsg, Uint256};

/// Chain-bound vote-delegation contract plus native staking builders.
pub struct DaoDelegationSuite<Chain: CwEnv> {
    /// Existing vote-power delegation module (same id as `voting.vote_delegation`).
    pub vote: DaoVoteDelegation<Chain>,
}

impl<Chain: CwEnv> DaoDelegationSuite<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            vote: DaoVoteDelegation::new("dao_vote_delegation", chain),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.vote.upload()?;
        Ok(())
    }

    pub fn upload_if_needed(&self) -> Result<(), CwOrchError> {
        self.vote.upload_if_needed()?;
        Ok(())
    }

    /// Bonded-set campaign: 100k to each validator. New-operator stake/THIOL later.
    pub fn foundation_native_plan(&self, other_validators: Vec<String>) -> NativeDelegationPlan {
        NativeDelegationPlan::bonded_set(other_validators)
    }

    pub fn foundation_msgs(&self, other_validators: Vec<String>) -> Vec<CosmosMsg> {
        self.foundation_native_plan(other_validators).cosmos_msgs()
    }

    /// Same as [`Self::foundation_msgs`] plus a subDAO instantiate (FIAB).
    pub fn foundation_msgs_with_subdao(
        &self,
        other_validators: Vec<String>,
        admin: impl Into<String>,
        subdao_code_id: u64,
        label: impl Into<String>,
        instantiate_msg: Vec<u8>,
        funds: Vec<Coin>,
    ) -> Vec<CosmosMsg> {
        self.foundation_native_plan(other_validators).with_extra(vec![
            instantiate_subdao_msg(admin, subdao_code_id, label, instantiate_msg, funds),
        ])
    }

    pub fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![Box::new(&mut self.vote)]
    }
}

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoDelegationSuite<Chain> {
    type Error = CwOrchError;
    type DeployData = ();

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let suite = Self::new(chain);
        suite.upload_if_needed()?;
        Ok(suite)
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        self.get_contracts_mut()
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        Ok(Self::new(chain))
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        Self::store_on(chain)
    }
}

/// Quoted live constants for scripts.
pub fn live_new_validator() -> (&'static str, &'static str) {
    (NEW_VALIDATOR_OPERATOR, NEW_VALIDATOR_ACCOUNT)
}

pub fn micro(whole: u128) -> Uint256 {
    Uint256::from(whole * super::native_delegation::MICRO)
}
