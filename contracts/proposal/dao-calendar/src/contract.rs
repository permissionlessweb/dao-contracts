use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo, MigrateInfo,
    Reply, Response, StdResult,
};
use cw2::set_contract_version;
use cw721::{
    extension::Cw721Extensions,
    msg::Cw721InstantiateMsg,
    traits::{Cw721Execute, Cw721Query},
};

use cw721_nips::{nips::nip52::Nip52Kind, NipKind};
use cw_hooks::Hooks;
use cw_storage_plus::{Item, Map};
use cw_utils::{Duration, DAY};
use dao_voting::{
    pre_propose::{PreProposeInfo, ProposalCreationPolicy},
    veto::VetoConfig,
    voting::validate_voting_period,
};

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
    /// Nostr event kind (31922 date-based, 31923 time-based).
    /// Always populated for efficient on-chain filtering.
    pub kind: u16,
    /// Cached d-tag for addressable events (kind 30000–39999).
    /// Avoids decoding `e` just to read the identifier.
    pub d_tag: Option<String>,
    /// Optional: Store the raw nostr event ID for verification.
    pub nostr_e_d: Option<String>,
    /// Optional: Store the author's pubkey.
    pub author_pubkey: Option<String>,
    /// Calendar this event belongs to. Token ID of the parent calendar NFT,
    /// or `None` for standalone events (no parent calendar).
    pub calendar_d: Option<String>,
}

