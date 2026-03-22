#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Empty, Env, MessageInfo, MigrateInfo, Reply, Response, StdResult, to_json_binary
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::execute;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::query;
use crate::state::{DAO, EVENT_COUNT, EVENT_HOOKS};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-calendar";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const FAILED_EVENT_HOOK_REPLY_ID_BASE: u64 = 1_000_000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    DAO.save(deps.storage, &info.sender)?;
    EVENT_COUNT.save(deps.storage, &0u64)?;

    if let Some(groups) = msg.initial_groups {
        for group_init in groups {
            execute::save_group_from_init(deps.api, deps.storage, &info.sender, group_init)?;
        }
    }

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("dao", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<Empty>,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateEvent {
            title,
            description,
            start_time,
            end_time,
            managing_groups,
            services,
            extension,
        } => execute::execute_create_event(
            deps,
            env,
            info,
            title,
            description,
            start_time,
            end_time,
            managing_groups,
            services,
            extension,
        ),
        ExecuteMsg::UpdateEvent {
            event_id,
            title,
            description,
            start_time,
            end_time,
            managing_groups,
            services,
            extension,
        } => execute::execute_update_event(
            deps,
            env,
            info,
            event_id,
            title,
            description,
            start_time,
            end_time,
            managing_groups,
            services,
            extension,
        ),
        ExecuteMsg::CancelEvent { event_id } => {
            execute::execute_cancel_event(deps, env, info, event_id)
        }
        ExecuteMsg::RegisterEventGauges {
            event_id,
            begin_msgs,
            end_msgs,
        } => execute::execute_register_event_gauges(deps, info, event_id, begin_msgs, end_msgs),
        ExecuteMsg::TriggerEventStart { event_id } => {
            execute::execute_trigger_event_start(deps, env, event_id)
        }
        ExecuteMsg::TriggerEventEnd { event_id } => {
            execute::execute_trigger_event_end(deps, env, event_id)
        }
        ExecuteMsg::RegisterGroup { group } => execute::execute_register_group(deps, info, group),
        ExecuteMsg::UpdateGroup {
            group_id,
            suppliers,
        } => execute::execute_update_group(deps, info, group_id, suppliers),
        ExecuteMsg::RemoveGroup { group_id } => execute::execute_remove_group(deps, info, group_id),
        ExecuteMsg::RenewCalendar { limit } => execute::execute_renew_calendar(deps, env, limit),
        ExecuteMsg::AddEventHook { address } => {
            execute::execute_add_event_hook(deps, info, address)
        }
        ExecuteMsg::RemoveEventHook { address } => {
            execute::execute_remove_event_hook(deps, info, address)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Event { event_id } => to_json_binary(&query::query_event(deps, event_id)?),
        QueryMsg::ListEvents {
            filter,
            start_after,
            limit,
        } => to_json_binary(&query::query_list_events(deps, filter, start_after, limit)?),
        QueryMsg::ReverseEvents {
            filter,
            start_before,
            limit,
        } => to_json_binary(&query::query_reverse_events(
            deps,
            filter,
            start_before,
            limit,
        )?),
        QueryMsg::ListGroups {} => to_json_binary(&query::query_list_groups(deps)?),
        QueryMsg::Group { group_id } => to_json_binary(&query::query_group(deps, &group_id)?),
        QueryMsg::GroupsManagingEvent { event_id } => {
            to_json_binary(&query::query_groups_managing_event(deps, event_id)?)
        }
        QueryMsg::EventGauges { event_id } => {
            to_json_binary(&query::query_event_gauges(deps, event_id)?)
        }
        QueryMsg::EventCount {} => to_json_binary(&query::query_event_count(deps)?),
        QueryMsg::EventHooks {} => to_json_binary(&EVENT_HOOKS.query_hooks(deps)?),
        QueryMsg::DumpState {} => to_json_binary(&query::query_dump_state(deps)?),
        QueryMsg::Dao {} => to_json_binary(&DAO.load(deps.storage)?),
        QueryMsg::Info {} => to_json_binary(&query::query_info(deps)?),
        QueryMsg::NextProposalId {} => to_json_binary(&(EVENT_COUNT.load(deps.storage)? + 1)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    msg: MigrateMsg,
    _info: MigrateInfo,
) -> Result<Response, ContractError> {
    match msg {
        MigrateMsg::FromCompatible {} => {
            set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
            Ok(Response::new().add_attribute("method", "migrate"))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let id = msg.id;
    if id >= FAILED_EVENT_HOOK_REPLY_ID_BASE {
        let idx = id - FAILED_EVENT_HOOK_REPLY_ID_BASE;
        let addr = EVENT_HOOKS.remove_hook_by_index(deps.storage, idx)?;
        Ok(Response::new().add_attribute("removed_event_hook", format!("{addr}:{idx}")))
    } else {
        Err(ContractError::InvalidReplyID { id })
    }
}

pub fn failed_event_hook_id(idx: u64) -> u64 {
    FAILED_EVENT_HOOK_REPLY_ID_BASE + idx
}
