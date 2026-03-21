use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::CosmosMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub root: String,
}

#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    Execute { msgs: Vec<CosmosMsg> },
}

#[cw_serde]
#[derive(QueryResponses)]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
pub enum QueryMsg {
    #[returns(cosmwasm_std::Addr)]
    Admin {},
    #[returns(cosmwasm_std::Addr)]
    Dao {},
    #[returns(dao_interface::voting::InfoResponse)]
    Info {},
}
