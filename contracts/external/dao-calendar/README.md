# DAO Calendar

On-chain event scheduling, group-based authorization, and time-triggered governance for DAOs.

---

## Overview

DAO Calendar is a proposal module that brings **time-aware governance** to DAOs. Instead of proposals being abstract votes, calendar events are first-class on-chain entities with start times, end times, managing groups, and — critically — **gauge messages that automatically execute through the DAO when events begin and end**.

Think of it as a cron scheduler crossed with a governance module: events represent things that happen over time, groups control who can manage those events, and gauges are the actions that fire when the calendar says so.

### Why a Calendar?

Most DAO governance is reactive — someone submits a proposal, people vote, it executes. But many DAO operations are **temporal**:

- Recurring reward distributions that start and stop on schedule
- Seasonal governance periods (e.g., "budget season")
- Time-boxed delegation windows
- Event-driven gauge weight adjustments
- Coordinated multi-contract state changes at known future times

The calendar module makes these patterns first-class. An event isn't just a record — it's a **trigger** that can execute DAO-level messages when its start time and end time are reached.

---

## Core Concepts

### Events

An event is the fundamental unit. It has:

| Field | Purpose |
|-------|---------|
| `id` | Auto-incrementing unique identifier |
| `title` | Human-readable name |
| `description` | Optional details |
| `start_time` / `end_time` | Timestamps defining the event window |
| `managing_groups` | Which groups can modify/cancel this event |
| `event_services` | Named service endpoints attached to the event |
| `extension` | Generic metadata field (CW721-style, default `Empty`) |
| `status` | Lifecycle state: `Upcoming` → `Active` → `Completed` or `Cancelled` |
| `created_by` / `created_at` | Audit trail |

#### Event Lifecycle

```
                    ┌──────────────┐
                    │   Upcoming   │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
         (cancelled)  (start_time    (cancelled)
              │        reached)         │
              │            │            │
              ▼            ▼            │
       ┌──────────┐ ┌──────────┐       │
       │ Cancelled │ │  Active  │       │
       └──────────┘ └────┬─────┘       │
                         │             │
              ┌──────────┼────────────┐│
              │          │            ││
         (cancelled) (end_time     (cancelled)
              │      reached)         │
              │          │            │
              ▼          ▼            ▼
       ┌──────────┐ ┌──────────┐ ┌──────────┐
       │ Cancelled │ │Completed │ │ Cancelled │
       └──────────┘ └──────────┘ └──────────┘
```

- **Upcoming**: Event has been created but hasn't started yet. Can be updated or cancelled.
- **Active**: Start time has been reached (via `TriggerEventStart`). Gauge begin-messages execute. Can be cancelled.
- **Completed**: End time has been reached (via `TriggerEventEnd`). Gauge end-messages execute. Immutable.
- **Cancelled**: Removed from the active timeline at any point before completion.

Triggering is **manual** — someone must call `TriggerEventStart` or `TriggerEventEnd` after the relevant timestamp has passed. This is by design: on-chain time is block time, and the calendar doesn't run a daemon. Triggers can be called by anyone once the time condition is met.

---

### Groups

Groups are the **authorization backbone** of the calendar. A group is:

```rust
Group {
    id: String,           // Unique identifier (e.g., "treasury", "dev-team")
    dao: Addr,            // The DAO that owns this group
    suppliers: Vec<EventSupplier>,  // Contracts providing capabilities
}
```

Groups serve two purposes:

1. **Authorization**: Events specify which groups can manage them. Any supplier in a managing group with type `Authorization` can create, update, or cancel events in that group.
2. **Gauge Discovery**: When querying event gauges, the calendar surfaces which groups have gauge-type suppliers attached.

#### Suppliers

Each group contains suppliers — external contracts that provide capabilities to the group:

| Supplier Type | Purpose |
|---------------|---------|
| `Authorization` | This contract answers "is address X authorized?" — used for event management permissions |
| `Gauge` | This contract is a gauge target — surfaced in gauge queries for coordination |
| `Service` | This contract provides a service — informational, for service discovery |
| `Account` | This contract manages an account — informational, for accounting |

The authorization check flow:

