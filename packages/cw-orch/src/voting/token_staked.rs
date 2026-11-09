use cw_orch::{interface, prelude::*};

use dao_voting_token_staked::contract::{execute, instantiate, migrate, query, reply};
use dao_voting_token_staked::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg)]
pub struct DaoVotingTokenStaked;

impl<Chain> Uploadable for DaoVotingTokenStaked<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(chain: &ChainInfoOwned) -> WasmPath {
        let dir = artifacts_dir_from_workspace!();
        if crate::is_terp_chain(chain) {
            return WasmPath::new(dir.path().join(crate::terp_wasm_file("dao_voting_token_staked")))
                .expect("dao_voting_token_staked-default.wasm");
        }
        dir.find_wasm_path("dao_voting_token_staked").unwrap()
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
