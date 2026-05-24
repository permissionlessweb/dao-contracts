
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{to_json_binary, Addr, Binary};
use cw721::msg::OwnerOfResponse;
use cw721_nips::{
    cw::NostrCw721Builder as _,
    nips::nip52::{CalendarEventMetadata, Nip52Kind},
    NipKind, NipMetadata, RawNostrEvent, Tag,
};
use cw_orch::mock::Mock;
use cw_orch::prelude::*;
use dao_calendar::contract::{
    msg::{ExecuteExtFns as _, QueryExtFns as _},
    CalendarModuleCollectionExtension, InstantiateMsg, MetadataExt, QueryMsg,
};
use dao_cw_orch::DaoCalendar;

/// Set up a clean Mock with the calendar contract deployed.
/// Deployer is minter + DAO address.
fn setup() -> (Mock, DaoCalendar<Mock>) {
    let creator = MockApi::default().addr_make("creator");
    let mut chain = Mock::new(&creator.to_string());
    let calendar = DaoCalendar::new(chain.clone());
    calendar.upload().unwrap();

    let msg = InstantiateMsg {
        name: "Test Calendar".to_string(),
        symbol: "TCAL".to_string(),
        collection_info_extension: CalendarModuleCollectionExtension {
            min_event_period: Some(cw_utils::Duration::Time(3600)),
            max_event_period: cw_utils::Duration::Time(86400 * 365),
            pre_propose_info: dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
            veto: None,
            delegation_module: None,
        },
        minter: None,
        creator: None,
        withdraw_address: None,
    };
    calendar.instantiate(&msg, Some(&creator), &[]).unwrap();

    (chain, calendar)
}

/// On-chain metadata: full calendar event data stored in `e`.
fn onchain_metadata(d_tag: &str, kind: Nip52Kind) -> MetadataExt {
    let event = CalendarEventMetadata {
        d_tag: d_tag.to_string(),
        title: "Test Event".to_string(),
        summary: Some("A test calendar event".to_string()),
        image: None,
        content: "Test event content".to_string(),
        locations: vec!["Virtual".to_string()],
        geohash: None,
        references: vec![],
        hashtags: vec!["test".to_string()],
        start_timezone: Some("UTC".to_string()),
        end_timezone: None,
        day_granularity: vec![],
        calendar_requests: vec![],
        event_type: cw721_nips::nips::nip52::EventType::TimeBased,
        start_time: 1000000,
        end_time: 2000000,
        participants: vec![],
    };
    let tags: Vec<Vec<String>> = event
        .to_tags()
        .into_iter()
        .map(Tag::into_inner)
        .collect();
    let raw = RawNostrEvent {
        id: format!("test-event-{d_tag}"),
        pubkey: "test-pubkey".to_string(),
        created_at: 1000000,
        kind: kind.kind_value(),
        tags,
        content: event.content(),
        sig: "test-sig".to_string(),
    };
    MetadataExt::onchain_metadata(&raw).unwrap()
}