impl Default for MetadataExt {
    fn default() -> Self {
        Self {
            on_chain: Default::default(),
            e: Default::default(),
            cid: Default::default(),
            kind: Nip52Kind::Calendar.kind_value(),
            d_tag: Default::default(),
            nostr_e_d: Default::default(),
            author_pubkey: Default::default(),
            calendar_d: Default::default(),
        }
    }
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

pub type InstantiateMsg = cw721::msg::Cw721InstantiateMsg<CalendarModuleCollectionExtension>;
pub type ExecuteMsg =
    cw721::msg::Cw721ExecuteMsg<MetadataExt, CalendarModuleCollectionExtension, ExecuteExt>;
pub type QueryMsg =
    cw721::msg::Cw721QueryMsg<MetadataExt, CalendarModuleCollectionExtension, QueryExt>;

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

#[cw_serde]
pub struct CalendarModuleCollectionExtension {
    pub min_event_period: Option<Duration>,
    pub max_event_period: Duration,
    pub pre_propose_info: PreProposeInfo,
    pub veto: Option<VetoConfig>,
    pub delegation_module: Option<String>,
}

pub const DAO: Item<Addr> = Item::new("dao");
pub const CREATION_POLICY: Item<ProposalCreationPolicy> = Item::new("creation_policy");
pub const DELEGATION_MODULE: Item<Addr> = Item::new("delegation_module");
pub const CALENDAR_HOOKS: Hooks = Hooks::new("calendar_hooks");

pub const DAO_CALENDAR: &str = "crates.io:dao-calendar";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const FAILED_HOOK_REPLY_ID_BASE: u64 = 1_000_000;

pub const CALENDAR_COUNT: Item<u64> = Item::new("calendar_count");
pub const EVENT_COUNT: Map<&str, u64> = Map::new("event_count");

// Token ID prefix design
//
//   Calendars:  cal/{counter}        e.g. cal/1, cal/2
//   Events:     evt/{cal_d}/{counter} e.g. evt/cal/1/1, evt/cal/2/1
//   Standalone: evt/_/{counter}       e.g. evt/_/1, evt/_/2
//
// The delimiter `/` ensures no prefix collision between calendars
// (cal/1 vs cal/10) when using Bound-based range queries.
pub const CALENDAR_PREFIX: &str = "cal";
pub const EVENT_PREFIX: &str = "evt";
pub const STANDALONE_TOKEN: &str = "_";

/// Generate a calendar token ID.
///
/// Calendars use token IDs of the form `cal/{counter}`.
///
/// ```
/// # use dao_calendar::contract::cal_d;
/// assert_eq!(cal_d(1), "cal/1");
/// assert_eq!(cal_d(10), "cal/10");
/// assert_eq!(cal_d(100), "cal/100");
/// ```
pub fn cal_d(counter: u64) -> String {
    format!("{CALENDAR_PREFIX}/{counter}")
}

/// Generate an event token ID.
///
/// Events use token IDs of the form `evt/{cal_d}/{counter}`.
/// Standalone events (no parent calendar) use `evt/_/{counter}`.
///
/// ```
/// # use dao_calendar::contract::event_d;
/// assert_eq!(event_d("cal/1", 1), "evt/cal/1/1");
/// assert_eq!(event_d("", 1), "evt/_/1");
/// assert_eq!(event_d("cal/2", 10), "evt/cal/2/10");
/// ```
pub fn event_d(d: &str, event_counter: u64) -> String {
    if d.is_empty() {
        format!("{EVENT_PREFIX}/{STANDALONE_TOKEN}/{event_counter}")
    } else {
        format!("{EVENT_PREFIX}/{d}/{event_counter}")
    }
}

pub mod state {
    use super::*;
    use cosmwasm_std::{from_json, StdError};
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
            // Semantic match: only compare kind and calendar_d.
            // This allows query_nft_by_extension to find events by
            // calendar association without needing exact-equality on
            // all event-specific fields (e_tag, d_tag, on_chain, etc.).
            self.kind == other.kind && self.calendar_d == other.calendar_d
        }
    }

    // ── NIP-52 integration ──────────────────────────────────────────
    use cw721_nips::{
        cw::{NostrCw721Builder, NostrCw721Ext},
        error::{NipError, NipResult},
        nips::nip52::{CalendarEventMetadata, CalendarMetadata, Nip52Kind},
        NipKind, NipMetadata, NostrExt, RawNostrEvent,
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
                    return Err(NipError::Validation("Empty event data".into()));
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
                vec![]
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
            let kind = match event.kind {
                31924 => Nip52Kind::Calendar,
                31922 => Nip52Kind::DateEvent,
                31923 => Nip52Kind::TimeEvent,
                other => return Err(NipError::Validation(format!("unsupported kind {other} for calendar/event"))),
            };
            match kind {
                Nip52Kind::Calendar => {
                    let inner = CalendarMetadata::from_raw_event(event)?;
                    let d_tag = inner.d_tag();
                    let e = Binary::from(serde_json::to_vec(&inner)?);
                    Ok(Self {
                        on_chain: true,
                        e,
                        cid: None,
                        kind: event.kind,
                        d_tag,
                        nostr_e_d: None,
                        author_pubkey: None,
                        calendar_d: None,
                    })
                }
                _ => {
                    // DateEvent (31922) or TimeBased (31923)
                    let inner = CalendarEventMetadata::from_raw_event(event)?;
                    let d_tag = inner.d_tag();
                    let e = Binary::from(serde_json::to_vec(&inner)?);
                    Ok(Self {
                        on_chain: true,
                        e,
                        cid: None,
                        kind: event.kind,
                        d_tag,
                        nostr_e_d: Some(event.id.clone()),
                        author_pubkey: Some(event.pubkey.clone()),
                        calendar_d: None,
                    })
                }
            }
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

    // ── NostrCw721Builder (on-chain / off-chain constructors) ──────

    impl NostrCw721Builder for MetadataExt {
        type Extension = Self;

        fn onchain_metadata(event: &RawNostrEvent) -> StdResult<Self> {
            // Reuse the NipMetadata::from_raw_event logic
            Self::from_raw_event(event).map_err(|e| {
                cosmwasm_std::StdError::msg(format!(
                    "Failed to build on-chain metadata: {e}"
                ))
            })
        }

        fn offchain_metadata(cid: String, kind: u16) -> StdResult<Self> {
            Ok(Self {
                on_chain: false,
                e: Binary::default(),
                cid: Some(cid),
                kind,
                d_tag: None,
                nostr_e_d: None,
                author_pubkey: None,
                calendar_d: None,
            })
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
                        min_event_period = Some(from_json(attr.value.clone())?);
                    }
                    "max_event_period" => {
                        max_event_period = Some(from_json(attr.value.clone())?);
                    }
                    "pre_propose_info" => {
                        pre_propose_info = Some(from_json(attr.value.clone())?);
                    }
                    "veto" => {
                        veto = Some(from_json(attr.value.clone())?);
                    }
                    "delegation_module" => {
                        delegation_module = Some(from_json(attr.value.clone())?);
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
            deps: Deps,
            env: &Env,
            info: Option<&MessageInfo>,
            current: Option<&CalendarModuleCollectionExtension>,
        ) -> Result<CalendarModuleCollectionExtension, Cw721ContractError> {
            self.validate(deps, env, info, current)?;
            Ok(self.clone())
        }

        fn validate(
            &self,
            _deps: Deps,
            _env: &Env,
            _info: Option<&MessageInfo>,
            current: Option<&CalendarModuleCollectionExtension>,
        ) -> Result<(), Cw721ContractError> {
            //  we require top-level dao calendar parameters for anti-lockout
            match current {
                Some(ext) => {
                    // Update path: validate existing extension for anti-lockout
                    validate_voting_period(ext.min_event_period, ext.max_event_period)
                        .map_err(|e| StdError::msg(e.to_string()))?;
                    Ok(())
                }
                None => {
                    // First-time instantiation: validate our own parameters instead.
                    validate_voting_period(self.min_event_period, self.max_event_period)
                        .map_err(|e| StdError::msg(e.to_string()))?;
                    Ok(())
                }
            }
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
            // Mint path: accept the provided nft extension as-is.
            // Update path: accept the updated extension.
            Ok(self.clone())
        }

        fn validate(
            &self,
            _deps: Deps,
            _env: &Env,
            _info: Option<&MessageInfo>,
            _current: Option<&MetadataExt>,
        ) -> Result<(), Cw721ContractError> {
            // MetadataExt validation is handled at the caller
            // (create_calendar, create_event) before minting.
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
    OwnershipError(#[from] cw_ownable::OwnershipError),

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

    #[error("no such calendar ({d})")]
    NoSuchCalendar { d: String },

    #[error("no such event (calendar {d}, event {e_d})")]
    NoSuchEvent { d: String, e_d: String },

    #[error("calendar ({0}) is not active")]
    CalendarNotActive(String),

    #[error("event ({e_d}) is not upcoming")]
    EventNotUpcoming { e_d: String },

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

    #[cw_serde]
    #[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
    pub enum ExecuteExt {
        /// Create a new calendar NFT. The pre-propose module validates
        /// who may call this.
        CreateCalendar {
            /// Optional initial owner. Defaults to sender if unset.
            owner: Option<String>,
            extension: MetadataExt,
        },
        /// Transfer a calendar NFT to a new owner.
        TransferCalendar { d: String, recipient: String },
        /// Create a NIP-52 event within a calendar NFT. The calendar
        /// owner must be the sender.
        CreateEvent { d: String, event: MetadataExt },
        /// Update a NIP-52 event's mutable fields.
        UpdateEvent {
            e_d: String,
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
        CancelEvent { e_d: String },
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
        #[returns(cw721::msg::CollectionInfoAndExtensionResponse<CalendarModuleCollectionExtension> )]
        Config {},
        /// Calendar info by NFT token ID.
        #[returns(MetadataExt)]
        Calendar { d: String },
        /// Paginated list of all calendar NFTs.
        #[returns(Vec<cw721::msg::NftInfoResponse<MetadataExt>>)]
        ListCalendars {
            start_after: Option<String>,
            limit: Option<u32>,
        },
        /// A specific event within a calendar.
        #[returns(cw721::msg::NftInfoResponse<MetadataExt> )]
        CalendarEvent { e_d: String },
        /// Events within a calendar, paginated.
        #[returns(Vec<cw721::msg::NftInfoResponse<MetadataExt>>)]
        CalendarEvents {
            d: String,
            start_after: Option<String>,
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
        Hooks,
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

// ── Entry Points ──
use crate::contract::msg::*;
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721InstantiateMsg<CalendarModuleCollectionExtension>,
) -> Result<Response, ContractError> {
    DaoNostrCalendar::default().instantiate(deps.branch(), &env, &info, msg.clone())?;
    set_contract_version(deps.storage, DAO_CALENDAR, CONTRACT_VERSION)?;
    let dao = &info.sender;
    let msg = msg.collection_info_extension;
    let (initial_policy, pre_propose_messages) = msg
        .pre_propose_info
        .into_initial_policy_and_messages(dao.clone())?;

    // internal collection state
    CALENDAR_COUNT.save(deps.storage, &0u64)?;
    CREATION_POLICY.save(deps.storage, &initial_policy)?;
    DAO.save(deps.storage, dao)?;

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
    let own = DaoNostrCalendar::default()
        .query_minter_ownership(deps.storage)?
        .owner;
    println!("{:#?}", own);
    match own {
        Some(o) => match o == info.sender {
            true => {}
            false => return Err(ContractError::Unauthorized {}),
        },
        None => {}
    }
    match msg {
        cw721::msg::Cw721ExecuteMsg::UpdateExtension { msg } => match msg {
            ExecuteExt::CreateCalendar { owner, extension } => {
                execute::create_calendar(deps, env, info, owner, extension)
            }
            ExecuteExt::TransferCalendar { d, recipient } => {
                execute::transfer_calendar(deps, info, d, recipient)
            }
            ExecuteExt::CreateEvent { d, event } => {
                execute::create_event(deps, env, info, d, event)
            }
            ExecuteExt::UpdateEvent {
                e_d,
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
                e_d,
                title,
                description,
                start_time,
                end_time,
                timezone,
                locations,
                hashtags,
                summary,
            ),
            ExecuteExt::CancelEvent { e_d } => execute::cancel_event(deps, info, e_d),

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
        // Delegate standard cw721 operations (TransferNft, Burn, etc.)
        // to the underlying cw721 base.
        other => {
            let resp = DaoNostrCalendar::default().execute(deps, &env, &info, other)?;
            Ok(resp)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Extension { msg } => match msg {
            QueryExt::CalendarCount {} => to_json_binary(&CALENDAR_COUNT.load(deps.storage)?),
            QueryExt::EventCount {} => query_event_count(deps),
            QueryExt::ProposalCreationPolicy {} => to_json_binary(&query_creation_policy(deps)?),
            QueryExt::DelegationModule {} => to_json_binary(&query_delegation_module(deps)?),
            QueryExt::Info {} => query_info(deps),
            QueryExt::Hooks {} => to_json_binary(&CALENDAR_HOOKS.query_hooks(deps)?),
            QueryExt::Dao {} => to_json_binary(&DAO.load(deps.storage)?),
            QueryExt::Config {} => to_json_binary(
                &DaoNostrCalendar::default().query_collection_info_and_extension(deps)?,
            ),
            QueryExt::Calendar { d } => to_json_binary(
                &DaoNostrCalendar::default()
                    .query_nft_info(deps.storage, d)?
                    .extension,
            ),
            QueryExt::ListCalendars { start_after, limit } => {
                to_json_binary(&DaoNostrCalendar::default().query_nft_by_extension(
                    deps.storage,
                    MetadataExt {
                        kind: Nip52Kind::Calendar.kind_value(),
                        ..Default::default()
                    },
                    start_after,
                    limit,
                )?)
            }
            QueryExt::CalendarEvent { e_d } => {
                to_json_binary(&DaoNostrCalendar::default().query_nft_info(deps.storage, e_d)?)
            }
            QueryExt::CalendarEvents {
                d,
                start_after,
                limit,
            } => to_json_binary(
                &DaoNostrCalendar::default()
                    .query_nft_by_extension(
                        deps.storage,
                        MetadataExt {
                            kind: Nip52Kind::TimeEvent.kind_value(),
                            calendar_d: Some(d),
                            ..Default::default()
                        },
                        start_after,
                        limit,
                    )?
                    .unwrap_or_default(),
            ),
        },
        // Delegate standard cw721 queries (OwnerOf, Tokens, etc.)
        // to the underlying cw721 base collection.
        other => DaoNostrCalendar::default()
            .query(deps, &env, other)
            .map_err(|e| cosmwasm_std::StdError::msg(e.to_string())),
    }
}

// TERRIBLE design. needs improvement
pub fn query_event_count(deps: Deps) -> StdResult<Binary> {
    // Sum all events across all calendars
    let total: u64 = EVENT_COUNT
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, v)| v).unwrap_or(0))
        .sum();
    to_json_binary(&total)
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_json_binary(&dao_interface::voting::InfoResponse { info })
}

pub fn query_creation_policy(deps: Deps) -> StdResult<Binary> {
    let policy = CREATION_POLICY.load(deps.storage)?;
    to_json_binary(&policy)
}

pub fn query_delegation_module(deps: Deps) -> StdResult<Binary> {
    let module = DELEGATION_MODULE.may_load(deps.storage)?;
    to_json_binary(&module)
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
            set_contract_version(deps.storage, DAO_CALENDAR, CONTRACT_VERSION)?;
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
    use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};
    use cw721::traits::Cw721Query;
    pub fn create_calendar(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        owner: Option<String>,
        extension: MetadataExt,
    ) -> Result<Response, ContractError> {
        let dao = DAO.load(deps.storage)?;
        let owner_addr = owner
            .map(|o| deps.api.addr_validate(&o))
            .transpose()?
            .unwrap_or_else(|| dao);
        let cal_count = CALENDAR_COUNT.load(deps.storage)? + 1;
        let d = cal_d(cal_count);
        CALENDAR_COUNT.save(deps.storage, &cal_count)?;
        EVENT_COUNT.save(deps.storage, &d, &0u64)?;
        let hook_msgs = fire_calendar_hooks(deps.storage, format!("calendar_created:{}", d))?;
        DaoNostrCalendar::default().execute(
            deps,
            &env,
            &info,
            ExecuteMsg::Mint {
                token_id: d.clone(),
                owner: owner_addr.to_string(),
                token_uri: None,
                extension,
            },
        )?;

        Ok(Response::new()
            .add_submessages(hook_msgs)
            .add_attribute("action", "create_calendar")
            .add_attribute("d", d.to_string())
            .add_attribute("owner", owner_addr.to_string()))
    }

    pub fn transfer_calendar(
        deps: DepsMut,
        info: MessageInfo,
        d: String,
        recipient: String,
    ) -> Result<Response, ContractError> {
        let contract = DaoNostrCalendar::default();
        let mut cal = contract.config.nft_info.load(deps.storage, &d)?;
        if cal.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        let recipient_addr = deps.api.addr_validate(&recipient)?;

        cal.owner = recipient_addr;

        Ok(Response::new()
            .add_attribute("action", "transfer_calendar")
            .add_attribute("d", d.to_string())
            .add_attribute("new_owner", &cal.owner))
    }

    pub fn create_event(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        d: String,
        extension: MetadataExt,
    ) -> Result<Response, ContractError> {
        // event must be able to be created and counted without a tag to the
        // calendar. events without calendar prefix is empty, and we must
        // include them in the count normally.
        //
        // validate the calendar exists when d is non-empty
        if !d.is_empty() {
            DaoNostrCalendar::default()
                .config
                .nft_info
                .load(deps.storage, &d)?;
        }

        let e_count = EVENT_COUNT.load(deps.storage, &d).unwrap_or(0) + 1;
        let e_d = event_d(&d, e_count);
        EVENT_COUNT.save(deps.storage, &d, &e_count)?;

        let hook_msgs = fire_calendar_hooks(deps.storage, format!("event_created:{d}:{e_d}"))?;

        let mut ext = extension;
        // Tag the event with its parent calendar for efficient query-by-association
        ext.calendar_d = if d.is_empty() { None } else { Some(d.clone()) };

        let resp = DaoNostrCalendar::default().execute(
            deps,
            &env,
            &info,
            ExecuteMsg::Mint {
                token_id: e_d.clone(),
                owner: info.sender.to_string(),
                token_uri: None,
                extension: ext,
            },
        )?;

        Ok(resp
            .add_submessages(hook_msgs)
            .add_attribute("action", "create_event")
            .add_attribute("d", d)
            .add_attribute("e_d", e_d))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_event(
        deps: DepsMut,
        info: MessageInfo,
        e_d: String,
        title: Option<String>,
        description: Option<String>,
        start_time: Option<u64>,
        end_time: Option<u64>,
        timezone: Option<String>,
        locations: Option<Vec<String>>,
        hashtags: Option<Vec<String>>,
        summary: Option<String>,
    ) -> Result<Response, ContractError> {
        let cals = DaoNostrCalendar::default();
        let mut event = cals.config.nft_info.load(deps.storage, &e_d)?;
        match event.extension.on_chain {
            true => {
                // if let Some(t) = title {
                //     title = t;
                // }
                // if let Some(d) = description {
                //     event.description = d;
                // }
                // if let Some(st) = start_time {
                //     event.start_time = st;
                // }
                // if let Some(et) = end_time {
                //     event.end_time = et;
                // }
                // if let Some(tz) = timezone {
                //     event.timezone = Some(tz);
                // }
                // if let Some(l) = locations {
                //     event.locations = l;
                // }
                // if let Some(h) = hashtags {
                //     event.hashtags = h;
                // }
                // if let Some(s) = summary {
                //     event.summary = Some(s);
                // }

                // if event.start_time >= event.end_time {
                //     return Err(ContractError::InvalidTimeRange {});
                // }

                // EVENTS.save(deps.storage, (d, e_d), &event)?;
            }
            false => {

                // validate new ipfs and update on-chain pointers
            }
        }

        Ok(Response::new()
            .add_attribute("action", "update_event")
            .add_attribute("e_d", e_d.to_string()))
    }

    pub fn cancel_event(
        deps: DepsMut,
        info: MessageInfo,
        e_d: String,
    ) -> Result<Response, ContractError> {
        let cals = DaoNostrCalendar::default();
        let mut event = cals.config.nft_info.load(deps.storage, &e_d)?;

        // let mut event = EVENTS
        //     .may_load(deps.storage, (d, e_d))?
        //     .ok_or(ContractError::NoSuchEvent { d: d, e_d })?;

        // if event.status == EventStatus::Completed {
        //     return Err(ContractError::EventNotUpcoming { e_d });
        // }

        // event.status = EventStatus::Cancelled;
        // EVENTS.save(deps.storage, (d, e_d), &event)?;

        let hook_msgs = fire_calendar_hooks(deps.storage, format!("event_cancelled:{}", e_d))?;

        Ok(Response::new()
            .add_submessages(hook_msgs)
            .add_attribute("action", "cancel_event")
            .add_attribute("e_d", e_d.to_string()))
    }

    pub fn update_pre_propose_info(
        deps: DepsMut,
        info: MessageInfo,
    ) -> Result<Response, ContractError> {
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
