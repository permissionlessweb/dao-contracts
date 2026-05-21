use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint256;
use cw_storage_plus::{Item, Map};

/// The DAO that instantiated this voting module.
pub const DAO: Item<cosmwasm_std::Addr> = Item::new("dao");

/// The underlying voting module that provides the real voter membership
/// and token weights. This adapter delegates to it for membership queries
/// and wraps results with ZK proof verification via PollRegistry.
pub const UNDERLYING_VOTING_MODULE: Item<cosmwasm_std::Addr> = Item::new("underlying_voting_module");

/// The PollRegistry contract address that manages ZK poll lifecycle
/// (merkle root registration, proof verification, tally accumulation).
pub const POLL_REGISTRY: Item<cosmwasm_std::Addr> = Item::new("poll_registry");

/// Active threshold config for DAO activity check.
pub const ACTIVE_THRESHOLD: Item<dao_voting::threshold::ActiveThreshold> =
    Item::new("active_threshold");

/// A snapshot of the voter set taken at proposal creation time.
/// The Merkle root is registered with PollRegistry so voters can prove
/// membership via ZK proofs.
#[cw_serde]
pub struct Snapshot {
    /// The DAO proposal ID this snapshot corresponds to.
    pub proposal_id: u64,
    /// Hex-encoded Merkle root of eligible voter identity commitments,
    /// computed from the underlying voting module's member set at snapshot time.
    pub merkle_root: String,
    /// Total voting power at snapshot time (from underlying module).
    pub total_power: Uint256,
    /// Block height at which the snapshot was taken.
    pub created_at: u64,
    /// PollRegistry poll ID assigned for this snapshot's ZK poll.
    pub poll_id: String,
}

/// proposal_id -> Snapshot
pub const SNAPSHOTS: Map<u64, Snapshot> = Map::new("snapshots");

/// poll_id -> proposal_id (reverse lookup)
pub const POLL_MAP: Map<&str, u64> = Map::new("poll_map");