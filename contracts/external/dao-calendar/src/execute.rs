use cosmwasm_std::{
    to_json_binary, Addr, Api, CosmosMsg, DepsMut, Empty, Env, MessageInfo, Order, Response,
    Storage, SubMsg, Timestamp, WasmMsg,
};
use cw_storage_plus::Bound;

use crate::contract::failed_event_hook_id;
use crate::error::ContractError;
use crate::msg::{
    AuthorizationQueryMsg, CalendarHookExecuteMsg, CalendarHookMsg, Event, EventGauge,
    EventService, EventServiceInit, EventStatus, EventSupplier, EventSupplierInit,
    EventSupplierType, Group, GroupInit, IsAuthorizedResponse, RecurrenceRule,
};
use crate::state::{DAO, EVENTS_BY_GROUP, EVENT_COUNT, EVENT_GAUGES, EVENT_HOOKS, EVENT_RECURRENCE, GROUPS, events};

// ═══════════════════════════ Auth Helpers ═══════════════════════════

fn is_dao(storage: &dyn Storage, sender: &Addr) -> Result<bool, ContractError> {
    let dao = DAO.load(storage)?;
    Ok(*sender == dao)
}

fn assert_dao(storage: &dyn Storage, sender: &Addr) -> Result<(), ContractError> {
    if !is_dao(storage, sender)? {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

/// Check if sender is authorized by any of the given managing groups.
/// The DAO is always authorized.
fn is_authorized(
    deps: &DepsMut,
    sender: &Addr,
    managing_groups: &[String],
) -> Result<bool, ContractError> {
    if is_dao(deps.storage, sender)? {
        return Ok(true);
    }
    for group_id in managing_groups {
        let group = GROUPS.may_load(deps.storage, group_id)?;
        if let Some(group) = group {
            for supplier in &group.suppliers {
                if supplier.supplier_type == EventSupplierType::Authorization {
                    let resp: IsAuthorizedResponse = deps.querier.query_wasm_smart(
                        &supplier.contract,
                        &AuthorizationQueryMsg::IsAuthorized {
                            sender: sender.to_string(),
                        },
                    )?;
                    if resp.authorized {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}

fn assert_authorized(
    deps: &DepsMut,
    sender: &Addr,
    managing_groups: &[String],
) -> Result<(), ContractError> {
    if !is_authorized(deps, sender, managing_groups)? {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

// ═══════════════════════════ Group Helpers ═══════════════════════════

pub fn save_group_from_init(
    api: &dyn Api,
    storage: &mut dyn Storage,
    dao: &Addr,
    init: GroupInit,
) -> Result<(), ContractError> {
    if GROUPS.has(storage, &init.id) {
        return Err(ContractError::GroupAlreadyExists { id: init.id });
    }
    let suppliers: Vec<EventSupplier> = init
        .suppliers
        .into_iter()
        .map(|s| {
            Ok(EventSupplier {
                contract: api.addr_validate(&s.contract)?,
                supplier_type: s.supplier_type,
            })
        })
        .collect::<Result<_, ContractError>>()?;
    let members: Option<Vec<Addr>> = init
        .members
        .map(|m| {
            m.into_iter()
                .filter_map(|addr| api.addr_validate(&addr).ok())
                .collect::<Vec<Addr>>()
        })
        .filter(|v: &Vec<Addr>| !v.is_empty());
    let group = Group {
        id: init.id.clone(),
        dao: dao.clone(),
        suppliers,
        name: init.name.unwrap_or_else(|| init.id.clone()),
        description: init.description,
        color: init.color,
        members,
    };
    GROUPS.save(storage, &init.id, &group)?;
    Ok(())
}

fn convert_services(services: Option<Vec<EventServiceInit>>) -> Vec<EventService> {
    services
        .unwrap_or_default()
        .into_iter()
        .map(|s| EventService {
            name: s.name,
            description: s.description,
            target_contract: s.target_contract,
            msgs: s.msgs,
        })
        .collect()
}

// ═══════════════════════════ Hook Helpers ═══════════════════════════

fn fire_calendar_hooks(
    storage: &dyn Storage,
    hook_msg: CalendarHookMsg,
) -> Result<Vec<SubMsg>, ContractError> {
    let msgs = EVENT_HOOKS.prepare_hooks(storage, |addr| {
        let msg = to_json_binary(&CalendarHookExecuteMsg::CalendarHook(hook_msg.clone()))?;
        let execute = WasmMsg::Execute {
            contract_addr: addr.to_string(),
            msg,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, failed_event_hook_id(0)))
    })?;
    Ok(msgs)
}

/// Collect gauge begin/end msgs for an event and wrap them into ExecuteProposalHook
/// messages through the DAO core.
fn collect_gauge_msgs<TMetadata>(
    storage: &dyn Storage,
    event: &Event<TMetadata>,
    is_begin: bool,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let dao = DAO.load(storage)?;
    let mut all_msgs: Vec<CosmosMsg> = Vec::new();

    // Collect event-specific gauge msgs
    if let Some(event_gauge) = EVENT_GAUGES.may_load(storage, event.id)? {
        let msgs = if is_begin {
            event_gauge.begin_msgs
        } else {
            event_gauge.end_msgs
        };
        all_msgs.extend(msgs);
    }

    // Wrap all msgs into ExecuteProposalHook through the DAO
    if !all_msgs.is_empty() {
        let execute_hook = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: dao.to_string(),
            msg: to_json_binary(&dao_interface::msg::ExecuteMsg::ExecuteProposalHook {
                msgs: all_msgs,
            })?,
            funds: vec![],
        });
        return Ok(vec![execute_hook]);
    }

    Ok(vec![])
}

// ═══════════════════════════ Execute Handlers ═══════════════════════════

#[allow(clippy::too_many_arguments)]
pub fn execute_create_event(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    description: Option<String>,
    start_time: Timestamp,
    end_time: Timestamp,
    managing_groups: Vec<String>,
    services: Option<Vec<EventServiceInit>>,
    extension: Option<Empty>,
    timezone: Option<String>,
    recurrence: Option<RecurrenceRule>,
) -> Result<Response, ContractError> {
    if start_time >= end_time {
        return Err(ContractError::InvalidTimeRange {});
    }

    // Validate timezone if provided
    if let Some(ref tz) = timezone {
        if !is_valid_timezone(tz) {
            return Err(ContractError::InvalidTimezone {
                tz: tz.clone(),
            });
        }
    }

    // Validate all groups exist
    for gid in &managing_groups {
        if !GROUPS.has(deps.storage, gid) {
            return Err(ContractError::NoSuchGroup { id: gid.clone() });
        }
    }

    // Check auth
    assert_authorized(&deps, &info.sender, &managing_groups)?;

    let event_id = EVENT_COUNT.load(deps.storage)? + 1;
    EVENT_COUNT.save(deps.storage, &event_id)?;

    let event: Event<Empty> = Event {
        id: event_id,
        title,
        description,
        start_time,
        end_time,
        timezone,
        managing_groups: managing_groups.clone(),
        event_services: convert_services(services),
        extension,
        status: EventStatus::Upcoming,
        created_by: info.sender.clone(),
        created_at: env.block.time,
    };

    events::<Empty>().save(deps.storage, event_id, &event)?;

    // Save join table entries
    for gid in &managing_groups {
        EVENTS_BY_GROUP.save(deps.storage, (gid.as_str(), event_id), &Empty {})?;
    }

    let hook_msgs = fire_calendar_hooks(
        deps.storage,
        CalendarHookMsg::EventCreated {
            event_id,
            creator: info.sender.to_string(),
        },
    )?;

    // Save recurrence rule if provided
    if let Some(rr) = recurrence {
        EVENT_RECURRENCE.save(deps.storage, event_id, &rr)?;
    }

    Ok(Response::new()
        .add_submessages(hook_msgs)
        .add_attribute("action", "create_event")
        .add_attribute("event_id", event_id.to_string())
        .add_attribute("creator", info.sender))
}

#[allow(clippy::too_many_arguments)]
pub fn execute_update_event(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    event_id: u64,
    title: Option<String>,
    description: Option<String>,
    start_time: Option<Timestamp>,
    end_time: Option<Timestamp>,
    managing_groups: Option<Vec<String>>,
    services: Option<Vec<EventServiceInit>>,
    extension: Option<Empty>,
    timezone: Option<String>,
) -> Result<Response, ContractError> {
    let mut event = events::<Empty>()
        .may_load(deps.storage, event_id)?
        .ok_or(ContractError::NoSuchEvent { id: event_id })?;

    // Can't update completed or cancelled events
    match event.status {
        EventStatus::Completed | EventStatus::Cancelled => {
            return Err(ContractError::EventNotUpcoming { id: event_id });
        }
        _ => {}
    }

    assert_authorized(&deps, &info.sender, &event.managing_groups)?;

    if let Some(t) = title {
        event.title = t;
    }
    if description.is_some() {
        event.description = description;
    }
    if let Some(st) = start_time {
        event.start_time = st;
    }
    if let Some(et) = end_time {
        event.end_time = et;
    }

    // Validate time range after potential updates
    if event.start_time >= event.end_time {
        return Err(ContractError::InvalidTimeRange {});
    }

    // Update managing groups if provided
    if let Some(new_groups) = managing_groups {
        for gid in &new_groups {
            if !GROUPS.has(deps.storage, gid) {
                return Err(ContractError::NoSuchGroup { id: gid.clone() });
            }
        }
        // Remove old join table entries
        for old_gid in &event.managing_groups {
            EVENTS_BY_GROUP.remove(deps.storage, (old_gid.as_str(), event_id));
        }
        // Add new join table entries
        for new_gid in &new_groups {
            EVENTS_BY_GROUP.save(deps.storage, (new_gid.as_str(), event_id), &Empty {})?;
        }
        event.managing_groups = new_groups;
    }

    if let Some(svc) = services {
        event.event_services = convert_services(Some(svc));
    }
    if extension.is_some() {
        event.extension = extension;
    }
    if let Some(tz) = timezone {
        if !is_valid_timezone(&tz) {
            return Err(ContractError::InvalidTimezone { tz });
        }
        event.timezone = Some(tz);
    }

    events::<Empty>().save(deps.storage, event_id, &event)?;

    Ok(Response::new()
        .add_attribute("action", "update_event")
        .add_attribute("event_id", event_id.to_string()))
}

pub fn execute_cancel_event(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    event_id: u64,
) -> Result<Response, ContractError> {
    let mut event = events::<Empty>()
        .may_load(deps.storage, event_id)?
        .ok_or(ContractError::NoSuchEvent { id: event_id })?;

    match event.status {
        EventStatus::Completed | EventStatus::Cancelled => {
            return Err(ContractError::EventNotUpcoming { id: event_id });
        }
        _ => {}
    }

    assert_authorized(&deps, &info.sender, &event.managing_groups)?;

    let old_status = event.status.to_string();
    event.status = EventStatus::Cancelled;
    events::<Empty>().save(deps.storage, event_id, &event)?;

    let hook_msgs = fire_calendar_hooks(
        deps.storage,
        CalendarHookMsg::EventCancelled { event_id },
    )?;

    Ok(Response::new()
        .add_submessages(hook_msgs)
        .add_attribute("action", "cancel_event")
        .add_attribute("event_id", event_id.to_string())
        .add_attribute("old_status", old_status))
}

pub fn execute_register_event_gauges(
    deps: DepsMut,
    info: MessageInfo,
    event_id: u64,
    begin_msgs: Vec<CosmosMsg>,
    end_msgs: Vec<CosmosMsg>,
) -> Result<Response, ContractError> {
    let event = events::<Empty>()
        .may_load(deps.storage, event_id)?
        .ok_or(ContractError::NoSuchEvent { id: event_id })?;

    assert_authorized(&deps, &info.sender, &event.managing_groups)?;

    let gauge = EventGauge {
        event_id,
        begin_msgs,
        end_msgs,
    };
    EVENT_GAUGES.save(deps.storage, event_id, &gauge)?;

    Ok(Response::new()
        .add_attribute("action", "register_event_gauges")
        .add_attribute("event_id", event_id.to_string()))
}

pub fn execute_trigger_event_start(
    deps: DepsMut,
    env: Env,
    event_id: u64,
) -> Result<Response, ContractError> {
    let mut event = events::<Empty>()
        .may_load(deps.storage, event_id)?
        .ok_or(ContractError::NoSuchEvent { id: event_id })?;

    if event.status != EventStatus::Upcoming {
        return Err(ContractError::EventNotUpcoming { id: event_id });
    }

    if env.block.time < event.start_time {
        return Err(ContractError::EventNotUpcoming { id: event_id });
    }

    let old_status = event.status.to_string();
    event.status = EventStatus::Active;
    events::<Empty>().save(deps.storage, event_id, &event)?;

    let gauge_msgs = collect_gauge_msgs(deps.storage, &event, true)?;
    let hook_msgs = fire_calendar_hooks(
        deps.storage,
        CalendarHookMsg::EventStatusChanged {
            event_id,
            old_status,
            new_status: event.status.to_string(),
        },
    )?;

    Ok(Response::new()
        .add_messages(gauge_msgs)
        .add_submessages(hook_msgs)
        .add_attribute("action", "trigger_event_start")
        .add_attribute("event_id", event_id.to_string()))
}

pub fn execute_trigger_event_end(
    deps: DepsMut,
    env: Env,
    event_id: u64,
) -> Result<Response, ContractError> {
    let mut event = events::<Empty>()
        .may_load(deps.storage, event_id)?
        .ok_or(ContractError::NoSuchEvent { id: event_id })?;

    if event.status != EventStatus::Active {
        return Err(ContractError::EventNotActive { id: event_id });
    }

    if env.block.time < event.end_time {
        return Err(ContractError::EventNotActive { id: event_id });
    }

    let old_status = event.status.to_string();
    event.status = EventStatus::Completed;
    events::<Empty>().save(deps.storage, event_id, &event)?;

    let gauge_msgs = collect_gauge_msgs(deps.storage, &event, false)?;
    let hook_msgs = fire_calendar_hooks(
        deps.storage,
        CalendarHookMsg::EventStatusChanged {
            event_id,
            old_status,
            new_status: event.status.to_string(),
        },
    )?;

    Ok(Response::new()
        .add_messages(gauge_msgs)
        .add_submessages(hook_msgs)
        .add_attribute("action", "trigger_event_end")
        .add_attribute("event_id", event_id.to_string()))
}

pub fn execute_register_group(
    deps: DepsMut,
    info: MessageInfo,
    group: GroupInit,
) -> Result<Response, ContractError> {
    assert_dao(deps.storage, &info.sender)?;

    let group_id = group.id.clone();
    save_group_from_init(deps.api, deps.storage, &info.sender, group)?;

    Ok(Response::new()
        .add_attribute("action", "register_group")
        .add_attribute("group_id", group_id))
}

pub fn execute_update_group(
    deps: DepsMut,
    info: MessageInfo,
    group_id: String,
    suppliers: Option<Vec<EventSupplierInit>>,
) -> Result<Response, ContractError> {
    assert_dao(deps.storage, &info.sender)?;

    let mut group = GROUPS
        .may_load(deps.storage, &group_id)?
        .ok_or(ContractError::NoSuchGroup {
            id: group_id.clone(),
        })?;

    if let Some(new_suppliers) = suppliers {
        group.suppliers = new_suppliers
            .into_iter()
            .map(|s| {
                Ok(EventSupplier {
                    contract: deps.api.addr_validate(&s.contract)?,
                    supplier_type: s.supplier_type,
                })
            })
            .collect::<Result<_, ContractError>>()?;
    }

    GROUPS.save(deps.storage, &group_id, &group)?;

    Ok(Response::new()
        .add_attribute("action", "update_group")
        .add_attribute("group_id", group_id))
}

pub fn execute_remove_group(
    deps: DepsMut,
    info: MessageInfo,
    group_id: String,
) -> Result<Response, ContractError> {
    assert_dao(deps.storage, &info.sender)?;

    if !GROUPS.has(deps.storage, &group_id) {
        return Err(ContractError::NoSuchGroup {
            id: group_id.clone(),
        });
    }

    GROUPS.remove(deps.storage, &group_id);

    // Clean up join table entries for this group
    let keys: Vec<(String, u64)> = EVENTS_BY_GROUP
        .prefix(&group_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(event_id, _)| (group_id.clone(), event_id)))
        .collect::<Result<_, _>>()?;

    for (gid, eid) in keys {
        EVENTS_BY_GROUP.remove(deps.storage, (&gid, eid));
    }

    Ok(Response::new()
        .add_attribute("action", "remove_group")
        .add_attribute("group_id", group_id))
}

pub fn execute_renew_calendar(
    deps: DepsMut,
    env: Env,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let limit = limit.unwrap_or(100) as usize;
    let cutoff_secs = env.block.time.seconds().saturating_sub(365 * 24 * 3600);

    // Range scan by_start index for old events
    let old_event_ids: Vec<u64> = events::<Empty>()
        .idx
        .by_start
        .range(
            deps.storage,
            None,
            Some(Bound::inclusive((cutoff_secs, u64::MAX))),
            Order::Ascending,
        )
        .take(limit)
        .filter_map(|r| r.ok())
        .map(|(_, event)| event.id)
        .collect();

    let pruned_count = old_event_ids.len();

    for event_id in old_event_ids {
        // Load event to get managing groups for join table cleanup
        if let Some(event) = events::<Empty>().may_load(deps.storage, event_id)? {
            for gid in &event.managing_groups {
                EVENTS_BY_GROUP.remove(deps.storage, (gid.as_str(), event_id));
            }
        }
        events::<Empty>().remove(deps.storage, event_id)?;
        EVENT_GAUGES.remove(deps.storage, event_id);
        EVENT_RECURRENCE.remove(deps.storage, event_id);
    }

    Ok(Response::new()
        .add_attribute("action", "renew_calendar")
        .add_attribute("pruned_count", pruned_count.to_string()))
}

pub fn execute_add_event_hook(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    assert_dao(deps.storage, &info.sender)?;
    let addr = deps.api.addr_validate(&address)?;
    EVENT_HOOKS.add_hook(deps.storage, addr)?;
    Ok(Response::new()
        .add_attribute("action", "add_event_hook")
        .add_attribute("address", address))
}

pub fn execute_remove_event_hook(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    assert_dao(deps.storage, &info.sender)?;
    let addr = deps.api.addr_validate(&address)?;
    EVENT_HOOKS.remove_hook(deps.storage, addr)?;
    Ok(Response::new()
        .add_attribute("action", "remove_event_hook")
        .add_attribute("address", address))
}

// ═══════════════════════════ Timezone Validation ═══════════════════════════

/// Validate an IANA timezone identifier string.
pub fn is_valid_timezone(tz: &str) -> bool {
    if tz.is_empty() || tz.len() > 64 {
        return false;
    }
    tz.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '_' || c == '-' || c == '+')
}
