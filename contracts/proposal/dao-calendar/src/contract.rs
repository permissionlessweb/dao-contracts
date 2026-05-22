use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Addr, Binary, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo, MigrateInfo, Reply, Response,
    StdResult,
};
use cw2::set_contract_version;
use cw721::{extension::Cw721Extensions, msg::Cw721InstantiateMsg, traits::Cw721Execute};
use cw_hooks::Hooks;
use cw_storage_plus::{Item, Map};
use cw_utils::{Duration, DAY};
use dao_voting::{
    pre_propose::{PreProposeInfo, ProposalCreationPolicy},
    veto::VetoConfig,
    voting::validate_voting_period,
};

pub type DaoNostrCalendar<'a> = Cw721Extensions<
    'a,
    MetadataExt,                       // TNftExtension
    MetadataExt,                       // TNftExtensionMsg
    CalendarModuleCollectionExtension, // TCollectionExtension
    CalendarModuleCollectionExtension, // TCollectionExtensionMsg
    ExecuteExt,                        // TExtensionMsg
    QueryExt,                          // TExtensionQueryMsg
    Empty,                             // TCustomResponseMsg
>;

pub type InstantiateMsg = cw721::msg::Cw721InstantiateMsg<CalendarModuleCollectionExtension>;
pub type ExecuteMsg =
    cw721::msg::Cw721ExecuteMsg<MetadataExt, CalendarModuleCollectionExtension, ExecuteExt>;
pub type QueryMsg =
    cw721::msg::Cw721QueryMsg<MetadataExt, CalendarModuleCollectionExtension, QueryExt>;
#[cw_serde]
pub struct MetadataExt {
    /// `true` = full Nostr event stored on-chain in `e`.
    /// `false` = only an IPFS CID pointer lives on-chain.
    pub on_chain: bool,
    /// Encoded NIP-52 `CalendarEventMetadata` (on-chain).
    /// Empty `Binary` when off-chain.
    pub e: Binary,
    /// IPFS CID pointing to the full Nostr event JSON (off-chain only).
    pub cid: Option<String>,
    /// Optional IPFS gateway URL for content resolution.
    pub gateway: Option<String>,
    /// Nostr event kind (31922 date-based, 31923 time-based).
    /// Always populated for efficient on-chain filtering.
    pub kind: u16,
    /// Cached d-tag for addressable events (kind 30000–39999).
    /// Avoids decoding `e` just to read the identifier.
    pub d_tag: Option<String>,
    /// Optional: Store the raw nostr event ID for verification.
    pub nostr_event_id: Option<String>,
    /// Optional: Store the author's pubkey.
    pub author_pubkey: Option<String>,
}

impl Default for CalendarModuleCollectionExtension {
    fn default() -> Self {
        Self {
            min_event_period: None,
            max_event_period: DAY * 365,
            pre_propose_info: PreProposeInfo::AnyoneMayPropose {},
            veto: None,
            delegation_module: None,
        }
    }
}

#[cw_serde]
pub struct CalendarModuleCollectionExtension {
    pub min_event_period: Option<Duration>,
    pub max_event_period: Duration,
    pub pre_propose_info: PreProposeInfo,
    pub veto: Option<VetoConfig>,
    pub delegation_module: Option<String>,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const DAO: Item<Addr> = Item::new("dao");
pub const CALENDAR_COUNT: Item<u64> = Item::new("calendar_count");
pub const CALENDARS: Map<u64, Calendar> = Map::new("calendars");
pub const EVENT_COUNT: Map<u64, u64> = Map::new("event_count");
pub const EVENTS: Map<(u64, u64), CalendarEvent> = Map::new("events");
pub const CREATION_POLICY: Item<ProposalCreationPolicy> = Item::new("creation_policy");
pub const DELEGATION_MODULE: Item<Addr> = Item::new("delegation_module");
pub const CALENDAR_HOOKS: Hooks = Hooks::new("calendar_hooks");
pub const CONTRACT_NAME: &str = "crates.io:dao-calendar";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const FAILED_HOOK_REPLY_ID_BASE: u64 = 1_000_000;

pub mod state {
    use super::*;
    use cw721::{
        error::Cw721ContractError,
        traits::{
            Contains, Cw721CustomMsg, Cw721State, FromAttributesState, StateFactory,
            ToAttributesState,
        },
    };

    impl Cw721State for MetadataExt {}
    impl Cw721CustomMsg for MetadataExt {}
    impl Contains for MetadataExt {
        fn contains(&self, other: &Self) -> bool {
            self == other
        }
    }

    // ── NIP-52 integration ──────────────────────────────────────────
    use cw721_nips::{
        cw::NostrCw721Ext,
        error::{NipError, NipResult},
        nips::nip52::{CalendarEventMetadata, Nip52Kind},
        NipMetadata, NostrExt, RawNostrEvent,
    };

    impl NipMetadata for MetadataExt {
        type Kind = Nip52Kind;

        fn kind(&self) -> Self::Kind {
            if self.on_chain {
                CalendarEventMetadata::from_storage_binary(&self.e)
                    .map(|m| m.kind())
                    .unwrap_or(Nip52Kind::TimeEvent)
            } else {
                match self.kind {
                    31922 => Nip52Kind::DateEvent,
                    _ => Nip52Kind::TimeEvent,
                }
            }
        }

