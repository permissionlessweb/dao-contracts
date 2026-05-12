use cw_orch::prelude::*;
use dao_cw_orch::{GaugeAdapter, GaugeOrchestrator};

/// Deploy data for gauge modules (placeholder for future config).
#[derive(Clone, Debug, Default)]
pub struct DaoGaugeDeployData;

/// Gauge module interfaces.
pub struct DaoGaugeSuite<Chain: CwEnv> {
    pub orchestrator: GaugeOrchestrator<Chain>,
    pub adapter: GaugeAdapter<Chain>,
}

impl<Chain: CwEnv> DaoGaugeSuite<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            orchestrator: GaugeOrchestrator::new(chain.clone()),
            adapter: GaugeAdapter::new(chain),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.orchestrator.upload()?;
        self.adapter.upload()?;
        Ok(())
    }

    pub fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![
            Box::new(&mut self.orchestrator),
            Box::new(&mut self.adapter),
        ]
    }
}

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoGaugeSuite<Chain> {
    type Error = CwOrchError;
    type DeployData = DaoGaugeDeployData;

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
