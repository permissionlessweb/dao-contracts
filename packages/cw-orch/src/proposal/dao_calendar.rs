use cw_orch::{interface, prelude::*};

use dao_calendar::contract::{execute, instantiate, migrate, query};
use dao_calendar::contract::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

/// cw-orch interface for the **dao-calendar** proposal module (CW721 + NIP-52).
///
/// With `dao-calendar/features = ["interface"]`, `ExecuteExtFns` / `QueryExtFns`
/// are available for remote-control scripts:
///
/// | Plane | Methods |
/// |-------|---------|
/// | **Display** | `config`, `dao`, `list_calendars`, `calendar`, `calendar_events`, `calendar_event`, `event_count` |
/// | **Lifecycle** | `create_calendar`, `create_event`, `update_event`, `cancel_event` |
/// | **Admin (DAO)** | `update_pre_propose_info`, `add_calendar_hook`, … |
///
/// Mesh (`mesh.v1`) and governance votes live on **dao-dao-core** /
/// **dao-proposal-single** — see `dao-scripts` calendar remote CLI.
///
/// Token ID layout:
/// - calendars: `cal/{n}`
/// - events: `evt/{cal_d}/{n}` or standalone `evt/_/{n}`
#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = "dao-calendar")]
pub struct DaoCalendar;

impl<Chain> Uploadable for DaoCalendar<Chain> {
    fn wasm(_chain: &ChainInfoOwned) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("dao_calendar")
            .unwrap()
    }

    fn wrapper() -> Box<dyn MockContract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(execute, instantiate, query).with_migrate(migrate))
    }
}
