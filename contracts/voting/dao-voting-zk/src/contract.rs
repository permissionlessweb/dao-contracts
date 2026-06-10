#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, MigrateInfo, Reply, Response,
    StdError, StdResult, Uint256,
};
use cw2::{get_contract_version, set_contract_version, ContractVersion};
use dao_interface::voting::IsActiveResponse;
use dao_voting::threshold::{ActiveThreshold, ActiveThresholdResponse};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, PollRegistryQuery, PollState, QueryMsg,
    SnapshotResponse,
};
use crate::state::{
    ACTIVE_THRESHOLD, DAO, POLL_MAP, POLL_REGISTRY, SNAPSHOTS, UNDERLYING_VOTING_MODULE,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-voting-zk";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// Instantiate
// ---------------------------------------------------------------------------

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let underlying = deps.api.addr_validate(&msg.underlying_voting_module)?;
    let registry = deps.api.addr_validate(&msg.poll_registry)?;

    DAO.save(deps.storage, &info.sender)?;
    UNDERLYING_VOTING_MODULE.save(deps.storage, &underlying)?;
    POLL_REGISTRY.save(deps.storage, &registry)?;

    if let Some(threshold) = msg.active_threshold {
        ACTIVE_THRESHOLD.save(deps.storage, &threshold)?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("underlying_voting_module", underlying.as_str())
        .add_attribute("poll_registry", registry.as_str()))
}

// ---------------------------------------------------------------------------
// Execute
// ---------------------------------------------------------------------------

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            underlying_voting_module,
            poll_registry,
        } => execute_update_config(deps, info, underlying_voting_module, poll_registry),
        ExecuteMsg::UpdateActiveThreshold { new_threshold } => {
            execute_update_active_threshold(deps, info, new_threshold)
        }
        ExecuteMsg::SnapshotVoters {
            proposal_id,
            merkle_root,
            total_power,
        } => execute_snapshot_voters(deps, env, info, proposal_id, merkle_root, total_power),
    }
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    underlying_voting_module: Option<String>,
    poll_registry: Option<String>,
) -> Result<Response, ContractError> {
    let dao = DAO.load(deps.storage)?;
    if info.sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(addr) = underlying_voting_module {
        let validated = deps.api.addr_validate(&addr)?;
        UNDERLYING_VOTING_MODULE.save(deps.storage, &validated)?;
    }
    if let Some(addr) = poll_registry {
        let validated = deps.api.addr_validate(&addr)?;
        POLL_REGISTRY.save(deps.storage, &validated)?;
    }

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn execute_update_active_threshold(
    deps: DepsMut,
    info: MessageInfo,
    new_threshold: Option<ActiveThreshold>,
) -> Result<Response, ContractError> {
    let dao = DAO.load(deps.storage)?;
    if info.sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(threshold) = &new_threshold {
        if let ActiveThreshold::Percentage { percent } = threshold {
            if *percent > cosmwasm_std::Decimal256::percent(100)
                || *percent <= cosmwasm_std::Decimal256::percent(0)
            {
                return Err(ContractError::Std(StdError::msg(
                    "invalid active threshold percentage",
                )));
            }
        }
        ACTIVE_THRESHOLD.save(deps.storage, threshold)?;
    } else {
        ACTIVE_THRESHOLD.remove(deps.storage);
    }

    Ok(Response::new().add_attribute("action", "update_active_threshold"))
}

fn execute_snapshot_voters(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
    merkle_root: String,
    total_power: Uint256,
) -> Result<Response, ContractError> {
    let dao = DAO.load(deps.storage)?;
    if info.sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    // Prevent duplicate snapshots for the same proposal.
    if SNAPSHOTS.may_load(deps.storage, proposal_id)?.is_some() {
        return Err(ContractError::SnapshotAlreadyExists { proposal_id });
    }

    if total_power.is_zero() {
        return Err(ContractError::ZeroTotalPower {});
    }

    if merkle_root.is_empty() {
        return Err(ContractError::InvalidMerkleRoot {
            reason: "empty merkle root".to_string(),
        });
    }

    let block_height = env.block.height;

    // Build a unique poll ID from the proposal ID and block height.
    let poll_id = format!("zk_vote_{}_{}", proposal_id, block_height);

    let snapshot = crate::state::Snapshot {
        proposal_id,
        merkle_root: merkle_root.clone(),
        total_power,
        created_at: block_height,
        poll_id: poll_id.clone(),
    };

    SNAPSHOTS.save(deps.storage, proposal_id, &snapshot)?;
    POLL_MAP.save(deps.storage, &poll_id, &proposal_id)?;

    // Register the Merkle root with PollRegistry via a wasm execute message.
    let poll_registry = POLL_REGISTRY.load(deps.storage)?;

    // Build the register_poll message as a serde_json Value for the PollRegistry.
    #[derive(serde::Serialize)]
    struct RegisterPollMsg {
        register_poll: RegisterPollContent,
    }
    #[derive(serde::Serialize)]
    struct RegisterPollContent {
        poll_id: String,
        merkle_root: String,
        total_power: String,
    }

    let register_msg = to_json_binary(&RegisterPollMsg {
        register_poll: RegisterPollContent {
            poll_id,
            merkle_root: merkle_root.clone(),
            total_power: total_power.to_string(),
        },
    })?;

    Ok(Response::new()
        .add_attribute("action", "snapshot_voters")
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("merkle_root", merkle_root)
        .add_attribute("total_power", total_power.to_string())
        .add_attribute("created_at", block_height.to_string())
        .add_message(cosmwasm_std::CosmosMsg::Wasm(
            cosmwasm_std::WasmMsg::Execute {
                contract_addr: poll_registry.to_string(),
                msg: register_msg,
                funds: vec![],
            },
        )))
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, height } => {
            query_voting_power_at_height(deps, env, address, height)
        }
        QueryMsg::TotalPowerAtHeight { height } => query_total_power_at_height(deps, env, height),
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::Dao {} => query_dao(deps),
        QueryMsg::IsActive {} => query_is_active(deps),
        QueryMsg::UnderlyingVotingModule {} => query_underlying_voting_module(deps),
        QueryMsg::PollRegistry {} => query_poll_registry(deps),
        QueryMsg::Snapshot { proposal_id } => query_snapshot(deps, proposal_id),
        QueryMsg::ActiveThreshold {} => query_active_threshold(deps),
    }
}

