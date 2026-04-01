use cw_orch::prelude::*;
use dao_cw_orch::{GaugeAdapter, GaugeOrchestrator};

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
