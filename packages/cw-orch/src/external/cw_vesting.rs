use cw_orch::{interface, prelude::*};

use cw_vesting::contract::{execute, instantiate, query};
use cw_vesting::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
pub struct DaoExternalCwVesting;

impl<Chain> Uploadable for DaoExternalCwVesting<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(chain: &ChainInfoOwned) -> WasmPath {
        let dir = artifacts_dir_from_workspace!();
        if crate::is_terp_chain(chain) {
            return WasmPath::new(dir.path().join(crate::terp_wasm_file("cw_vesting")))
                .expect("cw_vesting-staking.wasm");
        }
        dir.find_wasm_path("cw_vesting").unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
    }
}
