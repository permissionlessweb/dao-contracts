//! Shadow types for migrating from v1 to v2 state.
//!
//! Because the v1 crates depend on cosmwasm-std v1 while this crate uses
//! cosmwasm-std v2, we cannot directly call v1 `Item::load` / `Map::range`
//! (the `Storage` trait is a different type). Instead we define shadow structs
//! that mirror the v1 storage layout but use v2 cosmwasm-std types, and
//! deserialize directly from raw storage bytes. The JSON serialization format
//! for Addr, Uint128, Expiration, etc. is identical between v1 and v2, so
//! this works transparently.

use cosmwasm_std::{Addr, CosmosMsg, Empty, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::{Duration, Expiration};
use dao_voting::{
    status::Status,
    threshold::Threshold,
    voting::Votes,
};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Shadow types that match the v1 storage layout but use v2 cosmwasm-std types.
// ---------------------------------------------------------------------------

/// Mirrors `cw_proposal_single_v1::state::CheckedDepositInfo`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct V1CheckedDepositInfo {
    pub token: Addr,
    pub deposit: Uint128,
    pub refund_failed_proposals: bool,
}

/// Mirrors `cw_proposal_single_v1::state::Config`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct V1Config {
    pub threshold: Threshold,
    pub max_voting_period: Duration,
    pub min_voting_period: Option<Duration>,
    pub only_members_execute: bool,
    pub allow_revoting: bool,
    pub dao: Addr,
    pub deposit_info: Option<V1CheckedDepositInfo>,
}

/// Mirrors `cw_proposal_single_v1::proposal::Proposal`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct V1Proposal {
    pub title: String,
    pub description: String,
    pub proposer: Addr,
    pub start_height: u64,
    pub min_voting_period: Option<Expiration>,
    pub expiration: Expiration,
    pub threshold: Threshold,
    pub total_power: Uint128,
    pub msgs: Vec<CosmosMsg<Empty>>,
    pub status: Status,
    pub votes: Votes,
    pub allow_revoting: bool,
    pub deposit_info: Option<V1CheckedDepositInfo>,
}

/// V2-compatible storage accessors pointing at the same storage keys as the
/// v1 crate. These use the v2 `Storage` trait so they work with `deps.storage`.
pub const V1_CONFIG: Item<V1Config> = Item::new("config");
pub const V1_PROPOSALS: Map<u64, V1Proposal> = Map::new("proposals");