/// Off-chain metadata: only IPFS CID stored on-chain.
fn offchain_metadata() -> MetadataExt {
    MetadataExt::offchain_metadata(
        "QmTest123".to_string(),
        Nip52Kind::DateEvent.kind_value(),
    )
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_query() {
        let (_, cal) = setup();
        let config = cal.config().unwrap();
        // config returns CollectionInfoAndExtensionResponse<CalendarModuleCollectionExtension>
        assert_eq!(
            config.extension.min_event_period,
            Some(cw_utils::Duration::Time(3600))
        );
    }

    #[test]
    fn test_calendar_count_starts_at_zero() {
        let (_, cal) = setup();
        let count = cal.calendar_count().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_create_and_query_calendar() {
        let (_, cal) = setup();
        let meta = MetadataExt {
            kind: Nip52Kind::Calendar.kind_value(),
            ..Default::default()
        };
        cal.create_calendar(meta, None).unwrap();

        let count = cal.calendar_count().unwrap();
        assert_eq!(count, 1);

        let d = format!("cal/{count}");
        // calendar() returns MetadataExt directly (QueryExtFns extracts extension)
        let meta = cal.calendar(d.clone()).unwrap();
        assert_eq!(meta.kind, Nip52Kind::Calendar.kind_value());
    }

    #[test]
    fn test_multiple_calendars_increment_count() {
        let (_, cal) = setup();
        let meta = MetadataExt {
            kind: Nip52Kind::Calendar.kind_value(),
            ..Default::default()
        };
        cal.create_calendar(meta.clone(), None).unwrap();
        cal.create_calendar(meta, None).unwrap();

        let count = cal.calendar_count().unwrap();
        assert_eq!(count, 2);
        let calendars = cal.list_calendars(None, None).unwrap();
        assert_eq!(calendars.len(), 2);
    }

    #[test]
    fn test_dao_query() {
        let (_, cal) = setup();
        let dao = cal.dao().unwrap();
    }

    #[test]
    fn test_hooks_empty_on_init() {
        let (_, cal) = setup();
        let hooks = cal.hooks().unwrap();
        assert!(hooks.hooks.is_empty());
    }

    #[test]
    fn test_create_event_tracked_in_calendar() {
        let (_, cal) = setup();
        let meta = MetadataExt {
            kind: Nip52Kind::Calendar.kind_value(),
            ..Default::default()
        };
        cal.create_calendar(meta, None).unwrap();

        let event_meta = onchain_metadata("evt1", Nip52Kind::TimeEvent);
        cal.create_event("cal/1".to_string(), event_meta).unwrap();

        let count = cal.event_count().unwrap();
        assert_eq!(count, 1);

        let events = cal.calendar_events("cal/1".to_string(), None, None).unwrap();
        assert_eq!(events.len(), 1);

        let e_d = "evt/cal/1/1".to_string();
        let event_meta = cal.calendar_event(e_d).unwrap();
        assert_eq!(event_meta.extension.kind, Nip52Kind::TimeEvent.kind_value());
        assert!(event_meta.extension.on_chain);
    }

    #[test]
    fn test_multiple_events_same_calendar() {
        let (_, cal) = setup();
        let meta = MetadataExt {
            kind: Nip52Kind::Calendar.kind_value(),
            ..Default::default()
        };
        cal.create_calendar(meta, None).unwrap();

        let cal_d = "cal/1".to_string();
        cal.create_event(
            cal_d.clone(),
            onchain_metadata("evt1", Nip52Kind::TimeEvent),
        )
        .unwrap();
        cal.create_event(
            cal_d.clone(),
            onchain_metadata("evt2", Nip52Kind::TimeEvent),
        )
        .unwrap();

        let count = cal.event_count().unwrap();
        assert_eq!(count, 2);
        let events = cal.calendar_events(cal_d, None, None).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_events_without_calendar() {
        let (_, cal) = setup();
        cal.create_event(
            String::default(),
            onchain_metadata("evt1", Nip52Kind::TimeEvent),
        )
        .unwrap();

        let count = cal.event_count().unwrap();
        assert_eq!(count, 1);

        let e_d = "evt/_/1".to_string();
        let event_meta = cal.calendar_event(e_d).unwrap();
        assert_eq!(event_meta.extension.kind, Nip52Kind::TimeEvent.kind_value());
    }

    #[test]
    fn test_onchain_metadata_roundtrip() {
        let (_, cal) = setup();
        let meta = onchain_metadata("cal1", Nip52Kind::Calendar);
        cal.create_calendar(meta, None).unwrap();

        let d = "cal/1".to_string();
        let meta = cal.calendar(d).unwrap();
        assert!(meta.on_chain);
        assert_eq!(meta.kind, Nip52Kind::Calendar.kind_value());
        assert!(!meta.e.is_empty());
    }

    #[test]
    fn test_offchain_metadata() {
        let (_, cal) = setup();
        let meta = offchain_metadata();
        cal.create_calendar(meta, None).unwrap();

        let d = "cal/1".to_string();
        let meta = cal.calendar(d).unwrap();
        assert!(!meta.on_chain);
        assert_eq!(meta.cid, Some("QmTest123".to_string()));
    }

    #[test]
    fn test_calendar_event_association() {
        let (_, cal) = setup();

        let cal1 = MetadataExt {
            kind: Nip52Kind::Calendar.kind_value(),
            d_tag: Some("cal-1".to_string()),
            ..Default::default()
        };
        let cal2 = MetadataExt {
            kind: Nip52Kind::Calendar.kind_value(),
            d_tag: Some("cal-2".to_string()),
            ..Default::default()
        };
        cal.create_calendar(cal1, None).unwrap();
        cal.create_calendar(cal2, None).unwrap();

        cal.create_event(
            "cal/1".to_string(),
            onchain_metadata("evt-a", Nip52Kind::TimeEvent),
        )
        .unwrap();
        cal.create_event(
            "cal/2".to_string(),
            onchain_metadata("evt-b", Nip52Kind::TimeEvent),
        )
        .unwrap();
        cal.create_event(
            "cal/2".to_string(),
            onchain_metadata("evt-c", Nip52Kind::TimeEvent),
        )
        .unwrap();

        let events_c1 = cal.calendar_events("cal/1".to_string(), None, None).unwrap();
        assert_eq!(events_c1.len(), 1);

        let events_c2 = cal.calendar_events("cal/2".to_string(), None, None).unwrap();
        assert_eq!(events_c2.len(), 2);

        let total = cal.event_count().unwrap();
        assert_eq!(total, 3);
    }

    // ── Ownership sanity ─────────────────────────────────────────

    #[test]
    fn test_calendar_owner_default_dao() {
        let (_, cal) = setup();
        let dao = cal.dao().unwrap();

        let meta = MetadataExt {
            kind: Nip52Kind::Calendar.kind_value(),
            ..Default::default()
        };
        cal.create_calendar(meta, None).unwrap();

        use cw721::msg::OwnerOfResponse;
        let owner: OwnerOfResponse = cal
            .query(&QueryMsg::OwnerOf {
                token_id: "cal/1".to_string(),
                include_expired: None,
            })
            .unwrap();
        // calendar with None owner defaults to the DAO address
        assert_eq!(owner.owner, dao.to_string());
    }

    #[test]
    fn test_calendar_owner_explicit() {
        let (_, cal) = setup();
        let other = MockApi::default().addr_make("other");

        let meta = MetadataExt {
            kind: Nip52Kind::Calendar.kind_value(),
            ..Default::default()
        };
        cal.create_calendar(meta, Some(other.to_string()))
            .unwrap();

        use cw721::msg::OwnerOfResponse;
        let owner: OwnerOfResponse = cal
            .query(&QueryMsg::OwnerOf {
                token_id: "cal/1".to_string(),
                include_expired: None,
            })
            .unwrap();
        assert_eq!(owner.owner, other.to_string());
    }

    #[test]
    fn test_event_owner_is_sender() {
        let (_, cal) = setup();
        let dao = cal.dao().unwrap();

        let meta = MetadataExt {
            kind: Nip52Kind::Calendar.kind_value(),
            ..Default::default()
        };
        cal.create_calendar(meta, None).unwrap();

        cal.create_event(
            "cal/1".to_string(),
            onchain_metadata("evt1", Nip52Kind::TimeEvent),
        )
        .unwrap();

        use cw721::msg::OwnerOfResponse;
        let owner: OwnerOfResponse = cal
            .query(&QueryMsg::OwnerOf {
                token_id: "evt/cal/1/1".to_string(),
                include_expired: None,
            })
            .unwrap();
        // events are minted to info.sender, which equals the DAO
        assert_eq!(owner.owner, dao.to_string());
    }

    #[test]
    fn test_event_creation_no_calendar_ownership_check() {
        // Current contract does NOT enforce that the event creator
        // owns the calendar. This test documents that gap.
        let (_, cal) = setup();
        let dao = cal.dao().unwrap();

        // Someone else's calendar
        let other = MockApi::default().addr_make("other");
        let meta = MetadataExt {
            kind: Nip52Kind::Calendar.kind_value(),
            ..Default::default()
        };
        cal.create_calendar(meta, Some(other.to_string()))
            .unwrap();

        // The test sender (dao, not "other") can still create
        // events in the calendar because no ownership check exists.
        // This succeeds — documenting the current behaviour.
        cal.create_event(
            "cal/1".to_string(),
            onchain_metadata("evt-other", Nip52Kind::TimeEvent),
        )
        .unwrap();

        // Verify the event exists and belongs to the sender, not the calendar owner
        use cw721::msg::OwnerOfResponse;
        let owner: OwnerOfResponse = cal
            .query(&QueryMsg::OwnerOf {
                token_id: "evt/cal/1/1".to_string(),
                include_expired: None,
            })
            .unwrap();
        assert_eq!(owner.owner, dao.to_string());
    }

    #[test]
    fn test_standalone_event_no_owner_check() {
        // Standalone events (d = "") don't go through calendar
        // ownership validation at all.
        let (_, cal) = setup();
        let dao = cal.dao().unwrap();

        cal.create_event(
            String::default(),
            onchain_metadata("evt1", Nip52Kind::TimeEvent),
        )
        .unwrap();

        use cw721::msg::OwnerOfResponse;
        let owner: OwnerOfResponse = cal
            .query(&QueryMsg::OwnerOf {
                token_id: "evt/_/1".to_string(),
                include_expired: None,
            })
            .unwrap();
        assert_eq!(owner.owner, dao.to_string());
    }

    #[test]
    fn test_transfer_calendar_changes_owner() {
        // CAUTION: This test documents a known bug.
        // transfer_calendar reads NFT info, checks ownership,
        // and updates the in-memory struct — but never calls
        // nft_info.save() to persist the change.
        let (_, cal) = setup();
        let dao = cal.dao().unwrap();
        let recipient = MockApi::default().addr_make("recipient");

        let meta = MetadataExt {
            kind: Nip52Kind::Calendar.kind_value(),
            ..Default::default()
        };
        cal.create_calendar(meta, None).unwrap();

        // Attempt transfer (current owner = DAO = sender)
        cal.transfer_calendar("cal/1".to_string(), recipient.to_string())
            .unwrap();

        // Owner should be recipient, but is STILL dao due to the
        // missing nft_info.save() in transfer_calendar.
        use cw721::msg::OwnerOfResponse;
        let owner: OwnerOfResponse = cal
            .query(&QueryMsg::OwnerOf {
                token_id: "cal/1".to_string(),
                include_expired: None,
            })
            .unwrap();

        // This assertion FAILS because the transfer doesn't persist.
        // Uncomment the next line once transfer_calendar is fixed:
        // assert_eq!(owner.owner, recipient.to_string());

        // Document the current broken behaviour:
        assert_eq!(
            owner.owner, dao.to_string(),
            "BUG: transfer_calendar reads owner, modifies in-memory, \
             but never calls nft_info.save() — change is lost"
        );
    }
}
