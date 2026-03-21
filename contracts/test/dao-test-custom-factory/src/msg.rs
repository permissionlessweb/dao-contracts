use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use cw721::{msg::Cw721InstantiateMsg, EmptyOptionalCollectionExtension};

#[cfg(feature = "osmosis_tokenfactory")]
use dao_interface::token::NewTokenInfo;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
#[cfg_attr(feature = "interface", derive(cw_orch::ExecuteFns))]
pub enum ExecuteMsg {
    /// Example NFT factory implementation
    NftFactory {
        code_id: u64,
        cw721_instantiate_msg: Cw721InstantiateMsg<EmptyOptionalCollectionExtension>,
        initial_nfts: Vec<Binary>,
    },
    /// Example NFT factory implentation that execpts funds
    NftFactoryWithFunds {
        code_id: u64,
        cw721_instantiate_msg: Cw721InstantiateMsg<EmptyOptionalCollectionExtension>,
        initial_nfts: Vec<Binary>,
    },
    /// Used for testing no callback
    NftFactoryNoCallback {},
    /// Used for testing wrong callback
    NftFactoryWrongCallback {},
    /// Example Factory Implementation
    #[cfg(feature = "osmosis_tokenfactory")]
    TokenFactoryFactory(NewTokenInfo),
    /// Example Factory Implementation that accepts funds
    #[cfg(feature = "osmosis_tokenfactory")]
    TokenFactoryFactoryWithFunds(NewTokenInfo),
    /// Used for testing no callback
    #[cfg(feature = "osmosis_tokenfactory")]
    TokenFactoryFactoryNoCallback {},
    /// Used for testing wrong callback
    #[cfg(feature = "osmosis_tokenfactory")]
    TokenFactoryFactoryWrongCallback {},
    /// Validate NFT DAO
    ValidateNftDao {},
}

#[cw_serde]
#[derive(QueryResponses)]
#[cfg_attr(feature = "interface", derive(cw_orch::QueryFns))]
pub enum QueryMsg {
    #[returns(dao_interface::voting::InfoResponse)]
    Info {},
}