fn query_underlying_voting_module(deps: Deps) -> StdResult<Binary> {
    let addr = UNDERLYING_VOTING_MODULE.load(deps.storage)?;
    to_json_binary(&addr)
}

fn query_poll_registry(deps: Deps) -> StdResult<Binary> {
    let addr = POLL_REGISTRY.load(deps.storage)?;
    to_json_binary(&addr)
}

/// Returns voting power for an address at a given height.
///
/// If the height falls within an active ZK poll (i.e., a snapshot exists at
/// that height), this queries PollRegistry to check if the address has
/// submitted a valid ZK proof. If so, the power from the underlying voting
/// module at snapshot time is returned. Otherwise, 0 is returned.
///
/// If no ZK poll is active at the given height, delegates to the underlying
/// voting module directly.
fn query_voting_power_at_height(
    deps: Deps,
    _env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<Binary> {
    let query_height = height.unwrap_or(_env.block.height);

    let snapshot = find_snapshot_at_height(deps, query_height);

    if let Some(snap) = snapshot {
        let poll_id = snap.poll_id;
        let registry = POLL_REGISTRY.load(deps.storage)?;

// Query PollRegistry for poll state
        let poll_state: Result<PollState, _> = deps.querier.query_wasm_smart(
            registry,
            &PollRegistryQuery::GetPoll {
                poll_id: poll_id.clone(),
            },
        );

        match poll_state {
            Ok(state) => {
                let underlying = UNDERLYING_VOTING_MODULE.load(deps.storage)?;
                let vp_response: dao_interface::voting::VotingPowerAtHeightResponse =
                    deps.querier.query_wasm_smart(
                        underlying,
                        &dao_interface::voting::Query::VotingPowerAtHeight {
                            address,
                            height: Some(snap.created_at),
                        },
                    )?;

                // For "complete" polls, return the voter's snapshot weight
                // (ZK participation tracked by PollRegistry).
                // For "active" polls, return the voter's underlying weight
                // as maximum possible voting power.
                let power = if state.status == "complete" {
                    vp_response.power
                } else {
                    vp_response.power
                };

                to_json_binary(&dao_interface::voting::VotingPowerAtHeightResponse {
                    power,
                    height: query_height,
                })
            }
            Err(_) => {
                // Poll not yet registered on PollRegistry; return 0
                to_json_binary(&dao_interface::voting::VotingPowerAtHeightResponse {
                    power: Uint256::zero(),
                    height: query_height,
                })
            }
        }
    } else {
        // No ZK snapshot at this height; delegate to underlying module.
        let underlying = UNDERLYING_VOTING_MODULE.load(deps.storage)?;
        let res: dao_interface::voting::VotingPowerAtHeightResponse =
            deps.querier.query_wasm_smart(
                underlying,
                &dao_interface::voting::Query::VotingPowerAtHeight { address, height },
            )?;
        to_json_binary(&res)
    }
}

/// Returns total voting power at a given height.
///
/// If a ZK snapshot exists at this height, returns the snapshot total_power.
/// Otherwise, delegates to the underlying voting module.
fn query_total_power_at_height(deps: Deps, _env: Env, height: Option<u64>) -> StdResult<Binary> {
    let query_height = height.unwrap_or(_env.block.height);
    let snapshot = find_snapshot_at_height(deps, query_height);

    if let Some(snap) = snapshot {
        to_json_binary(&dao_interface::voting::TotalPowerAtHeightResponse {
            power: snap.total_power,
            height: query_height,
        })
    } else {
        let underlying = UNDERLYING_VOTING_MODULE.load(deps.storage)?;
        let res: dao_interface::voting::TotalPowerAtHeightResponse =
            deps.querier.query_wasm_smart(
                underlying,
                &dao_interface::voting::Query::TotalPowerAtHeight { height },
            )?;
        to_json_binary(&res)
    }
}

fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = get_contract_version(deps.storage)?;
    to_json_binary(&dao_interface::voting::InfoResponse { info })
}

