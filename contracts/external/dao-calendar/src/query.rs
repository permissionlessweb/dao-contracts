use cosmwasm_std::{Deps, Empty, Order, StdResult, Timestamp};
use cw2::get_contract_version;
use cw_storage_plus::Bound;

use crate::msg::{
    AgendaResponse, DumpStateResponse, Event, EventFilter, EventGaugeResponse, EventListResponse, EventResponse,
    EventStatus, EventSupplierType, GaugeConfig, Group, GroupGaugeSummary, GroupListResponse,
    GroupResponse, GroupsManagingEventResponse, RecurrenceConfig, RecurrenceConfigResponse,
    RecurringInstancesResponse,
};
use crate::recurrence;
use crate::state::{DAO, EVENTS_BY_GROUP, EVENT_COUNT, EVENT_GAUGES, EVENT_RECURRENCE, GROUPS, events};
use dao_interface::voting::InfoResponse;

const DEFAULT_LIMIT: u32 = 30;
const MAX_LIMIT: u32 = 100;

fn clamp_limit(limit: Option<u32>) -> usize {
    limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize
}

fn event_to_response(id: u64, event: Event<Empty>) -> EventResponse<Empty> {
    EventResponse { id, event }
}

pub fn query_event(deps: Deps, event_id: u64) -> StdResult<EventResponse<Empty>> {
    let event = events::<Empty>().load(deps.storage, event_id)?;
    Ok(event_to_response(event_id, event))
}

