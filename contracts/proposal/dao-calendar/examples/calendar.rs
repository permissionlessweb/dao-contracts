use cw_orch::environment::ZkCwEnv;
use cw_orch::prelude::*;
pub use dao_calendar::contract::msg::ExecuteExtFns as _;
use dao_calendar::contract::MetadataExt;
use dao_testing::{DaoDaoDeployData, DaoDaoSuite};
use std::collections::HashMap;

/// Unified deployment suite composing all website contract suites.
pub struct MinDaoCalendarSuite<Chain: ZkCwEnv> {
    pub chain: Chain,
    pub dao: DaoDaoSuite<Chain>,
}

impl<Chain: ZkCwEnv> MinDaoCalendarSuite<Chain>
where
    cw_orch::prelude::CwOrchError: From<<Chain as cw_orch::prelude::TxHandler>::Error>,
{
    pub fn deploy_on(chain: Chain, data: DaoDaoDeployData) -> Result<Self, CwOrchError> {
        // spawn local test nostr relayer and client with dao suite data
        // deploy minimal default DAO with calendar for complete authority  + no wait for execute
        let dao = DaoDaoSuite::deploy_on(chain.clone(), data)?;
        // creat multiple events calendar events
        dao.proposal.calendar.create_event(
            String::default(),
            MetadataExt {
                kind: todo!(),
                on_chain: todo!(),
                e: todo!(),
                cid: todo!(),
                d_tag: todo!(),
                nostr_e_d: todo!(),
                author_pubkey: todo!(),
            },
        )?;

        Ok(Self { chain, dao })
    }

    /// Collect deployed contract addresses as a map (config key -> address).
    pub fn collect_addresses(&self) -> HashMap<String, String> {
        let mut addrs = HashMap::new();
        if let Ok(addr) = self.dao.dao_core.addr_str() {
            addrs.insert("daoCore".into(), addr);
        }
        addrs
    }

    /// Print deployed contract addresses in `CONTRACT_ADDR:name=addr` format.
    pub fn print_addresses(&self) {
        for (key, addr) in self.collect_addresses() {
            println!("CONTRACT_ADDR:{}={}", key, addr);
        }
    }
}

fn main() {}
