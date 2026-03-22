use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, Api, Binary, BlockInfo, Coin, CustomMsg, CustomQuery, Querier, StdError, StdResult,
    Storage, Uint64, from_json, to_json_binary,
};
use cw_multi_test::{AppResponse, BankSudo, CosmosRouter, Stargate, SudoMsg};
use prost::Message;
use serde::de::DeserializeOwned;

use crate::bitsong::{
    MsgIssue, MsgIssueResponse, MsgMint, MsgMintResponse, MsgSetAuthority, MsgSetMinter,
    MsgSetMinterResponse, MsgSetUri, MsgSetUriResponse,
};

const DENOMS_PREFIX: &str = "denoms";
const DENOMS_COUNT_KEY: &str = "denoms_count";

#[cw_serde]
struct FanToken {
    pub denom: String,
    pub name: String,
    pub symbol: String,
    pub max_supply: String,
    pub authority: String,
    pub minter: String,
    pub uri: String,
}

pub struct StargateKeeper {}

impl StargateKeeper {}

impl Stargate for StargateKeeper {
    fn execute_stargate<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        type_url: String,
        value: Binary,
    ) -> StdResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        if type_url == *"/bitsong.fantoken.MsgIssue" {
            let denoms_count: Uint64 = storage
                .get(DENOMS_COUNT_KEY.as_bytes())
                .map_or_else(Uint64::zero, |d| from_json(d).unwrap());
            let denom = format!("fantoken{}", denoms_count.u64() + 1);

            let msg: MsgIssue = Message::decode(value.as_slice()).unwrap();
            let ft = FanToken {
                denom: denom.clone(),
                name: msg.name,
                symbol: msg.symbol,
                max_supply: msg.max_supply,
                authority: msg.authority,
                minter: msg.minter,
                uri: msg.uri,
            };

            let key = format!("{}:{}", DENOMS_PREFIX, denom.clone());
            let serialized_ft = to_json_binary(&ft).expect("Failed to serialize FanToken");
            storage.set(key.as_bytes(), &serialized_ft);

            return Ok(AppResponse {
                events: vec![],
                data: Some(Binary::from(MsgIssueResponse { denom })),
            });
        }
        if type_url == *"/bitsong.fantoken.MsgMint" {
            let msg: MsgMint = Message::decode(value.as_slice()).unwrap();

            let coin = msg.coin.unwrap();

            let key = format!("{}:{}", DENOMS_PREFIX, coin.denom.clone());
            let serialized_ft = storage.get(key.as_bytes());
            let fantoken: FanToken =
                from_json(serialized_ft.unwrap()).expect("Failed to deserialize FanToken");

            if sender.as_str() != fantoken.minter || msg.minter != fantoken.minter {
                return Err(StdError::msg("Minter unauthorized"));
            }

            router.sudo(
                api,
                storage,
                block,
                SudoMsg::Bank(BankSudo::Mint {
                    to_address: msg.recipient.clone(),
                    amount: vec![Coin::new(
                        coin.amount.parse::<u128>().unwrap(),
                        coin.denom.clone(),
                    )],
                }),
            ).map_err(|e| StdError::msg(e.to_string()))?;

            return Ok(AppResponse {
                events: vec![],
                data: Some(Binary::from(MsgMintResponse {})),
            });
        }
        if type_url == *"/bitsong.fantoken.MsgSetMinter" {
            let msg: MsgSetMinter = Message::decode(value.as_slice()).unwrap();

            let key = format!("{}:{}", DENOMS_PREFIX, msg.denom.clone());
            let serialized_ft = storage.get(key.as_bytes());
            let mut fantoken: FanToken =
                from_json(serialized_ft.unwrap()).expect("Failed to deserialize FanToken");

            if sender.as_str() != fantoken.minter {
                return Err(StdError::msg("Unauthorized"));
            }

            if msg.old_minter != fantoken.minter {
                return Err(StdError::msg("Old minter does not match"));
            }

            fantoken.minter = msg.new_minter;
            storage.set(key.as_bytes(), &to_json_binary(&fantoken).unwrap());

            return Ok(AppResponse {
                events: vec![],
                data: Some(Binary::from(MsgSetMinterResponse {})),
            });
        }
        if type_url == *"/bitsong.fantoken.MsgSetAuthority" {
            let msg: MsgSetAuthority = Message::decode(value.as_slice()).unwrap();

            let key = format!("{}:{}", DENOMS_PREFIX, msg.denom.clone());
            let serialized_ft = storage.get(key.as_bytes());
            let mut fantoken: FanToken =
                from_json(serialized_ft.unwrap()).expect("Failed to deserialize FanToken");

            if sender.as_str() != fantoken.authority {
                return Err(StdError::msg("Unauthorized"));
            }

            if msg.old_authority != fantoken.authority {
                return Err(StdError::msg("Old authority does not match"));
            }

            fantoken.authority = msg.new_authority;
            storage.set(key.as_bytes(), &to_json_binary(&fantoken).unwrap());

            return Ok(AppResponse {
                events: vec![],
                data: Some(Binary::from(MsgSetMinterResponse {})),
            });
        }
        if type_url == *"/bitsong.fantoken.MsgSetUri" {
            let msg: MsgSetUri = Message::decode(value.as_slice()).unwrap();

            let key = format!("{}:{}", DENOMS_PREFIX, msg.denom.clone());
            let serialized_ft = storage.get(key.as_bytes());
            let mut fantoken: FanToken =
                from_json(serialized_ft.unwrap()).expect("Failed to deserialize FanToken");

            if sender.as_str() != fantoken.authority || msg.authority != fantoken.authority {
                return Err(StdError::msg("Authority unauthorized"));
            }

            fantoken.uri = msg.uri;
            storage.set(key.as_bytes(), &to_json_binary(&fantoken).unwrap());

            return Ok(AppResponse {
                events: vec![],
                data: Some(Binary::from(MsgSetUriResponse {})),
            });
        }
        Ok(AppResponse::default())
    }

    fn query_stargate(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _path: String,
        data: Binary,
    ) -> StdResult<Binary> {
        Ok(data)
    }
}