pub fn query_list_events(
    deps: Deps,
    filter: Option<EventFilter>,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<EventListResponse<Empty>> {
    let limit = clamp_limit(limit);

    match filter {
        Some(EventFilter::ByGroups { groups }) => {
            query_events_by_groups(deps, &groups, start_after, limit)
        }
        Some(EventFilter::ByTimeRange {
            start_after: time_start,
            end_before,
        }) => query_events_by_time_range(deps, time_start, end_before, start_after, limit),
        Some(EventFilter::ByStatus { status }) => {
            query_events_by_status(deps, status, start_after, limit)
        }
        None => query_all_events(deps, start_after, limit, Order::Ascending),
    }
}

pub fn query_reverse_events(
    deps: Deps,
    filter: Option<EventFilter>,
    start_before: Option<u64>,
    limit: Option<u32>,
) -> StdResult<EventListResponse<Empty>> {
    let limit = clamp_limit(limit);

    match filter {
        None => {
            let max_bound = start_before.map(Bound::exclusive);
            let event_list: Vec<EventResponse<Empty>> = events::<Empty>()
                .range(deps.storage, None, max_bound, Order::Descending)
                .take(limit)
                .map(|r| r.map(|(id, event)| event_to_response(id, event)))
                .collect::<StdResult<_>>()?;
            Ok(EventListResponse {
                events: event_list,
            })
        }
        Some(EventFilter::ByStatus { status }) => {
            let status_key = status.as_u8();
            let event_list: Vec<EventResponse<Empty>> = events::<Empty>()
                .idx
                .by_status
                .prefix(status_key)
                .range(deps.storage, None, None, Order::Descending)
                .take(limit)
                .map(|r| r.map(|(id, event)| event_to_response(id, event)))
                .collect::<StdResult<_>>()?;
            Ok(EventListResponse {
                events: event_list,
            })
        }
        Some(f) => {
            // For other filters in reverse, use forward query then reverse
            let result = query_list_events(deps, Some(f), None, Some(limit as u32))?;
            let mut events_list = result.events;
            events_list.reverse();
            Ok(EventListResponse {
                events: events_list,
            })
        }
    }
}

fn query_all_events(
    deps: Deps,
    start_after: Option<u64>,
    limit: usize,
    order: Order,
) -> StdResult<EventListResponse<Empty>> {
    let min_bound = start_after.map(Bound::exclusive);
    let event_list: Vec<EventResponse<Empty>> = events::<Empty>()
        .range(deps.storage, min_bound, None, order)
        .take(limit)
        .map(|r| r.map(|(id, event)| event_to_response(id, event)))
        .collect::<StdResult<_>>()?;
    Ok(EventListResponse {
        events: event_list,
    })
}

fn query_events_by_groups(
    deps: Deps,
    groups: &[String],
    start_after: Option<u64>,
    limit: usize,
) -> StdResult<EventListResponse<Empty>> {
    let mut seen = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    let mut remaining = limit;

    for group_id in groups {
        if remaining == 0 {
            break;
        }
        let min_bound = start_after.map(Bound::<u64>::exclusive);
        // Lazy iteration: avoids collecting ALL event IDs into a Vec before processing.
        // Short-circuits once we've accumulated enough events across all groups.
        for item in EVENTS_BY_GROUP
            .prefix(group_id.as_str())
            .range(deps.storage, min_bound, None, Order::Ascending)
            .take(remaining)
        {
            let (eid, _) = item?;
            if seen.insert(eid) {
                if let Ok(event) = events::<Empty>().load(deps.storage, eid) {
                    result.push(event_to_response(eid, event));
                    remaining -= 1;
                    if remaining == 0 {
                        result.sort_by_key(|r| r.id);
                        return Ok(EventListResponse { events: result });
                    }
                }
            }
        }
    }

    result.sort_by_key(|r| r.id);
    result.truncate(limit);

    Ok(EventListResponse { events: result })
}

fn query_events_by_time_range(
    deps: Deps,
    time_start: Option<Timestamp>,
    time_end: Option<Timestamp>,
    start_after: Option<u64>,
    limit: usize,
) -> StdResult<EventListResponse<Empty>> {
    // Composite bounds on (start_seconds, event_id) — avoids in-memory post-filter
    let min_secs = time_start.map(|t| t.seconds());
    let max_secs = time_end.map(|t| t.seconds());

    let min_bound: Option<Bound<(u64, u64)>> = match (min_secs, start_after) {
        (Some(secs), Some(sa)) => Some(Bound::exclusive((secs, sa))),
        (Some(secs), None) => Some(Bound::inclusive((secs, 0))),
        (None, Some(sa)) => Some(Bound::exclusive((0, sa))),
        (None, None) => None,
    };
    let max_bound = max_secs.map(|s| Bound::inclusive((s, u64::MAX)));

    let event_list: Vec<EventResponse<Empty>> = events::<Empty>()
        .idx
        .by_start
        .range(deps.storage, min_bound, max_bound, Order::Ascending)
        .take(limit)
        .map(|r| r.map(|(id, event)| event_to_response(id, event)))
        .collect::<StdResult<_>>()?;

    Ok(EventListResponse { events: event_list })
}

fn query_events_by_status(
    deps: Deps,
    status: EventStatus,
    start_after: Option<u64>,
    limit: usize,
) -> StdResult<EventListResponse<Empty>> {
    let status_key = status.as_u8();
    let event_list: Vec<EventResponse<Empty>> = events::<Empty>()
        .idx
        .by_status
        .prefix(status_key)
        .range(
            deps.storage,
            start_after.map(Bound::exclusive),
            None,
            Order::Ascending,
        )
        .take(limit)
        .map(|r| r.map(|(id, event)| event_to_response(id, event)))
        .collect::<StdResult<_>>()?;
    Ok(EventListResponse {
        events: event_list,
    })
}

pub fn query_list_groups(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<GroupListResponse> {
    let limit = clamp_limit(limit);
    let min_bound = start_after.as_ref().map(|sa| Bound::exclusive(sa.as_str()));
    let groups: Vec<Group> = GROUPS
        .range(deps.storage, min_bound, None, Order::Ascending)
        .take(limit)
        .map(|r| r.map(|(_, group)| group))
        .collect::<StdResult<_>>()?;
    Ok(GroupListResponse { groups })
}

pub fn query_group(deps: Deps, group_id: &str) -> StdResult<GroupResponse> {
    let group = GROUPS.load(deps.storage, group_id)?;
    Ok(GroupResponse { group })
}

pub fn query_groups_managing_event(
    deps: Deps,
    event_id: u64,
) -> StdResult<GroupsManagingEventResponse> {
    let event = events::<Empty>().load(deps.storage, event_id)?;
    let mut groups = Vec::new();
    for gid in &event.managing_groups {
        if let Some(group) = GROUPS.may_load(deps.storage, gid)? {
            groups.push(group);
        }
    }
    Ok(GroupsManagingEventResponse { groups })
}

pub fn query_event_gauges(deps: Deps, event_id: u64) -> StdResult<EventGaugeResponse> {
    let event = events::<Empty>().load(deps.storage, event_id)?;

    let (begin_msgs, end_msgs) = match EVENT_GAUGES.may_load(deps.storage, event_id)? {
        Some(gauge) => (gauge.begin_msgs, gauge.end_msgs),
        None => (vec![], vec![]),
    };

    let mut group_gauges = Vec::new();
    for gid in &event.managing_groups {
        if let Some(group) = GROUPS.may_load(deps.storage, gid)? {
            let gauges: Vec<GaugeConfig> = group
                .suppliers
                .iter()
                .filter(|s| s.supplier_type == EventSupplierType::Gauge)
                .map(|s| GaugeConfig {
                    label: s.contract.to_string(),
                    begin_msgs: vec![],
                    end_msgs: vec![],
                })
                .collect();
            if !gauges.is_empty() {
                group_gauges.push(GroupGaugeSummary {
                    group_id: gid.clone(),
                    gauges,
                });
            }
        }
    }

    Ok(EventGaugeResponse {
        event_id,
        begin_msgs,
        end_msgs,
        group_gauges,
    })
}

pub fn query_event_count(deps: Deps) -> StdResult<u64> {
    EVENT_COUNT.load(deps.storage)
}

pub fn query_dump_state(deps: Deps) -> StdResult<DumpStateResponse> {
    let dao = DAO.load(deps.storage)?;
    let groups: Vec<Group> = GROUPS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(_, g)| g))
        .collect::<StdResult<_>>()?;
    let event_count = EVENT_COUNT.load(deps.storage)?;
    let contract_version = get_contract_version(deps.storage)?;

    Ok(DumpStateResponse {
        dao,
        groups,
        event_count,
        contract_version,
    })
}

