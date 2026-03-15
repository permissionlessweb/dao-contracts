use cosmwasm_std::{Addr, Empty};
use cw_hooks::Hooks;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::msg::{Event, EventGauge, Group};

pub const DAO: Item<Addr> = Item::new("dao");
pub const EVENT_COUNT: Item<u64> = Item::new("event_count");
pub const EVENT_HOOKS: Hooks = Hooks::new("event_hooks");

/// Secondary indexes for events, generic over the metadata extension type.
pub struct EventIndexes<'a, TMetadata = Empty>
where
    TMetadata: Serialize + DeserializeOwned + Clone + PartialEq + JsonSchema + 'static,
{
    pub by_start: MultiIndex<'a, u64, Event<TMetadata>, u64>,
    pub by_end: MultiIndex<'a, u64, Event<TMetadata>, u64>,
    pub by_status: MultiIndex<'a, u8, Event<TMetadata>, u64>,
}

impl<TMetadata> IndexList<Event<TMetadata>> for EventIndexes<'_, TMetadata>
where
    TMetadata: Serialize + DeserializeOwned + Clone + PartialEq + JsonSchema + 'static,
{
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn Index<Event<TMetadata>>> + '_> {
        let v: Vec<&dyn Index<Event<TMetadata>>> =
            vec![&self.by_start, &self.by_end, &self.by_status];
        Box::new(v.into_iter())
    }
}

/// Create an IndexedMap for events, generic over the metadata extension type.
/// The default contract uses `events::<Empty>()` (aliased as `events()`).
/// Custom contracts can call `events::<MyMetadata>()` for typed storage.
pub fn events<'a, TMetadata>() -> IndexedMap<u64, Event<TMetadata>, EventIndexes<'a, TMetadata>>
where
    TMetadata: Serialize + DeserializeOwned + Clone + PartialEq + JsonSchema + 'static,
{
    IndexedMap::new(
        "events",
        EventIndexes {
            by_start: MultiIndex::new(
                |_pk, e: &Event<TMetadata>| e.start_time.nanos() / 1_000_000_000,
                "events",
                "events__by_start",
            ),
            by_end: MultiIndex::new(
                |_pk, e: &Event<TMetadata>| e.end_time.nanos() / 1_000_000_000,
                "events",
                "events__by_end",
            ),
            by_status: MultiIndex::new(
                |_pk, e: &Event<TMetadata>| e.status.as_u8(),
                "events",
                "events__by_status",
            ),
        },
    )
}

/// Join table: (group_id, event_id) → Empty
pub const EVENTS_BY_GROUP: Map<(&str, u64), Empty> = Map::new("events_by_group");

/// Groups: group_id → Group
pub const GROUPS: Map<&str, Group> = Map::new("groups");

/// Per-event gauge overrides
pub const EVENT_GAUGES: Map<u64, EventGauge> = Map::new("event_gauges");
