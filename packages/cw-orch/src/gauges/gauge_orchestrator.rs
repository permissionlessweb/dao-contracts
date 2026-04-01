use cw_orch::{interface, prelude::*};

use gauge_orchestrator::contract::{execute, instantiate, query};
use gauge_orchestrator::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = "gauge-orchestrator")]
pub struct GaugeOrchestrator;

impl<Chain> Uploadable for GaugeOrchestrator<Chain> {
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("gauge_orchestrator")
            .unwrap()
    }

    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
    }
}