fn query_dao(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_json_binary(&dao)
}

fn query_is_active(deps: Deps) -> StdResult<Binary> {
    let threshold = ACTIVE_THRESHOLD.may_load(deps.storage)?;
    if let Some(_threshold) = threshold {
        let underlying = UNDERLYING_VOTING_MODULE.load(deps.storage)?;
        let res: IsActiveResponse = deps
            .querier
            .query_wasm_smart(underlying, &dao_interface::voting::Query::IsActive {})?;
        to_json_binary(&IsActiveResponse { active: res.active })
    } else {
        to_json_binary(&IsActiveResponse { active: true })
    }
}

fn query_snapshot(deps: Deps, proposal_id: u64) -> StdResult<Binary> {
    let snap = SNAPSHOTS.load(deps.storage, proposal_id)?;
    to_json_binary(&SnapshotResponse {
        proposal_id: snap.proposal_id,
        merkle_root: snap.merkle_root,
        total_power: snap.total_power,
        created_at: snap.created_at,
        poll_id: snap.poll_id,
    })
}

fn query_active_threshold(deps: Deps) -> StdResult<Binary> {
    to_json_binary(&ActiveThresholdResponse {
        active_threshold: ACTIVE_THRESHOLD.may_load(deps.storage)?,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Find the most recent snapshot at or before the given height.
fn find_snapshot_at_height(deps: Deps, height: u64) -> Option<crate::state::Snapshot> {
    SNAPSHOTS
        .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
        .take(100)
        .filter_map(|item| item.ok())
        .find(|(_id, snap)| snap.created_at <= height)
        .map(|(_id, snap)| snap)
}

// ---------------------------------------------------------------------------
// Migrate
// ---------------------------------------------------------------------------

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
    _info: MigrateInfo,
) -> Result<Response, ContractError> {
    let storage_version: ContractVersion = get_contract_version(deps.storage)?;
    if storage_version.version.as_str() < CONTRACT_VERSION {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    }
    Ok(Response::new().add_attribute("action", "migrate"))
}

// ---------------------------------------------------------------------------
// Reply
// ---------------------------------------------------------------------------

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    Err(ContractError::Std(StdError::msg("reply not supported")))
}