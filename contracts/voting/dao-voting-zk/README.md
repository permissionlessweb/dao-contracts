# DAO-DAO ZK Voting Adapter (`dao-voting-zk`)

A CosmWasm voting module that integrates zero-knowledge voting into the
DA0-DAO governance framework. This contract wraps an existing voting module
and enables privacy-preserving voting through Merkle tree snapshots and
PollRegistry-based ZK proof verification.

## Overview

Standard DAO voting systems (cw20-staked, cw4, etc.) reveal every vote on
chain. The ZK Voting Adapter changes this by:

1. Snapshotting the eligible voter set as a Merkle tree root
2. Registering the root with PollRegistry for ZK proof verification
3. Delegating voting power queries through the PollRegistry
4. Returning aggregate results without revealing individual votes

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  dao-contracts DAO Framework                                 │
│  ┌──────────────────────────────────────┐                   │
│  │  dao-voting-zk (this contract)       │                   │
│  │  - wraps underlying voting module    │                   │
│  │  - manages Merkle snapshot state     │                   │
│  │  - routes queries through registry   │                   │
│  └──────────┬─────────────┬────────────┘                   │
│             │             │                                  │
│             ▼             ▼                                  │
│  ┌─────────────────┐  ┌──────────────────────┐              │
│  │ Underlying      │  │ PollRegistry         │              │
│  │ Voting Module   │  │ (ZK poll lifecycle)  │              │
│  │ (cw20-staked /  │  │ - registers roots    │              │
│  │  cw4 / etc.)    │  │ - verifies ZK proofs │              │
│  └─────────────────┘  │ - accumulates tally  │              │
│                       └──────────────────────┘              │
└─────────────────────────────────────────────────────────────┘
```

## Integration Flow

### 1. DAO Setup

```rust
// Instantiate the ZK voting adapter with:
// - A reference to the underlying voting module (e.g., dao-voting-cw20-staked)
// - The PollRegistry contract address
// - Optional active threshold
InstantiateMsg {
    underlying_voting_module: "terp1...underlying...",
    poll_registry: "terp1...pollregistry...",
    active_threshold: Some(ActiveThreshold::AbsoluteCount { count: 100 }),
}
```

### 2. Proposal Lifecycle

| Step | Actor | Action |
|------|-------|--------|
| 1 | DAO | Creates a proposal via `dao-proposal-single` |
| 2 | Coordinator (off-chain) | Queries the underlying voting module for all eligible voters and their weights |
| 3 | Coordinator (off-chain) | Builds a Merkle tree of voter identity commitments |
| 4 | DAO/Coordinator | Calls `SnapshotVoters { proposal_id, merkle_root, total_power }` on this contract |
| 5 | This contract | Registers the Merkle root with PollRegistry |
| 6 | Voters (off-chain) | Generate ZK proofs of membership + vote preference |
| 7 | PollRegistry | Verifies ZK proofs, accumulates tally |
| 8 | This contract | Queries PollRegistry for final tally at proposal close |
| 9 | dao-proposal-single | Uses voting power from this contract to determine outcome |

### 3. Voting Power Queries

- **`VotingPowerAtHeight { address, height }`**
  - If `height` has an active ZK snapshot: queries PollRegistry for voter's
    ZK proof status and returns the voter's underlying weight if proven
  - If no snapshot at `height`: delegates to the underlying voting module

- **`TotalPowerAtHeight { height }`**
  - If `height` has an active ZK snapshot: returns the snapshot's `total_power`
    (maximum possible voting power from eligible voters)
  - If no snapshot at `height`: delegates to the underlying voting module

## Contract Messages

### Execute

| Message | Description | Auth |
|---------|-------------|------|
| `SnapshotVoters { proposal_id, merkle_root, total_power }` | Create a Merkle snapshot for a proposal and register with PollRegistry | DAO only |
| `UpdateConfig { underlying_voting_module, poll_registry }` | Update contract configuration | DAO only |
| `UpdateActiveThreshold { new_threshold }` | Set active threshold for DAO activity check | DAO only |

### Query

| Message | Returns | Description |
|---------|---------|-------------|
| `VotingPowerAtHeight { address, height }` | `VotingPowerAtHeightResponse` | Voting power with ZK integration |
| `TotalPowerAtHeight { height }` | `TotalPowerAtHeightResponse` | Total power with ZK integration |
| `Info {}` | `InfoResponse` | Contract version info |
| `Dao {}` | `Addr` | DAO address |
| `IsActive {}` | `bool` | Whether DAO is active |
| `ActiveThreshold {}` | `ActiveThresholdResponse` | Active threshold config |
| `UnderlyingVotingModule {}` | `Addr` | Underlying voting module address |
| `PollRegistry {}` | `Addr` | PollRegistry contract address |
| `Snapshot { proposal_id }` | `SnapshotResponse` | Snapshot details for a proposal |

## Dependencies

This contract depends on:

- **PollRegistry CosmWasm contract** (Phase 2b) — provides the `register_poll`
  execute endpoint and the `GetPoll` / `GetTally` query endpoints
- **Underlying voting module** — any existing dao-contracts voting module
  (dao-voting-cw20-staked, dao-voting-cw4, dao-voting-token-staked, etc.)
- **zk-wasmvm** — Terp Network's ZK proof verification host extension
  (used by PollRegistry, not directly by this contract)

## Expected PollRegistry Interface

This contract expects PollRegistry to expose:

```rust
// Execute
enum PollRegistryExecute {
    RegisterPoll {
        poll_id: String,
        merkle_root: String,
        total_power: String,  // Uint256 as decimal string
    },
}

// Query
enum PollRegistryQuery {
    GetPoll { poll_id: String },
    GetTally { poll_id: String },
}
```

These types are defined in `msg.rs` as `PollRegistryQuery` and `PollState`.

## Configuration

To configure a DAO for ZK voting:

1. Deploy the ZK voting adapter contract
2. Set it as the DAO's voting module via DAO governance
3. Ensure a PollRegistry contract is deployed and accessible
4. The underlying voting module (e.g., dao-voting-cw20-staked) must have
   a stable member set for snapshotting
5. A coordinator service must compute Merkle roots off-chain and submit
   them via `SnapshotVoters`

## Migrating from Existing Voting Modules

If a DAO already uses `dao-voting-cw20-staked` or similar:

1. Deploy `dao-voting-zk` with the existing voting module as `underlying_voting_module`
2. Update the DAO's voting module address to the new `dao-voting-zk` contract
3. Existing proposals that started before the migration will fall through
   to the underlying module (no ZK)
4. New proposals will use ZK voting through the snapshot mechanism

## Security Considerations

- Merkle roots must be computed off-chain from a reliable member enumeration
- The `SnapshotVoters` call must be authorized (DAO-only) to prevent
  malicious root registration
- The contract trusts the PollRegistry for proof verification — a compromised
  PollRegistry could return invalid tallies
- Voting power queries for snapshots use the underlying module's historical
  weights at snapshot time, preventing flash-loan manipulation