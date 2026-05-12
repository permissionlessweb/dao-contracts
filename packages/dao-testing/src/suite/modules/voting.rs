use cw_orch::prelude::*;
use dao_cw_orch::*;

/// Deploy data for voting modules (placeholder for future config).
#[derive(Clone, Debug, Default)]
pub struct DaoVotingDeployData;

/// Voting module interfaces.
pub struct DaoVotingSuite<Chain: CwEnv> {
    pub voting_cw4: DaoVotingCw4<Chain>,
    pub voting_cw20_staked: DaoVotingCw20Staked<Chain>,
    pub voting_cw721_roles: DaoVotingCw721Roles<Chain>,
    pub voting_cw721_staked: DaoVotingCw721Staked<Chain>,
    pub voting_token_staked: DaoVotingTokenStaked<Chain>,
}

impl<Chain: CwEnv> DaoVotingSuite<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            voting_cw4: DaoVotingCw4::new("voting_cw4", chain.clone()),
            voting_cw20_staked: DaoVotingCw20Staked::new("voting_cw20_staked", chain.clone()),
            voting_cw721_roles: DaoVotingCw721Roles::new("voting_cw721_roles", chain.clone()),
            voting_cw721_staked: DaoVotingCw721Staked::new("voting_cw721_staked", chain.clone()),
            voting_token_staked: DaoVotingTokenStaked::new("voting_token_staked", chain),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.voting_cw4.upload()?;
        self.voting_cw20_staked.upload()?;
        self.voting_cw721_roles.upload()?;
        self.voting_cw721_staked.upload()?;
        self.voting_token_staked.upload()?;
        Ok(())
    }

    pub fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![
            Box::new(&mut self.voting_cw4),
            Box::new(&mut self.voting_cw20_staked),
            Box::new(&mut self.voting_cw721_roles),
            Box::new(&mut self.voting_cw721_staked),
            Box::new(&mut self.voting_token_staked),
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
