use cosmwasm_schema::QueryResponses;
use cosmwasm_std::{Addr, CosmosMsg, Empty, Timestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ═══════════════════════════ CW721-style Generic Extension ═══════════════════════════

/// Core event type, generic over a metadata extension `T`.
/// Follows the CW721 extension pattern: any type satisfying the trait bounds
/// can be used as event metadata, giving DAOs complete modularity.
///
/// Use `Event<Empty>` (the default) for events with no custom metadata.
/// Define your own extension type for richer metadata:
/// ```rust,ignore
/// #[cw_serde]
/// pub struct MeetingMetadata {
///     pub location: String,
///     pub agenda: Vec<String>,
///     pub virtual_link: Option<String>,
/// }
/// type MeetingEvent = Event<MeetingMetadata>;
/// ```
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Event<TMetadata = Empty> {
    pub id: u64,
    pub title: String,
    pub description: Option<String>,
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub managing_groups: Vec<String>,
    pub event_services: Vec<EventService>,
    /// CW721-compatible metadata extension. Any serializable type accepted.
    pub extension: Option<TMetadata>,
    pub status: EventStatus,
    pub created_by: Addr,
    pub created_at: Timestamp,
}

// ═══════════════════════════ Event Status ═══════════════════════════

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Upcoming,
    Active,
    Completed,
    Cancelled,
}

impl EventStatus {
    pub fn as_u8(&self) -> u8 {
        match self {
            EventStatus::Upcoming => 0,
            EventStatus::Active => 1,
            EventStatus::Completed => 2,
            EventStatus::Cancelled => 3,
        }
    }
}

impl std::fmt::Display for EventStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventStatus::Upcoming => write!(f, "upcoming"),
            EventStatus::Active => write!(f, "active"),
            EventStatus::Completed => write!(f, "completed"),
            EventStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