        fn validate(&self) -> NipResult<()> {
            if self.on_chain {
                if self.e.is_empty() {
                    return Err(NipError::Validation(
                        "No calendar event data in MetadataExt.e".into(),
                    ));
                }
                CalendarEventMetadata::from_storage_binary(&self.e)
                    .map_err(|e| NipError::Validation(format!("Failed to decode: {e}")))?
                    .validate()
            } else {
                if self.cid.is_none() {
                    return Err(NipError::Validation(
                        "Off-chain metadata must have a CID".into(),
                    ));
                }
                Ok(())
            }
        }

        fn to_tags(&self) -> Vec<cw721_nips::Tag> {
            if self.on_chain {
                CalendarEventMetadata::from_storage_binary(&self.e)
                    .map(|m| m.to_tags())
                    .unwrap_or_default()
            } else {
                let mut tags: Vec<cw721_nips::Tag> = vec![];
                if let Some(gw) = &self.gateway {
                    tags.push(cw721_nips::Tag::new(vec![
                        "gateway".to_string(),
                        gw.clone(),
                    ]));
                }
                tags
            }
        }

        fn content(&self) -> String {
            if self.on_chain {
                CalendarEventMetadata::from_storage_binary(&self.e)
                    .map(|m| m.content())
                    .unwrap_or_default()
            } else {
                self.cid.clone().unwrap_or_default()
            }
        }

        fn d_tag(&self) -> Option<String> {
            self.d_tag.clone()
        }

        fn from_raw_event(event: &RawNostrEvent) -> NipResult<Self> {
            let inner = CalendarEventMetadata::from_raw_event(event)?;
            let d_tag = inner.d_tag();
            let e = Binary::from(serde_json::to_vec(&inner)?);
            Ok(Self {
                on_chain: true,
                e,
                cid: None,
                gateway: None,
                kind: event.kind,
                d_tag,
                nostr_event_id: Some(event.id.clone()),
                author_pubkey: Some(event.pubkey.clone()),
            })
        }
    }

    // ── NostrExt (on-chain / off-chain discriminator) ──────────────
    impl NostrExt for MetadataExt {
        fn location(&self) -> bool {
            if self.on_chain {
                true
            } else {
                false
            }
        }

        fn kind(&self) -> u16 {
            self.kind
        }

        fn d_tag(&self) -> Option<&str> {
            self.d_tag.as_deref()
        }

        fn into_event(self, pubkey: &str, created_at: u64) -> NipResult<RawNostrEvent> {
            if self.on_chain {
                let inner = CalendarEventMetadata::from_storage_binary(&self.e)
                    .map_err(|e| NipError::Validation(format!("{e}")))?;
                let tags: Vec<Vec<String>> = inner
                    .to_tags()
                    .into_iter()
                    .map(|t| t.into_inner())
                    .collect();
                let content = inner.content();
                let mut event = RawNostrEvent {
                    id: Default::default(),
                    pubkey: self.author_pubkey.unwrap_or_else(|| pubkey.to_string()),
                    created_at,
                    kind: self.kind,
                    tags,
                    content,
                    sig: Default::default(),
                };
                event.id = event.compute_id()?;
                Ok(event)
            } else {
                Err(NipError::MissingField(
                    "Off-chain events must be resolved via CID; use the IPFS pointer".into(),
                ))
            }
        }
    }

    impl Cw721State for CalendarModuleCollectionExtension {}
    impl Cw721CustomMsg for CalendarModuleCollectionExtension {}

    impl Contains for CalendarModuleCollectionExtension {
        fn contains(&self, other: &Self) -> bool {
            self == other
        }
    }
    impl ToAttributesState for CalendarModuleCollectionExtension {
        fn to_attributes_state(&self) -> Result<Vec<cw721::Attribute>, Cw721ContractError> {
            use cosmwasm_std::to_json_binary;
            Ok(vec![
                cw721::Attribute {
                    key: "min_event_period".to_string(),
                    value: to_json_binary(&self.min_event_period)?,
                },
                cw721::Attribute {
                    key: "max_event_period".to_string(),
                    value: to_json_binary(&self.max_event_period)?,
                },
                cw721::Attribute {
                    key: "pre_propose_info".to_string(),
                    value: to_json_binary(&self.pre_propose_info)?,
                },
                cw721::Attribute {
                    key: "veto".to_string(),
                    value: to_json_binary(&self.veto)?,
                },
                cw721::Attribute {
                    key: "delegation_module".to_string(),
                    value: to_json_binary(&self.delegation_module)?,
                },
            ])
        }
    }
    impl FromAttributesState for CalendarModuleCollectionExtension {
        fn from_attributes_state(value: &[cw721::Attribute]) -> Result<Self, Cw721ContractError> {
            let mut min_event_period = None;
            let mut max_event_period = None;
            let mut pre_propose_info = None;
            let mut veto = None;
            let mut delegation_module = None;

            for attr in value {
                match attr.key.as_str() {
                    "min_event_period" => {
                        min_event_period = Some(attr.value::<Option<Duration>>()?);
                    }
                    "max_event_period" => {
                        max_event_period = Some(attr.value::<Duration>()?);
                    }
                    "pre_propose_info" => {
                        pre_propose_info = Some(attr.value::<PreProposeInfo>()?);
                    }
                    "veto" => {
                        veto = Some(attr.value::<Option<VetoConfig>>()?);
                    }
                    "delegation_module" => {
                        delegation_module = Some(attr.value::<Option<String>>()?);
                    }
                    _ => {}
                }
            }

            Ok(Self {
                min_event_period: min_event_period.ok_or_else(|| {
                    Cw721ContractError::Std(cosmwasm_std::StdError::msg("Missing min_event_period"))
                })?,
                max_event_period: max_event_period.ok_or_else(|| {
                    Cw721ContractError::Std(cosmwasm_std::StdError::msg("Missing max_event_period"))
                })?,
                pre_propose_info: pre_propose_info.ok_or_else(|| {
                    Cw721ContractError::Std(cosmwasm_std::StdError::msg("Missing pre_propose_info"))
                })?,
                veto: veto.ok_or_else(|| {
                    Cw721ContractError::Std(cosmwasm_std::StdError::msg("Missing veto"))
                })?,
                delegation_module: delegation_module.ok_or_else(|| {
                    Cw721ContractError::Std(cosmwasm_std::StdError::msg(
                        "Missing delegation_module",
                    ))
                })?,
            })
        }
    }

