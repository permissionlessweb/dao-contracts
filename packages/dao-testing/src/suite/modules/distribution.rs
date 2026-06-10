use cw_orch::prelude::*;
use dao_cw_orch::*;

/// Deploy data for distribution modules (placeholder for future config).
#[derive(Clone, Debug, Default)]
pub struct DaoDistributionDeployData;

/// Distribution module interfaces.
pub struct DaoDistributionSuite<Chain: CwEnv> {
    pub fund_distr: DaoFundsDistributor<Chain>,
    pub reward_distr: DaoRewardsDistributor<Chain>,
}

impl<Chain: CwEnv> DaoDistributionSuite<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            fund_distr: DaoFundsDistributor::new("cw_funds_distributor", chain.clone()),
            reward_distr: DaoRewardsDistributor::new("dao_rewards_distributor", chain),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.fund_distr.upload()?;
        self.reward_distr.upload()?;
        Ok(())
    }

    pub fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![
            Box::new(&mut self.fund_distr),
            Box::new(&mut self.reward_distr),
        ]
    }
}

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoDistributionSuite<Chain> {
    type Error = CwOrchError;
    type DeployData = DaoDistributionDeployData;

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