// ═══════════════════════════ Supporting Types ═══════════════════════════

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EventService {
    pub name: String,
    pub description: Option<String>,
    pub target_contract: Option<String>,
    pub msgs: Vec<CosmosMsg>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Group {
    pub id: String,
    pub dao: Addr,
    pub suppliers: Vec<EventSupplier>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EventSupplier {
    pub contract: Addr,
    pub supplier_type: EventSupplierType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventSupplierType {
    Authorization,
    Gauge,
    Service,
    Account,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GaugeConfig {
    pub label: String,
    pub begin_msgs: Vec<CosmosMsg>,
    pub end_msgs: Vec<CosmosMsg>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EventGauge {
    pub event_id: u64,
    pub begin_msgs: Vec<CosmosMsg>,
    pub end_msgs: Vec<CosmosMsg>,
}

// ═══════════════════════════ Init Types ═══════════════════════════

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EventServiceInit {
    pub name: String,
    pub description: Option<String>,
    pub target_contract: Option<String>,
    pub msgs: Vec<CosmosMsg>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GroupInit {
    pub id: String,
    pub suppliers: Vec<EventSupplierInit>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EventSupplierInit {
    pub contract: String,
    pub supplier_type: EventSupplierType,
}

// ═══════════════════════════ Messages ═══════════════════════════

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub initial_groups: Option<Vec<GroupInit>>,
}

/// Execute messages, generic over the metadata extension type `TMetadata`.
/// Use `ExecuteMsg<Empty>` (the default) for events with no custom metadata.
/// Custom contracts can define `type MyExecuteMsg = ExecuteMsg<MyMetadata>`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg<TMetadata = Empty> {
    CreateEvent {
        title: String,
        description: Option<String>,
        start_time: Timestamp,
        end_time: Timestamp,
        managing_groups: Vec<String>,
        services: Option<Vec<EventServiceInit>>,
        extension: Option<TMetadata>,
    },

    UpdateEvent {
        event_id: u64,
        title: Option<String>,
        description: Option<String>,
        start_time: Option<Timestamp>,
        end_time: Option<Timestamp>,
        managing_groups: Option<Vec<String>>,
        services: Option<Vec<EventServiceInit>>,
        extension: Option<TMetadata>,
    },

    CancelEvent {
        event_id: u64,
    },

    RegisterEventGauges {
        event_id: u64,
        begin_msgs: Vec<CosmosMsg>,
        end_msgs: Vec<CosmosMsg>,
    },

    TriggerEventStart {
        event_id: u64,
    },

    TriggerEventEnd {
        event_id: u64,
    },

    RegisterGroup {
        group: GroupInit,
    },

    UpdateGroup {
        group_id: String,
        suppliers: Option<Vec<EventSupplierInit>>,
    },

    RemoveGroup {
        group_id: String,
    },

    RenewCalendar {
        limit: Option<u32>,
    },

    AddEventHook {
        address: String,
    },

    RemoveEventHook {
        address: String,
    },
}

/// Query messages. Non-generic — response types use default `Empty` extension.
/// Custom contracts override at the handler level to return their typed extension.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, QueryResponses)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    #[returns(EventResponse)]
    Event {
        event_id: u64,
    },

    #[returns(EventListResponse)]
    ListEvents {
        filter: Option<EventFilter>,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(EventListResponse)]
    ReverseEvents {
        filter: Option<EventFilter>,
        start_before: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(GroupListResponse)]
    ListGroups {},

    #[returns(GroupResponse)]
    Group {
        group_id: String,
    },

    #[returns(GroupsManagingEventResponse)]
    GroupsManagingEvent {
        event_id: u64,
    },

    #[returns(EventGaugeResponse)]
    EventGauges {
        event_id: u64,
    },

    #[returns(u64)]
    EventCount {},

    #[returns(cw_hooks::HooksResponse)]
    EventHooks {},

    #[returns(DumpStateResponse)]
    DumpState {},

    /// Returns the address of the DAO this module belongs to.
    #[returns(Addr)]
    Dao {},

    /// Returns contract version info.
    #[returns(dao_interface::voting::InfoResponse)]
    Info {},

    /// Returns the next event ID that will be assigned.
    #[returns(u64)]
    NextProposalId {},
}

// ═══════════════════════════ Filters ═══════════════════════════

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventFilter {
    ByGroups { groups: Vec<String> },
    ByTimeRange {
        start_after: Option<Timestamp>,
        end_before: Option<Timestamp>,
    },
    ByStatus { status: EventStatus },
}

// ═══════════════════════════ Responses ═══════════════════════════

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EventResponse<TMetadata = Empty> {
    pub id: u64,
    pub event: Event<TMetadata>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EventListResponse<TMetadata = Empty> {
    pub events: Vec<EventResponse<TMetadata>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GroupResponse {
    pub group: Group,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GroupListResponse {
    pub groups: Vec<Group>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GroupsManagingEventResponse {
    pub groups: Vec<Group>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EventGaugeResponse {
    pub event_id: u64,
    pub begin_msgs: Vec<CosmosMsg>,
    pub end_msgs: Vec<CosmosMsg>,
    pub group_gauges: Vec<GroupGaugeSummary>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GroupGaugeSummary {
    pub group_id: String,
    pub gauges: Vec<GaugeConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct DumpStateResponse {
    pub dao: Addr,
    pub groups: Vec<Group>,
    pub event_count: u64,
    pub contract_version: cw2::ContractVersion,
}

// ═══════════════════════════ Migrate ═══════════════════════════

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MigrateMsg {
    FromCompatible {},
}

// ═══════════════════════════ Hook Messages ═══════════════════════════

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CalendarHookMsg {
    EventCreated { event_id: u64, creator: String },
    EventStatusChanged {
        event_id: u64,
        old_status: String,
        new_status: String,
    },
    EventCancelled { event_id: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CalendarHookExecuteMsg {
    CalendarHook(CalendarHookMsg),
}

// ═══════════════════════════ Authorization Interface ═══════════════════════════

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationQueryMsg {
    IsAuthorized { sender: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct IsAuthorizedResponse {
    pub authorized: bool,
}

// ═══════════════════════════ Default Type Aliases ═══════════════════════════
// The base contract uses Empty extension. Custom contracts define their own
// metadata types and create aliases like:
//   pub type MyEvent = Event<MyMetadata>;
//   pub type MyExecuteMsg = ExecuteMsg<MyMetadata>;

pub type DefaultEvent = Event<Empty>;
pub type DefaultExecuteMsg = ExecuteMsg<Empty>;
pub type DefaultEventResponse = EventResponse<Empty>;
pub type DefaultEventListResponse = EventListResponse<Empty>;
