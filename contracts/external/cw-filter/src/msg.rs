use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{Addr, CosmosMsg};
pub use cw_ownable::Ownership;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use dao_interface::state::ModuleUpdate;

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the initial owner of the contract. Defaults to the
    /// sender.
    pub owner: Option<String>,
    /// The protobuf registry to use.
    pub protobuf_registry: Option<ModuleUpdate>,
}

#[cw_ownable_execute]
#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    /// Update the protobuf registry.
    UpdateProtobufRegistry {
        protobuf_registry: Option<ModuleUpdate>,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
pub enum QueryMsg {
    #[returns(dao_interface::proposal::InfoResponse)]
    Info {},
    #[returns(ProtobufRegistryResponse)]
    ProtobufRegistry {},
    #[returns(FilterResponse)]
    Filter {
        /// JSON-encoded filter expression.
        filter: String,
        msg: CosmosMsg,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

// Response types

#[cw_serde]
pub struct ProtobufRegistryResponse {
    /// The address of the protobuf registry, if set.
    pub protobuf_registry: Option<Addr>,
}

#[cw_serde]
pub enum FilterResponse {
    Pass {},
    Fail {
        /// The reason for the filter failing.
        reason: String,
    },
    Fatal {
        /// The fatal reason for the filter failing.
        reason: String,
    },
}
