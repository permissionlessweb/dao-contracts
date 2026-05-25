# dao-calendar

On-chain calendar for DAOs. Calendars and events are CW721 NFTs
with NIP-52 Nostr calendar metadata. Events are time-bounded
scheduling primitives that integrate with the DAO's event gauge
system.

## Token ID design

All calendars and events live in a single CW721 collection. Token IDs
use a prefix-based scheme to enable efficient range queries via
`Bound::Inclusive`/`Exclusive`:

```text
Calendars:   cal/{counter}           e.g. cal/1, cal/2, cal/10
Events:      evt/{cal_d}/{counter}   e.g. evt/cal/1/1, evt/cal/2/3
Standalone:  evt/_/{counter}         e.g. evt/_/1, evt/_/2
```

The `/` delimiter ensures no prefix collision when using
`Bound`-based range queries (cal/1 vs cal/10).

`cal_d()` and `event_d()` in `contract.rs` generate token IDs from
auto-increment counters. Standalone events (no parent calendar) use
`_` as the calendar ID placeholder.

## Query API

The primary interface is `QueryExt`, dispatched through
`Cw721QueryMsg::Extension { msg }`. Every query returns JSON
via `to_json_binary`.

### Calendar queries

```ignore
// Config — collection-level parameters
QueryExt::Config {}
// -> CalendarModuleCollectionExtension

// Single calendar NFT by token ID (e.g. "cal/1")
QueryExt::Calendar { d: String }
// -> MetadataExt (NIP-52 calendar metadata)

// Paginated list of all calendars
QueryExt::ListCalendars {
    start_after: Option<String>,
    limit: Option<u32>,
}
// -> Vec<NftInfoResponse<MetadataExt>>

// Total calendar count
QueryExt::CalendarCount {}
// -> u64
```

Calendars use token IDs of the form `cal/{n}` (auto-increment
counter). The `kind` field in `MetadataExt` is set to
`Nip52Kind::Calendar` (31924).

### Event queries

```ignore
// Single event NFT by token ID (e.g. "evt/cal/1/1" or "evt/_/1")
QueryExt::CalendarEvent { e_d: String }
// -> NftInfoResponse<MetadataExt>

// Paginated events within a calendar
QueryExt::CalendarEvents {
    d: String,
    start_after: Option<String>,
    limit: Option<u32>,
}
// -> Vec<NftInfoResponse<MetadataExt>>

// Total events across all calendars
QueryExt::EventCount {}
// -> u64
```

Events use token IDs of the form `evt/{calendar_d}/{n}`. Standalone
events (no parent calendar) use `evt/_/{n}`. The counter is
per-calendar (or global for standalone).

### Module standard queries

```ignore
QueryExt::ProposalCreationPolicy {}
// -> ProposalCreationPolicy

QueryExt::DelegationModule {}
// -> Option<Addr>

QueryExt::Hooks {}
// -> HooksResponse

QueryExt::Info {}
// -> InfoResponse

QueryExt::Dao {}
// -> Addr
```

### Design notes

- Calendar and event data shares a single CW721 collection.
  The `kind` field in `MetadataExt` disambiguates — `Nip52Kind::Calendar`
  versus `Nip52Kind::TimeEvent`/`Nip52Kind::DateEvent`. Both
  `query_nft_info` and `query_nft_by_extension` serve both types.

- `ListCalendars` and `CalendarEvents` are extension-filtered queries.
  They pass a `MetadataExt` template with the target `kind` set
  and use `query_nft_by_extension` to match. This avoids separate
  index maps.

- Event ordering is insertion order (auto-increment counter).
  There is no re-ordering. Frontends sort client-side by
  `start_time` / `end_time` as needed.

- `CalendarEvent` returns `NftInfoResponse<MetadataExt>`, matching
  the standard cw721 `NftInfo` query. The Nostr calendar metadata
  (title, description, start/end times, locations, hashtags,
  timezone) are embedded in the NIP-52 encoding within
  `MetadataExt.e` when `on_chain: true`, or referenced via IPFS CID
  when `on_chain: false`.

## Execute API

```ignore
ExecuteExt::CreateCalendar {
    owner: Option<String>,
    extension: MetadataExt,
}
ExecuteExt::TransferCalendar { d, recipient }
ExecuteExt::CreateEvent { d, event: MetadataExt }
ExecuteExt::UpdateEvent {
    e_d, title, description,
    start_time, end_time, timezone,
    locations, hashtags, summary,
}
ExecuteExt::CancelEvent { e_d }
ExecuteExt::UpdatePreProposeInfo { info }
ExecuteExt::UpdateDelegationModule { module }
ExecuteExt::AddCalendarHook { address }
ExecuteExt::RemoveCalendarHook { address }
```

All dispatched through `Cw721ExecuteMsg::UpdateExtension { msg }`.
The execute entry point asserts `cw_ownable` ownership against the
DAO address before routing.

## NIP-52 Nostr integration

`MetadataExt` implements `NipMetadata` and `NostrExt` from
`cw721-nips`. Two storage modes:

- **On-chain**: `on_chain: true`, full NIP-52 metadata encoded
  in `e` (Binary). Use `decode::<CalendarEventMetadata>()` or
  `decode::<CalendarMetadata>()` to extract the inner NIP type.
- **Off-chain**: `on_chain: false`, only `cid: Option<String>` with
  IPFS pointer. No NIP metadata parsing; use the CID to fetch
  the full Nostr event JSON externally.

## Build

```sh
cargo build -p dao-calendar
cargo test -p dao-calendar -p dao-testing --lib
```

Tests live in `packages/dao-testing/src/suite/tests/voting/calendar.rs`.
Run with `cargo test -p dao-testing -- voting::calendar::tests`.