    impl StateFactory<CalendarModuleCollectionExtension> for CalendarModuleCollectionExtension {
        fn create(
            &self,
            _deps: Deps,
            _env: &Env,
            _info: Option<&MessageInfo>,
            _current: Option<&CalendarModuleCollectionExtension>,
        ) -> Result<CalendarModuleCollectionExtension, Cw721ContractError> {
            Ok(self.clone())
        }

        fn validate(
            &self,
            _deps: Deps,
            _env: &Env,
            _info: Option<&MessageInfo>,
            _current: Option<&CalendarModuleCollectionExtension>,
        ) -> Result<(), Cw721ContractError> {
            Ok(())
        }
    }

    impl CustomMsg for QueryExt {}
    impl Cw721CustomMsg for QueryExt {}

    impl StateFactory<MetadataExt> for MetadataExt {
        fn create(
            &self,
            _deps: Deps,
            _env: &Env,
            _info: Option<&MessageInfo>,
            _current: Option<&MetadataExt>,
        ) -> Result<MetadataExt, Cw721ContractError> {
            Ok(self.clone())
        }

        fn validate(
            &self,
            _deps: Deps,
            _env: &Env,
            _info: Option<&MessageInfo>,
            _current: Option<&MetadataExt>,
        ) -> Result<(), Cw721ContractError> {
            Ok(())
        }
    }
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

#[cosmwasm_schema::cw_serde]
pub struct Config {
    pub min_event_period: Option<Duration>,
    pub max_event_period: Duration,
    pub veto: Option<VetoConfig>,
}

#[derive(thiserror::Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    HookError(#[from] cw_hooks::HookError),