pub fn query_info(deps: Deps) -> StdResult<InfoResponse> {
    let info = get_contract_version(deps.storage)?;
    Ok(InfoResponse { info })
}

// ═══════════════════════════ Recurrence Queries ═══════════════════════════

pub fn query_compute_recurring_instances(
    deps: Deps,
    event_id: u64,
    from: Option<Timestamp>,
    to: Option<Timestamp>,
    limit: Option<u32>,
) -> StdResult<RecurringInstancesResponse> {
    let event = events::<Empty>().load(deps.storage, event_id)?;
    let rule = match EVENT_RECURRENCE.may_load(deps.storage, event_id)? {
        Some(r) => r,
        None => {
            return Ok(RecurringInstancesResponse {
                instances: vec![],
                has_more: false,
            });
        }
    };

    let (instances, has_more) = recurrence::compute_instances(
        &rule,
        event.start_time,
        event.end_time,
        from,
        to,
        limit,
    );

    Ok(RecurringInstancesResponse {
        instances,
        has_more,
    })
}

pub fn query_event_recurrence(deps: Deps, event_id: u64) -> StdResult<RecurrenceConfigResponse> {
    let rule = EVENT_RECURRENCE.may_load(deps.storage, event_id)?;
    let config = rule.map(|r| RecurrenceConfig { event_id, rule: r });
    Ok(RecurrenceConfigResponse { config })
}

// ═══════════════════════════ Agenda Query ═══════════════════════════

const AGENDA_DEFAULT_LIMIT: u32 = 10;
const AGENDA_MAX_LIMIT: u32 = 50;

/// Returns upcoming and active events sorted by start_time — a compact dashboard view.
/// Filters to `Upcoming` and `Active` status events, ordered ascending by start_time.
pub fn query_agenda(
    deps: Deps,
    from: Option<Timestamp>,
    limit: Option<u32>,
) -> StdResult<AgendaResponse> {
    let limit = limit
        .unwrap_or(AGENDA_DEFAULT_LIMIT)
        .min(AGENDA_MAX_LIMIT) as usize;
    let from_secs = from.map(|t| t.seconds()).unwrap_or(0);

    // Walk the by_start index from `from` forward, collecting Upcoming + Active events
    let min_bound: Option<Bound<(u64, u64)>> = Some(Bound::inclusive((from_secs, 0)));

    let mut results: Vec<EventResponse<Empty>> = Vec::new();

    for item in events::<Empty>()
        .idx
        .by_start
        .range(deps.storage, min_bound, None, Order::Ascending)
    {
        let (_pk, event) = item?;
        match event.status {
            EventStatus::Upcoming | EventStatus::Active => {
                results.push(EventResponse {
                    id: event.id,
                    event,
                });
                if results.len() >= limit {
                    break;
                }
            }
            _ => continue,
        }
    }

    let count = results.len() as u64;
    Ok(AgendaResponse {
        events: results,
        count,
    })
}
