use cw_orch::{interface, prelude::*};

use gauge_adapter::contract::{execute, instantiate, query};
use gauge_adapter::msg::{AdapterQueryMsg, ExecuteMsg, InstantiateMsg, MigrateMsg};

#[interface(InstantiateMsg, ExecuteMsg, AdapterQueryMsg, MigrateMsg, id = "gauge-adapter")]
pub struct GaugeAdapter;

impl<Chain> Uploadable for GaugeAdapter<Chain> {
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("gauge_adapter")
            .unwrap()
    }

    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
    }
}