```
Sender wants to create/update/cancel an event
         │
         ▼
    Is sender the DAO? ──Yes──→ Authorized
         │
         No
         │
         ▼
    For each managing_group in event:
         │
         ▼
    For each supplier in group:
         │
         ▼
    Is supplier type == Authorization? ──No──→ Continue
         │
         Yes
         │
         ▼
    Query supplier.contract: IsAuthorized { sender }
         │
         ▼
    authorized == true? ──Yes──→ Authorized
         │
         No
         │
         ▼
    Continue checking...
         │
         ▼
    No group authorized sender → Unauthorized error
```

**The DAO itself is always authorized** — it's the super-admin that can bypass all group checks.

---

### Gauges

Gauges are the **action layer**. When an event starts or ends, gauge messages execute through the DAO core via `ExecuteProposalHook`.

There are two ways gauges attach to events:

#### 1. Event-Level Gauges (Explicit)

Registered directly on an event via `RegisterEventGauges`:

```rust
RegisterEventGauges {
    event_id: 5,
    begin_msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute { ... })],  // Fires on start
    end_msgs: vec![CosmosMsg::Wasm(WasmMsg::Execute { ... })],    // Fires on end
}
```

These are arbitrary `CosmosMsg` values that get wrapped in `dao_core::ExecuteProposalHook` and executed by the DAO when the event is triggered.

#### 2. Group-Level Gauges (Discovery)

Groups with `Gauge`-type suppliers are surfaced in queries but **are not automatically executed**. They serve as a discovery mechanism — frontends and off-chain indexers can see which gauge contracts are associated with an event's managing groups and coordinate accordingly.

The `EventGauges` query returns both:

```rust
EventGaugeResponse {
    event_id: 5,
    begin_msgs: [...],     // From explicit event-level registration
    end_msgs: [...],       // From explicit event-level registration
    group_gauges: [        // From managing groups' gauge suppliers
        GroupGaugeSummary {
            group_id: "rewards",
            gauges: [GaugeConfig { label: "...", ... }],
        }
    ],
}
```

---

### Event Services

Events can carry **named service endpoints** — references to contracts and messages that are relevant to the event. These are informational and don't auto-execute; they exist for discovery and frontend integration.

```rust
EventService {
    name: "voting_portal".to_string(),
    description: Some("Link to the voting interface for this election"),
    target_contract: Some("cosmos1..."),
    msgs: vec![],  // Template messages, not auto-executed
}
```

Use cases:
- A governance period event listing the voting contract
- A funding round event linking to the application contract
- A hackathon event pointing to the submission contract

---

### Calendar Hooks

The calendar fires **hook messages** to registered contracts when events change state:

| Hook | When |
|------|------|
| `EventCreated` | A new event is created |
| `EventStatusChanged` | An event transitions (Upcoming→Active, Active→Completed) |
| `EventCancelled` | An event is cancelled |

Hooks are registered by the DAO via `AddEventHook`. They're useful for:
- Indexing services tracking event state
- Notification systems alerting on changes
- Downstream contracts that need to react to calendar events

If a hook execution fails, the calendar **removes the failing hook** via the reply handler rather than reverting the entire transaction. This ensures a broken hook doesn't block calendar operations.

---

### Calendar Renewal

Over time, completed events accumulate. `RenewCalendar` prunes events older than one year:

```rust
RenewCalendar { limit: Some(50) }  // Process up to 50 old events
```

This removes:
- The event record
- Event-group join table entries
- Event gauge records

Renewal is idempotent and can be called by anyone. It's a maintenance operation, similar to garbage collection.

---

## Extension Pattern

The calendar follows the **CW721 extension pattern** for event metadata. The default contract uses `Event<Empty>` — no custom metadata. But any DAO can define their own:

```rust
#[cw_serde]
pub struct HackathonMetadata {
    pub prize_pool: Vec<Coin>,
    pub judges: Vec<String>,
    pub submission_deadline: Timestamp,
}

type HackathonEvent = Event<HackathonMetadata>;
type HackathonExecuteMsg = ExecuteMsg<HackathonMetadata>;
```

This allows the calendar to be customized per-DAO without modifying the core contract. The extension field is carried through create/update operations and stored alongside the event.

---

## Setup Patterns

### Minimal: Calendar as Event Log

