use cw_orch::{interface, prelude::*};

use dao_calendar::contract::{execute, instantiate, migrate, query};
use dao_calendar::contract::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

/// cw-orch interface for the dao-calendar contract.
///
/// Supports full scripting for DAO event lifecycle:
///   - CreateEvent / UpdateEvent / CancelEvent
///   - RegisterEventGauges / TriggerEventStart / TriggerEventEnd
///   - Group management (RegisterGroup / UpdateGroup / RemoveGroup)
///   - Hook management
///
/// Generic over metadata extension; defaults to `Empty` for base usage.
/// For custom metadata, define your own msg type aliases and interface impl.
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