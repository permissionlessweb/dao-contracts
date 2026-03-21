use cw_orch::prelude::*;
use dao_cw_orch::*;

/// Staking module interfaces.
pub struct DaoStakingSuite<Chain: CwEnv> {
    pub cw20_stake: DaoStakingCw20<Chain>,
    pub external_rewards: DaoStakingCw20ExternalRewards<Chain>,
    pub rewards_distributor: DaoStakingCw20RewardDistributor<Chain>,
}

impl<Chain: CwEnv> DaoStakingSuite<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            cw20_stake: DaoStakingCw20::new("cw20_stake", chain.clone()),
            external_rewards: DaoStakingCw20ExternalRewards::new(
                "cw20_external_rewards",
                chain.clone(),
            ),
            rewards_distributor: DaoStakingCw20RewardDistributor::new(
                "cw20_reward_distributor",
                chain,
            ),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.cw20_stake.upload()?;
        self.external_rewards.upload()?;
        self.rewards_distributor.upload()?;
        Ok(())
    }

    pub fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![
            Box::new(&mut self.cw20_stake),
            Box::new(&mut self.external_rewards),
            Box::new(&mut self.rewards_distributor),
        ]
    }
}

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoStakingSuite<Chain> {
    type Error = CwOrchError;
    type DeployData = Addr;

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