The simplest setup — just track events on-chain with no gauges or groups:

```rust
InstantiateMsg {
    initial_groups: None,  // No groups needed
}
```

Create events with empty managing groups (only the DAO can manage them). No gauges, no services. Pure on-chain scheduling.

### Standard: Group-Authorized Events

Add groups to delegate event management:

```rust
InstantiateMsg {
    initial_groups: Some(vec![
        GroupInit {
            id: "core-team".to_string(),
            suppliers: vec![EventSupplierInit {
                contract: "cosmos1...auth_contract".to_string(),
                supplier_type: EventSupplierType::Authorization,
            }],
        },
    ]),
}
```

Now the core team's authorization contract controls who can create events in the "core-team" group.

### Advanced: Time-Triggered Gauge Execution

The full power mode — events that execute DAO messages on schedule:

1. **Create the event** with managing groups:
   ```rust
   CreateEvent {
       title: "Q2 Rewards Distribution".to_string(),
       start_time: Timestamp::from_seconds(1717200000),
       end_time: Timestamp::from_seconds(1719792000),
       managing_groups: vec!["rewards-committee".to_string()],
       ...
   }
   ```

2. **Register gauge messages**:
   ```rust
   RegisterEventGauges {
       event_id: 1,
       begin_msgs: vec![/* start distributing rewards */],
       end_msgs: vec![/* stop distributing rewards */],
   }
   ```

3. **When start time is reached**, anyone calls `TriggerEventStart { event_id: 1 }`. The calendar:
   - Changes status to `Active`
   - Fires begin_msgs through `dao_core::ExecuteProposalHook`
   - Notifies hook subscribers

4. **When end time is reached**, anyone calls `TriggerEventEnd { event_id: 1 }`. The calendar:
   - Changes status to `Completed`
   - Fires end_msgs through `dao_core::ExecuteProposalHook`
   - Notifies hook subscribers

### Calendar as Proposal Module

The calendar implements the DAO proposal module interface (`Dao` query, `NextProposalId` query). This means it can be registered as a proposal module in `dao_core`:

```rust
// In DaoConfig
ProposalModuleConfig::Calendar(CalendarDeployData { ... })
```

When registered this way, the calendar's events are tracked as proposals in the DAO's governance system. The `NextProposalId` query returns the next event ID, maintaining compatibility with DAO tooling.

---

## Authorization Model Summary

| Action | Who Can Do It |
|--------|---------------|
| Create event | DAO, or any address authorized by a managing group's Authorization supplier |
| Update event | DAO, or authorized address from the event's current managing groups |
| Cancel event | DAO, or authorized address from the event's current managing groups |
| Register gauges | DAO, or authorized address from the event's managing groups |
| Trigger start/end | **Anyone** (once the time condition is met) |
| Register/update/remove groups | DAO only |
| Add/remove event hooks | DAO only |
| Renew calendar | **Anyone** |

The key insight: **management is permissioned, execution is permissionless**. Only authorized addresses can set up what happens, but anyone can trigger it once the time is right. This mirrors how anyone can execute a passed proposal — the governance happened in the setup, not the trigger.

---

## Query Capabilities

Events are indexed by start time, end time, and status, enabling efficient queries:

- **By groups**: Find all events managed by specific groups
- **By time range**: Find events within a time window (uses the start-time index)
- **By status**: Find all upcoming, active, completed, or cancelled events
- **Reverse pagination**: Walk events backwards from a given ID
- **Full state dump**: Get everything in one query for frontend hydration

---

## Design Principles

1. **Time is first-class**: Events aren't just records — they have temporal semantics with lifecycle transitions.

2. **Authorization is composable**: Groups with external authorization contracts let you plug in any access control logic — multisig, token-gated, NFT-gated, etc.

3. **Gauges are DAO-native**: Gauge messages execute through `ExecuteProposalHook`, meaning they have all the same permissions as any passed proposal. No special privileges needed.

4. **Extension is generic**: The CW721-style metadata extension means the calendar adapts to any DAO's needs without forking.

5. **Hooks are resilient**: A failing hook doesn't break the calendar — it gets removed. Operations take priority over notifications.

6. **Maintenance is permissionless**: Renewal and triggering are public goods. The DAO doesn't need to maintain its own calendar.