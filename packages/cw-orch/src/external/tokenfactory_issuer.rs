use cw_orch::{interface, prelude::*};

use cw_tokenfactory_issuer::contract::{execute, instantiate, migrate, query, reply};
use cw_tokenfactory_issuer::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
pub struct DaoExternalTokenfactoryIssuer;

impl<Chain> Uploadable for DaoExternalTokenfactoryIssuer<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(chain: &ChainInfoOwned) -> WasmPath {
        let dir = artifacts_dir_from_workspace!();
        if crate::is_terp_chain(chain) {
            return WasmPath::new(dir.path().join(crate::terp_wasm_file("cw_tokenfactory_issuer")))
                .expect("cw_tokenfactory_issuer-osmosis.wasm");
        }
        dir.find_wasm_path("cw_tokenfactory_issuer").unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(execute, instantiate, query)
                .with_reply(reply)
                .with_migrate(migrate),
        )
    }
}
