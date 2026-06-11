use cw_orch::{
    contract::Contract,
    core::contract::interface_traits::{ContractInstance, CwOrchUpload, Uploadable},
    prelude::*,
};
use dao_cw_orch::*;

/// Deploy data for voting modules (placeholder for future config).
#[derive(Clone, Debug, Default)]
pub struct DaoVotingDeployData;

/// Minimal contract wrapper for cw1-whitelist with correct wasm path resolution.
pub struct Cw1WhitelistContract<Chain> {
    pub(crate) contract: Contract<Chain>,
}

impl<Chain: ChainState> ContractInstance<Chain> for Cw1WhitelistContract<Chain> {
    fn as_instance(&self) -> &Contract<Chain> {
        &self.contract
    }
    fn as_instance_mut(&mut self) -> &mut Contract<Chain> {
        &mut self.contract
    }
}

impl<Chain: CwEnv> Uploadable for Cw1WhitelistContract<Chain> {
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("cw1_whitelist")
            .unwrap()
    }
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(
            cw1_whitelist::contract::execute,
            cw1_whitelist::contract::instantiate,
            cw1_whitelist::contract::query,
        ))
    }
}

impl<Chain: CwEnv> Cw1WhitelistContract<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            contract: Contract::new("cw1_whitelist", chain),
        }
    }
}
/// Minimal contract wrapper for cw4-group with correct wasm path resolution.
/// Uses the dao-contracts workspace artifacts dir, not cw-orchestrator's.
pub struct Cw4GroupContract<Chain> {
    pub(crate) contract: Contract<Chain>,
}

impl<Chain: ChainState> ContractInstance<Chain> for Cw4GroupContract<Chain> {
    fn as_instance(&self) -> &Contract<Chain> {
        &self.contract
    }
    fn as_instance_mut(&mut self) -> &mut Contract<Chain> {
        &mut self.contract
    }
}

impl<Chain: CwEnv> Uploadable for Cw4GroupContract<Chain> {
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("cw4_group")
            .unwrap()
    }
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(
            cw4_group::contract::execute,
            cw4_group::contract::instantiate,
            cw4_group::contract::query,
        ))
    }
}

impl<Chain: CwEnv> Cw4GroupContract<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            contract: Contract::new("cw4_group", chain),
        }
    }
}

/// Voting module interfaces.
pub struct DaoVotingSuite<Chain: CwEnv> {
    pub voting_cw4: DaoVotingCw4<Chain>,
    pub voting_cw20_staked: DaoVotingCw20Staked<Chain>,
    pub voting_cw721_roles: DaoVotingCw721Roles<Chain>,
    pub voting_cw721_staked: DaoVotingCw721Staked<Chain>,
    pub voting_token_staked: DaoVotingTokenStaked<Chain>,
    /// cw4-group contract, uploaded separately for cw4-voting to use.
    pub cw4_group: Cw4GroupContract<Chain>,
    pub cw1_whitelist: Cw1WhitelistContract<Chain>,
    pub vote_delegation: DaoVoteDelegation<Chain>,
}

impl<Chain: CwEnv> DaoVotingSuite<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            voting_cw4: DaoVotingCw4::new("voting_cw4", chain.clone()),
            voting_cw20_staked: DaoVotingCw20Staked::new("voting_cw20_staked", chain.clone()),
            voting_cw721_roles: DaoVotingCw721Roles::new("voting_cw721_roles", chain.clone()),
            voting_cw721_staked: DaoVotingCw721Staked::new("voting_cw721_staked", chain.clone()),
            voting_token_staked: DaoVotingTokenStaked::new("voting_token_staked", chain.clone()),
            cw4_group: Cw4GroupContract::new(chain.clone()),
            cw1_whitelist: Cw1WhitelistContract::new(chain.clone()),
            vote_delegation: DaoVoteDelegation::new("dao_vote_delegation", chain),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.voting_cw4.upload()?;
        self.voting_cw20_staked.upload()?;
        self.voting_cw721_roles.upload()?;
        self.voting_cw721_staked.upload()?;
        self.voting_token_staked.upload()?;
        self.vote_delegation.upload()?;
        self.cw1_whitelist.upload()?;
        self.cw4_group.upload()?;
        Ok(())
    }

    pub fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![
            Box::new(&mut self.voting_cw4),
            Box::new(&mut self.voting_cw20_staked),
            Box::new(&mut self.voting_cw721_roles),
            Box::new(&mut self.voting_cw721_staked),
            Box::new(&mut self.voting_token_staked),
            Box::new(&mut self.vote_delegation),
            Box::new(&mut self.cw4_group),
        ]
    }
}

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoVotingSuite<Chain> {
    type Error = CwOrchError;
    type DeployData = DaoVotingDeployData;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let suite = Self::new(chain);
        suite.upload()?;
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
