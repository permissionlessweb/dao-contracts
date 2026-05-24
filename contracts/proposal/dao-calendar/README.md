# dao-calendar

On-chain calendar for DAOs. Calendars and events are CW721 NFTs
with NIP-52 Nostr calendar metadata. Events are time-bounded
scheduling primitives that integrate with the DAO's event gauge
system.

## Query API

The primary interface is `QueryExt`, dispatched through
`Cw721QueryMsg::Extension { msg }`. Every query returns JSON
via `to_json_binary`.

### Calendar queries

```rust
// Config — collection-level parameters
QueryExt::Config {}
// -> CalendarModuleCollectionExtension

// Single calendar NFT by d-tag (token ID)
QueryExt::Calendar { d: String }
// -> MetadataExt (NIP-52 calendar metadata)

// Paginated list of all calendars
// Filters by kind = Nip52Kind::Calendar (31922)
QueryExt::ListCalendars {
    start_after: Option<String>,
    limit: Option<u32>,
}
// -> Vec<MetadataExt>

// Total calendar count
QueryExt::CalendarCount {}
// -> u64
```

Calendars use d-tags of the form `c{1}`, `c{2}`, … (auto-increment
counter). Each `kind` field is set to `Nip52Kind::Calendar`.

### Event queries

```rust
// Single event NFT by e_d-tag (event token ID)
QueryExt::CalendarEvent { e_d: String }
// -> MetadataExt (NIP-52 calendar event metadata)

// Paginated events within a calendar
// Filters by kind = Nip52Kind::TimeEvent (31923)
QueryExt::CalendarEvents {
    d: String,
    start_after: Option<String>,
    limit: Option<u32>,
}
// -> Vec<String>
```

Events use e_d-tags of the form `e{c1-1}`, `e{c1-2}`, … where
the prefix before the `-` is the parent calendar's d-tag and
the suffix is an auto-increment counter per calendar.

### Module standard queries

```rust
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
  versus `Nip52Kind::TimeEvent`. Both `query_nft_info` and
  `query_nft_by_extension` serve both types.

- `ListCalendars` and `CalendarEvents` are extension-filtered queries.
  They pass a `MetadataExt` template with the target `kind` set
  and use `query_nft_by_extension` to match. This avoids separate
  index maps.

- Event ordering is insertion order (auto-increment counter).
  There is no re-ordering. Frontends sort client-side by
  `start_time` / `end_time` as needed.

- `CalendarEvent` returns MetadataExt, not a rich event struct.
  The Nostr calendar metadata (title, description, start/end times,
  locations, hashtags, timezone) are embedded in the NIP-52
  encoding within `MetadataExt.e` when `on_chain: true`, or
  referenced via IPFS CID when `on_chain: false`.

## Execute API

```rust
ExecuteExt::CreateCalendar {
    owner: Option<String>,
    extension: MetadataExt,
}
ExecuteExt::TransferCalendar { d, recipient }
ExecuteExt::CreateEvent { d, event: MetadataExt }
ExecuteExt::UpdateEvent { e_d, title, description, start_time, end_time, timezone, locations, hashtags, summary }
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

- **On-chain**: `on_chain: true`, full `CalendarEventMetadata` encoded
  in `e` (Binary). Supports `to_tags()`, `content()`, `d_tag()`,
  and round-trip reconstruction via `into_event()`.
- **Off-chain**: `on_chain: false`, only `cid: Option<String>` with
  IPFS pointer. No NIP metadata parsing; use the CID to fetch
  the full Nostr event JSON externally.

## Build

```sh
cargo build -p dao-calendar
cargo test -p dao-calendar --lib
```

Tests under `src/testing/` cover calendar CRUD, event creation,
hook firing, and gauge message scheduling.
