use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint256;
use dao_dao_macros::{active_query, voting_module_query};
use dao_voting::threshold::{ActiveThreshold, ActiveThresholdResponse};

// ---------------------------------------------------------------------------
// Instantiate
// ---------------------------------------------------------------------------

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the underlying voting module that provides real voter
    /// membership data and token weights (e.g. dao-voting-cw20-staked).
    pub underlying_voting_module: String,
    /// Address of the PollRegistry contract that manages ZK poll lifecycle.
    pub poll_registry: String,
    /// Optional active threshold for DAO activity check.
    pub active_threshold: Option<ActiveThreshold>,
}

// ---------------------------------------------------------------------------
// Execute
// ---------------------------------------------------------------------------

#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    /// Update the underlying voting module or PollRegistry address.
    /// Only callable by the DAO.
    UpdateConfig {
        underlying_voting_module: Option<String>,
        poll_registry: Option<String>,
    },
    /// Sets the active threshold to a new value. Only the DAO may call this.
    UpdateActiveThreshold {
        new_threshold: Option<ActiveThreshold>,
    },
    /// Create a snapshot of the voter set for a given proposal ID and register
    /// the Merkle root with PollRegistry.
    ///
    /// The merkle_root must be computed OFF-CHAIN by enumerating voters from
    /// the underlying voting module at the current block height.
    /// total_power must match the underlying voting module's total power
    /// at this height.
    SnapshotVoters {
        /// DAO proposal ID this snapshot corresponds to.
        proposal_id: u64,
        /// Hex-encoded Merkle root of eligible voter identity commitments.
        merkle_root: String,
        /// Total voting power at snapshot time from underlying module.
        total_power: Uint256,
    },
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

#[voting_module_query]
#[active_query]
#[cw_serde]
#[derive(QueryResponses)]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
pub enum QueryMsg {
    /// Returns the underlying voting module address.
    #[returns(cosmwasm_std::Addr)]
    UnderlyingVotingModule {},
    /// Returns the PollRegistry contract address.
    #[returns(cosmwasm_std::Addr)]
    PollRegistry {},
    /// Returns snapshot details for a given proposal ID.
    #[returns(SnapshotResponse)]
    Snapshot {
        proposal_id: u64,
    },
    /// Returns the active threshold configuration.
    #[returns(ActiveThresholdResponse)]
    ActiveThreshold {},
}

// ---------------------------------------------------------------------------
// Custom response types
// ---------------------------------------------------------------------------

#[cw_serde]
pub struct SnapshotResponse {
    pub proposal_id: u64,
    pub merkle_root: String,
    pub total_power: Uint256,
    pub created_at: u64,
    pub poll_id: String,
}

// ---------------------------------------------------------------------------
// Expected PollRegistry query interface
// These types are used by this contract to query PollRegistry.
// They must match PollRegistry's actual query interface when both are deployed.
// ---------------------------------------------------------------------------

/// The expected query message for PollRegistry's GetPoll query.
#[cw_serde]
pub enum PollRegistryQuery {
    GetPoll {
        poll_id: String,
    },
    GetTally {
        poll_id: String,
    },
    VerifyNullifier {
        poll_id: String,
        nullifier: String,
    },
}

#[cw_serde]
pub struct PollState {
    pub poll_id: String,
    pub merkle_root: String,
    pub total_power: Uint256,
    pub status: String, // "setup" | "active" | "tally" | "complete"
    pub start_block: u64,
    pub end_block: u64,
}

#[cw_serde]
pub struct TallyResult {
    pub poll_id: String,
    pub votes_yes: Uint256,
    pub votes_no: Uint256,
    pub votes_abstain: Uint256,
    pub total_cast: Uint256,
    pub is_final: bool,
}

// ---------------------------------------------------------------------------
// Migrate
// ---------------------------------------------------------------------------

#[cw_serde]
pub struct MigrateMsg {}