    #[error(transparent)]
    Cw721ContractError(#[from] cw721::error::Cw721ContractError),

    #[error(transparent)]
    VetoError(#[from] dao_voting::veto::VetoError),

    #[error(transparent)]
    VotingError(#[from] dao_voting::error::VotingError),

    #[error("unauthorized")]
    Unauthorized {},

    #[error("no such calendar ({token_id})")]
    NoSuchCalendar { token_id: u64 },

    #[error("no such event (calendar {calendar_id}, event {event_id})")]
    NoSuchEvent { calendar_id: u64, event_id: u64 },

    #[error("calendar ({0}) is not active")]
    CalendarNotActive(u64),

    #[error("event ({event_id}) is not upcoming")]
    EventNotUpcoming { event_id: u64 },

    #[error("invalid time range: start must be before end")]
    InvalidTimeRange {},

    #[error("invalid timezone: {tz}")]
    InvalidTimezone { tz: String },

    #[error("invalid reply ID: {id}")]
    InvalidReplyID { id: u64 },

    #[error("event start must be after end")]
    StartAfterEnd {},
}

pub mod msg {
    use super::*;
    use cosmwasm_schema::{cw_serde, QueryResponses};
    use cw721::traits::Cw721CustomMsg;

    /// Input for creating a NIP-52 calendar event within a calendar NFT.
    #[cw_serde]
    pub struct CreateEventInput {
        pub title: String,
        pub description: Option<String>,
        /// 31922 (date-based) or 31923 (time-based).
        pub kind: u16,
        pub start_time: u64,
        pub end_time: u64,
        pub timezone: Option<String>,
        pub locations: Vec<String>,
        pub geohash: Option<String>,
        pub hashtags: Vec<String>,
        pub references: Vec<String>,
        pub summary: Option<String>,
    }

    #[cw_serde]
    #[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
    pub enum ExecuteExt {
        /// Create a new calendar NFT. The pre-propose module validates
        /// who may call this.
        CreateCalendar {
            title: String,
            description: Option<String>,
            /// Optional initial owner. Defaults to sender if unset.
            owner: Option<String>,
            // extension: NostrExtension, // TODO: replace with collection extension
        },
        /// Transfer a calendar NFT to a new owner.
        TransferCalendar { token_id: u64, recipient: String },
        /// Create a NIP-52 event within a calendar NFT. The calendar
        /// owner must be the sender.
        CreateEvent {
            calendar_token_id: u64,
            event: CreateEventInput,
        },
        /// Update a NIP-52 event's mutable fields.
        UpdateEvent {
            calendar_token_id: u64,
            event_id: u64,
            title: Option<String>,
            description: Option<String>,
            start_time: Option<u64>,
            end_time: Option<u64>,
            timezone: Option<String>,
            locations: Option<Vec<String>>,
            hashtags: Option<Vec<String>>,
            summary: Option<String>,
        },
        /// Cancel a NIP-52 event.
        CancelEvent {
            calendar_token_id: u64,
            event_id: u64,
        },
        /// Activate / deactivate a calendar. DAO-only.
        SetCalendarActive { token_id: u64, active: bool },
        /// Update proposal creation policy. DAO-only.
        UpdatePreProposeInfo { info: PreProposeInfo },
        /// Update delegation module address. DAO-only.
        UpdateDelegationModule { module: String },
        /// Register a calendar state change hook. DAO-only.
        AddCalendarHook { address: String },
        /// Remove a calendar state change hook. DAO-only.
        RemoveCalendarHook { address: String },
    }
    impl CustomMsg for ExecuteExt {}
    impl Cw721CustomMsg for ExecuteExt {}

    #[cw_serde]
    #[derive(QueryResponses)]
    #[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
    pub enum QueryExt {
        #[returns(Config)]
        Config {},
        /// Calendar info by NFT token ID.
        #[returns(crate::contract::calendar::Calendar)]
        Calendar { token_id: u64 },
        /// Paginated list of all calendar NFTs.
        #[returns(Vec<crate::contract::calendar::Calendar>)]
        ListCalendars {
            start_after: Option<u64>,
            limit: Option<u32>,
        },
        /// A specific event within a calendar.
        #[returns(crate::contract::calendar::CalendarEvent)]
        CalendarEvent {
            calendar_token_id: u64,
            event_id: u64,
        },
        /// Events within a calendar, paginated.
        #[returns(Vec<crate::contract::calendar::CalendarEvent>)]
        CalendarEvents {
            calendar_token_id: u64,
            start_after: Option<u64>,
            limit: Option<u32>,
        },
        /// Total number of calendar NFTs minted.
        #[returns(::std::primitive::u64)]
        CalendarCount {},
        /// Total events across all calendars.
        #[returns(::std::primitive::u64)]
        EventCount {},
        #[returns(::dao_voting::pre_propose::ProposalCreationPolicy)]
        ProposalCreationPolicy {},
        #[returns(Option<::cosmwasm_std::Addr>)]
        DelegationModule {},
        #[returns(::cw_hooks::HooksResponse)]
        CalendarHooks {},
        #[returns(::cw_hooks::HooksResponse)]
        ProposalHooks {},
        #[returns(::cw_hooks::HooksResponse)]
        VoteHooks {},
        #[returns(::dao_interface::voting::InfoResponse)]
        Info {},
        /// Address of the DAO this module belongs to.
        #[returns(Addr)]
        Dao {},
    }
}

// ── cw-orch ExecuteFns / QueryFns bridge ────────────────────────────
//
// The derive macros on msg::ExecuteExt and msg::QueryExt (behind
// `feature = "interface"`) generate methods that need
// Into<ExecuteMsg/QueryMsg>.  These impls wrap custom variants into
// the outer cw721 envelope.
#[cfg(feature = "interface")]
impl From<msg::ExecuteExt> for ExecuteMsg {
    fn from(ext: msg::ExecuteExt) -> Self {
        ExecuteMsg::UpdateExtension { msg: ext }
    }
}
#[cfg(feature = "interface")]
impl From<msg::QueryExt> for QueryMsg {
    fn from(ext: msg::QueryExt) -> Self {
        QueryMsg::Extension { msg: ext }
    }
}

pub mod calendar {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Timestamp};
    use sha2::Digest;

    /// A calendar represented as a single NFT within the DAO's collection.
    ///
    /// Each calendar has a unique `token_id`. The `owner` controls
    /// event creation and management. The `extension` field holds
    /// Nostr-compatible metadata (on-chain event or off-chain CID pointer).
    #[cw_serde]
    pub struct Calendar {
        pub token_id: u64,
        pub title: String,
        pub description: Option<String>,
        pub owner: Addr,
        pub created_at: Timestamp,
        pub created_by: Addr,
        pub active: bool,
    }

    impl Calendar {
        pub fn new(
            token_id: u64,
            title: String,
            description: Option<String>,
            owner: Addr,
            created_at: Timestamp,
            created_by: Addr,
        ) -> Self {
            Self {
                token_id,
                title,
                description,
                owner,
                created_at,
                created_by,
                active: true,
            }
        }
    }

    /// NIP-52 calendar event status.
    #[cw_serde]
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

    /// A NIP-52 calendar event within a calendar NFT.
    #[cw_serde]
    pub struct CalendarEvent {
        pub event_id: u64,
        pub calendar_token_id: u64,
        pub title: String,
        pub description: String,
        pub kind: u16,
        pub start_time: u64,
        pub end_time: u64,
        pub timezone: Option<String>,
        pub locations: Vec<String>,
        pub geohash: Option<String>,
        pub hashtags: Vec<String>,
        pub references: Vec<String>,
        pub status: EventStatus,
        pub summary: Option<String>,
        pub d_tag: String,
        pub created_by: Addr,
        pub created_at: Timestamp,
    }

    impl CalendarEvent {
        pub fn compute_d_tag(title: &str, calendar_token_id: u64, event_id: u64) -> String {
            let hash =
                sha2::Sha256::digest(format!("{title}:{calendar_token_id}:{event_id}").as_bytes());
            hex::encode(hash)[..16].to_string()
        }
    }
}

// ── Entry Points ──
use crate::contract::{
    calendar::{Calendar, CalendarEvent},
    msg::*,
};
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721InstantiateMsg<CalendarModuleCollectionExtension>,
) -> Result<Response, ContractError> {
    DaoNostrCalendar::default().instantiate(deps.branch(), &env, &info, msg.clone())?;
    let dao = &info.sender;
    let msg = msg.collection_info_extension;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let (min_voting_period, max_voting_period) =
        validate_voting_period(msg.min_event_period, msg.max_event_period)?;

    CONFIG.save(
        deps.storage,
        &Config {
            min_event_period: min_voting_period,
            max_event_period: max_voting_period,
            veto: msg.veto,
        },
    )?;
    DAO.save(deps.storage, dao)?;
    CALENDAR_COUNT.save(deps.storage, &0)?;

    let (initial_policy, pre_propose_messages) = msg
        .pre_propose_info
        .into_initial_policy_and_messages(dao.clone())?;

    CREATION_POLICY.save(deps.storage, &initial_policy)?;

    if let Some(module) = msg.delegation_module {
        let addr = deps.api.addr_validate(&module)?;
        DELEGATION_MODULE.save(deps.storage, &addr)?;
    }

    Ok(Response::new()
        .add_submessages(pre_propose_messages)
        .add_attribute("method", "instantiate")
        .add_attribute("dao", dao))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // _ => DaoNostrCalendar::default().execute(deps, &env, &info, msg).map_err(Into::into)
        cw721::msg::Cw721ExecuteMsg::UpdateExtension { msg } => match msg {
            ExecuteExt::CreateCalendar {
                title,
                description,
                owner,
            } => execute::create_calendar(deps, env, info, title, description, owner),
            ExecuteExt::TransferCalendar {
                token_id,
                recipient,
            } => execute::transfer_calendar(deps, info, token_id, recipient),
            ExecuteExt::CreateEvent {
                calendar_token_id,
                event,
            } => execute::create_event(deps, env, info, calendar_token_id, event),
            ExecuteExt::UpdateEvent {
                calendar_token_id,
                event_id,
                title,
                description,
                start_time,
                end_time,
                timezone,
                locations,
                hashtags,
                summary,
            } => execute::update_event(
                deps,
                info,
                calendar_token_id,
                event_id,
                title,
                description,
                start_time,
                end_time,
                timezone,
                locations,
                hashtags,
                summary,
            ),
            ExecuteExt::CancelEvent {
                calendar_token_id,
                event_id,
            } => execute::cancel_event(deps, info, calendar_token_id, event_id),
            ExecuteExt::SetCalendarActive { token_id, active } => {
                execute::set_calendar_active(deps, info, token_id, active)
            }
            ExecuteExt::UpdatePreProposeInfo { info: i } => {
                execute::update_pre_propose_info(deps, info)
            }
            ExecuteExt::UpdateDelegationModule { module } => {
                execute::update_delegation_module(deps, info, module)
            }
            ExecuteExt::AddCalendarHook { address } => {
                execute::add_calendar_hook(deps, info, address)
            }
            ExecuteExt::RemoveCalendarHook { address } => {
                execute::remove_calendar_hook(deps, info, address)
            }
        },
        _ => unimplemented!(),
        // cw721::msg::Cw721ExecuteMsg::UpdateMinterOwnership(action) => todo!(),
        // cw721::msg::Cw721ExecuteMsg::UpdateCreatorOwnership(action) => todo!(),
        // cw721::msg::Cw721ExecuteMsg::UpdateCollectionInfo { collection_info } => todo!(),
        // cw721::msg::Cw721ExecuteMsg::UpdateExtension { msg } => todo!(),
        // cw721::msg::Cw721ExecuteMsg::UpdateNftInfo { token_id, token_uri, extension } => todo!(),
        // cw721::msg::Cw721ExecuteMsg::SetWithdrawAddress { address } => todo!(),
        // cw721::msg::Cw721ExecuteMsg::RemoveWithdrawAddress {} => todo!(),
        // cw721::msg::Cw721ExecuteMsg::WithdrawFunds { amount } => todo!(),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // msg::QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        // msg::QueryMsg::Dao {} => to_json_binary(&DAO.load(deps.storage)?),
        // msg::QueryMsg::Calendar { token_id } => to_json_binary(&query_calendar(deps, token_id)?),
        // msg::QueryMsg::ListCalendars { start_after, limit } => {
        //     to_json_binary(&query_list_calendars(deps, start_after, limit)?)
        // }
        // msg::QueryMsg::CalendarEvent {
        //     calendar_token_id,
        //     event_id,
        // } => to_json_binary(&query_event(deps, calendar_token_id, event_id)?),
        // msg::QueryMsg::CalendarEvents {
        //     calendar_token_id,
        //     start_after,
        //     limit,
        // } => to_json_binary(&query_calendar_events(
        //     deps,
        //     calendar_token_id,
        //     start_after,
        //     limit,
        // )?),
        // msg::QueryMsg::CalendarCount {} => {
        //     to_json_binary(&CALENDAR_COUNT.load(deps.storage)?)
        // }
        // msg::QueryMsg::EventCount {} => to_json_binary(&query_event_count(deps)?),
        // msg::QueryMsg::ProposalCreationPolicy {} => {
        //     to_json_binary(&CREATION_POLICY.load(deps.storage)?)
        // }
        // msg::QueryMsg::DelegationModule {} => {
        //     to_json_binary(&DELEGATION_MODULE.may_load(deps.storage)?)
        // }
        // msg::QueryMsg::CalendarHooks {} => {
        //     to_json_binary(&CALENDAR_HOOKS.query_hooks(deps)?)
        // }
        // msg::QueryMsg::ProposalHooks {} => {
        //     to_json_binary(&CALENDAR_HOOKS.query_hooks(deps)?)
        // }
        // msg::QueryMsg::VoteHooks {} => to_json_binary(&CALENDAR_HOOKS.query_hooks(deps)?),
        // msg::QueryMsg::Info {} => {
        //     let info = cw2::get_contract_version(deps.storage)?;
        //     to_json_binary(&InfoResponse { info })
        // }
        // ownership and operator avoidance
        QueryMsg::Extension { msg } => match msg {
            QueryExt::Config {} => todo!(),
            QueryExt::Calendar { token_id } => todo!(),
            QueryExt::ListCalendars { start_after, limit } => todo!(),
            QueryExt::CalendarEvent {
                calendar_token_id,
                event_id,
            } => todo!(),
            QueryExt::CalendarEvents {
                calendar_token_id,
                start_after,
                limit,
            } => todo!(),
            QueryExt::CalendarCount {} => todo!(),
            QueryExt::EventCount {} => todo!(),
            QueryExt::ProposalCreationPolicy {} => todo!(),
            QueryExt::DelegationModule {} => todo!(),
            QueryExt::CalendarHooks {} => todo!(),
            QueryExt::ProposalHooks {} => todo!(),
            QueryExt::VoteHooks {} => todo!(),
            QueryExt::Info {} => todo!(),
            QueryExt::Dao {} => todo!(),
        },
        // cw721::msg::Cw721QueryMsg::NumTokens {} => todo!(),
        // cw721::msg::Cw721QueryMsg::GetConfig {} => todo!(),
        // cw721::msg::Cw721QueryMsg::GetCollectionInfoAndExtension {} => todo!(),
        // cw721::msg::Cw721QueryMsg::GetAllInfo {} => todo!(),
        // cw721::msg::Cw721QueryMsg::GetCollectionExtensionAttributes {} => todo!(),

        // cw721::msg::Cw721QueryMsg::GetMinterOwnership {} => todo!(),
        // cw721::msg::Cw721QueryMsg::GetCreatorOwnership {} => todo!(),
        // cw721::msg::Cw721QueryMsg::GetAdditionalMinters { .. } => todo!(),
        // cw721::msg::Cw721QueryMsg::NftInfo { .. } => todo!(),
        // cw721::msg::Cw721QueryMsg::GetNftByExtension { .. } => todo!(),
        // cw721::msg::Cw721QueryMsg::AllNftInfo { .. } => todo!(),
        // cw721::msg::Cw721QueryMsg::Tokens { .. } => todo!(),
        // cw721::msg::Cw721QueryMsg::AllTokens { .. } => todo!(),
        // cw721::msg::Cw721QueryMsg::GetCollectionExtension { .. } => todo!(),
        // cw721::msg::Cw721QueryMsg::GetWithdrawAddress {} => todo!(),
        _ => unimplemented!(),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    msg: MigrateMsg,
    _info: MigrateInfo,
) -> Result<Response, ContractError> {
    match msg {
        MigrateMsg::FromCompatible {} => {
            set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
            Ok(Response::new().add_attribute("method", "migrate"))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let id = msg.id;
    if id >= FAILED_HOOK_REPLY_ID_BASE {
        let idx = id - FAILED_HOOK_REPLY_ID_BASE;
        let addr = CALENDAR_HOOKS.remove_hook_by_index(deps.storage, idx)?;
        Ok(Response::new().add_attribute("removed_hook", format!("{addr}:{idx}")))
    } else {
        Err(ContractError::InvalidReplyID { id })
    }
}

// ── Execute Handlers ──

pub mod execute {
    use super::*;
    use crate::contract::{
        calendar::{Calendar, CalendarEvent, EventStatus},
        msg::*,
    };
    use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};
    pub fn create_calendar(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        title: String,
        description: Option<String>,
        owner: Option<String>,
    ) -> Result<Response, ContractError> {
        let _dao = DAO.load(deps.storage)?;
        let owner_addr = owner
            .map(|o| deps.api.addr_validate(&o))
            .transpose()?
            .unwrap_or_else(|| info.sender.clone());

        let token_id = CALENDAR_COUNT.load(deps.storage)? + 1;
        let calendar = Calendar::new(
            token_id,
            title,
            description,
            owner_addr,
            env.block.time,
            info.sender,
        );

        CALENDARS.save(deps.storage, token_id, &calendar)?;
        CALENDAR_COUNT.save(deps.storage, &token_id)?;
        EVENT_COUNT.save(deps.storage, token_id, &0)?;

        let hook_msgs =
            fire_calendar_hooks(deps.storage, format!("calendar_created:{}", token_id))?;

        Ok(Response::new()
            .add_submessages(hook_msgs)
            .add_attribute("action", "create_calendar")
            .add_attribute("token_id", token_id.to_string())
            .add_attribute("owner", calendar.owner))
    }

    pub fn transfer_calendar(
        deps: DepsMut,
        info: MessageInfo,
        token_id: u64,
        recipient: String,
    ) -> Result<Response, ContractError> {
        let mut calendar = CALENDARS
            .may_load(deps.storage, token_id)?
            .ok_or(ContractError::NoSuchCalendar { token_id })?;

        if calendar.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        let recipient_addr = deps.api.addr_validate(&recipient)?;
        calendar.owner = recipient_addr;
        CALENDARS.save(deps.storage, token_id, &calendar)?;

        Ok(Response::new()
            .add_attribute("action", "transfer_calendar")
            .add_attribute("token_id", token_id.to_string())
            .add_attribute("new_owner", &calendar.owner))
    }

    pub fn create_event(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        calendar_token_id: u64,
        input: CreateEventInput,
    ) -> Result<Response, ContractError> {
        let calendar = CALENDARS.may_load(deps.storage, calendar_token_id)?.ok_or(
            ContractError::NoSuchCalendar {
                token_id: calendar_token_id,
            },
        )?;

        if !calendar.active {
            return Err(ContractError::CalendarNotActive(calendar_token_id));
        }
        if calendar.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        if input.start_time >= input.end_time {
            return Err(ContractError::StartAfterEnd {});
        }

        let event_id = EVENT_COUNT.load(deps.storage, calendar_token_id)? + 1;
        let event = CalendarEvent {
            event_id,
            calendar_token_id,
            title: input.title,
            description: input.description.unwrap_or_default(),
            kind: input.kind,
            start_time: input.start_time,
            end_time: input.end_time,
            timezone: input.timezone,
            locations: input.locations,
            geohash: input.geohash,
            hashtags: input.hashtags,
            references: input.references,
            status: EventStatus::Upcoming,
            summary: input.summary,
            d_tag: CalendarEvent::compute_d_tag("event", calendar_token_id, event_id),
            created_by: info.sender.clone(),
            created_at: env.block.time,
        };

        EVENTS.save(deps.storage, (calendar_token_id, event_id), &event)?;
        EVENT_COUNT.save(deps.storage, calendar_token_id, &event_id)?;

        let hook_msgs = fire_calendar_hooks(
            deps.storage,
            format!("event_created:{}:{}", calendar_token_id, event_id),
        )?;

        Ok(Response::new()
            .add_submessages(hook_msgs)
            .add_attribute("action", "create_event")
            .add_attribute("calendar_token_id", calendar_token_id.to_string())
            .add_attribute("event_id", event_id.to_string()))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_event(
        deps: DepsMut,
        info: MessageInfo,
        calendar_token_id: u64,
        event_id: u64,
        title: Option<String>,
        description: Option<String>,
        start_time: Option<u64>,
        end_time: Option<u64>,
        timezone: Option<String>,
        locations: Option<Vec<String>>,
        hashtags: Option<Vec<String>>,
        summary: Option<String>,
    ) -> Result<Response, ContractError> {
        let calendar = CALENDARS.may_load(deps.storage, calendar_token_id)?.ok_or(
            ContractError::NoSuchCalendar {
                token_id: calendar_token_id,
            },
        )?;

        if calendar.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        let mut event = EVENTS
            .may_load(deps.storage, (calendar_token_id, event_id))?
            .ok_or(ContractError::NoSuchEvent {
                calendar_id: calendar_token_id,
                event_id,
            })?;

        if let Some(t) = title {
            event.title = t;
        }
        if let Some(d) = description {
            event.description = d;
        }
        if let Some(st) = start_time {
            event.start_time = st;
        }
        if let Some(et) = end_time {
            event.end_time = et;
        }
        if let Some(tz) = timezone {
            event.timezone = Some(tz);
        }
        if let Some(l) = locations {
            event.locations = l;
        }
        if let Some(h) = hashtags {
            event.hashtags = h;
        }
        if let Some(s) = summary {
            event.summary = Some(s);
        }

        if event.start_time >= event.end_time {
            return Err(ContractError::InvalidTimeRange {});
        }

        EVENTS.save(deps.storage, (calendar_token_id, event_id), &event)?;

        Ok(Response::new()
            .add_attribute("action", "update_event")
            .add_attribute("calendar_token_id", calendar_token_id.to_string())
            .add_attribute("event_id", event_id.to_string()))
    }

    pub fn cancel_event(
        deps: DepsMut,
        info: MessageInfo,
        calendar_token_id: u64,
        event_id: u64,
    ) -> Result<Response, ContractError> {
        let calendar = CALENDARS.may_load(deps.storage, calendar_token_id)?.ok_or(
            ContractError::NoSuchCalendar {
                token_id: calendar_token_id,
            },
        )?;

        if calendar.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        let mut event = EVENTS
            .may_load(deps.storage, (calendar_token_id, event_id))?
            .ok_or(ContractError::NoSuchEvent {
                calendar_id: calendar_token_id,
                event_id,
            })?;

        if event.status == EventStatus::Completed {
            return Err(ContractError::EventNotUpcoming { event_id });
        }

        event.status = EventStatus::Cancelled;
        EVENTS.save(deps.storage, (calendar_token_id, event_id), &event)?;

        let hook_msgs = fire_calendar_hooks(
            deps.storage,
            format!("event_cancelled:{}:{}", calendar_token_id, event_id),
        )?;

        Ok(Response::new()
            .add_submessages(hook_msgs)
            .add_attribute("action", "cancel_event")
            .add_attribute("calendar_token_id", calendar_token_id.to_string())
            .add_attribute("event_id", event_id.to_string()))
    }

    pub fn set_calendar_active(
        deps: DepsMut,
        info: MessageInfo,
        token_id: u64,
        active: bool,
    ) -> Result<Response, ContractError> {
        assert_dao(deps.storage, &info.sender)?;

        let mut calendar = CALENDARS
            .may_load(deps.storage, token_id)?
            .ok_or(ContractError::NoSuchCalendar { token_id })?;

        calendar.active = active;
        CALENDARS.save(deps.storage, token_id, &calendar)?;

        Ok(Response::new()
            .add_attribute(
                "action",
                if active {
                    "activate_calendar"
                } else {
                    "deactivate_calendar"
                },
            )
            .add_attribute("token_id", token_id.to_string()))
    }

    pub fn update_pre_propose_info(
        deps: DepsMut,
        info: MessageInfo,
    ) -> Result<Response, ContractError> {
        assert_dao(deps.storage, &info.sender)?;
        CREATION_POLICY.save(
            deps.storage,
            &dao_voting::pre_propose::ProposalCreationPolicy::Anyone {},
        )?;
        Ok(Response::new().add_attribute("action", "update_pre_propose_info"))
    }

    pub fn update_delegation_module(
        deps: DepsMut,
        info: MessageInfo,
        module: String,
    ) -> Result<Response, ContractError> {
        assert_dao(deps.storage, &info.sender)?;
        let addr = deps.api.addr_validate(&module)?;
        DELEGATION_MODULE.save(deps.storage, &addr)?;
        Ok(Response::new().add_attribute("action", "update_delegation_module"))
    }

    pub fn add_calendar_hook(
        deps: DepsMut,
        info: MessageInfo,
        address: String,
    ) -> Result<Response, ContractError> {
        assert_dao(deps.storage, &info.sender)?;
        let addr = deps.api.addr_validate(&address)?;
        CALENDAR_HOOKS.add_hook(deps.storage, addr)?;
        Ok(Response::new()
            .add_attribute("action", "add_calendar_hook")
            .add_attribute("address", address))
    }

    pub fn remove_calendar_hook(
        deps: DepsMut,
        info: MessageInfo,
        address: String,
    ) -> Result<Response, ContractError> {
        assert_dao(deps.storage, &info.sender)?;
        let addr = deps.api.addr_validate(&address)?;
        CALENDAR_HOOKS.remove_hook(deps.storage, addr)?;
        Ok(Response::new()
            .add_attribute("action", "remove_calendar_hook")
            .add_attribute("address", address))
    }

    fn assert_dao(storage: &dyn cosmwasm_std::Storage, sender: &Addr) -> Result<(), ContractError> {
        let dao = DAO.load(storage)?;
        if *sender != dao {
            return Err(ContractError::Unauthorized {});
        }
        Ok(())
    }

    fn fire_calendar_hooks(
        storage: &dyn cosmwasm_std::Storage,
        event: String,
    ) -> Result<Vec<cosmwasm_std::SubMsg>, ContractError> {
        let msgs = CALENDAR_HOOKS.prepare_hooks(storage, |addr| {
            let msg = cosmwasm_std::to_json_binary(&serde_json::json!({
                "calendar_hook": { "event": event.clone() }
            }))?;
            Ok(cosmwasm_std::SubMsg::reply_on_error(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: addr.to_string(),
                    msg,
                    funds: vec![],
                },
                crate::contract::FAILED_HOOK_REPLY_ID_BASE,
            ))
        })?;
        Ok(msgs)
    }
}

// ── Query Handlers ──

pub mod query {
    use super::*;
    use cosmwasm_std::{Deps, Order, StdResult};
    use cw_storage_plus::Bound;

    use crate::contract::calendar::{Calendar, CalendarEvent};

    const DEFAULT_LIMIT: u32 = 30;
    const MAX_LIMIT: u32 = 100;

    fn clamp_limit(limit: Option<u32>) -> usize {
        (limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT)) as usize
    }

    pub fn query_calendar(deps: Deps, token_id: u64) -> StdResult<Calendar> {
        CALENDARS.load(deps.storage, token_id)
    }

    pub fn query_list_calendars(
        deps: Deps,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> StdResult<Vec<Calendar>> {
        let limit = clamp_limit(limit);
        let start = start_after.map(Bound::exclusive);
        CALENDARS
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|r| r.map(|(_, cal)| cal))
            .collect()
    }

    pub fn query_event(
        deps: Deps,
        calendar_token_id: u64,
        event_id: u64,
    ) -> StdResult<CalendarEvent> {
        EVENTS.load(deps.storage, (calendar_token_id, event_id))
    }

    pub fn query_calendar_events(
        deps: Deps,
        calendar_token_id: u64,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> StdResult<Vec<CalendarEvent>> {
        let limit = clamp_limit(limit);
        let start = start_after.map(Bound::exclusive);
        EVENTS
            .prefix(calendar_token_id)
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|r| r.map(|(_, ev)| ev))
            .collect()
    }

    pub fn query_event_count(deps: Deps) -> StdResult<u64> {
        let count = EVENTS
            .keys(deps.storage, None, None, Order::Ascending)
            .count() as u64;
        Ok(count)
    }
}
