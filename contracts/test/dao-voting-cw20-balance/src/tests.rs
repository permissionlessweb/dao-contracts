use cosmwasm_std::{testing::MockApi, Addr, Uint128};
use cw2::ContractVersion;
use cw20::{Cw20Coin, MinterResponse, TokenInfoResponse};
use cw_multi_test::{App, Executor};
use dao_interface::voting::{InfoResponse, VotingPowerAtHeightResponse};
use dao_testing::contracts::{cw20_base_contract, dao_voting_cw20_balance_contract};

use crate::msg::{InstantiateMsg, QueryMsg};

const DAO_ADDR: &str = "dao";
const CREATOR_ADDR: &str = "creator";

fn instantiate_voting(app: &mut App, voting_id: u64, msg: InstantiateMsg) -> Addr {
    app.instantiate_contract(
        voting_id,
        MockApi::default().addr_make(DAO_ADDR),
        &msg,
        &[],
        "voting module",
        None,
    )
    .unwrap()
}

#[test]
#[should_panic(expected = "Initial governance token balances must not be empty")]
fn test_instantiate_zero_supply() {
    let mut app = App::default();
    let cw20_id = app.store_code(cw20_base_contract());
    let voting_id = app.store_code(dao_voting_cw20_balance_contract());
    instantiate_voting(
        &mut app,
        voting_id,
        InstantiateMsg {
            token_info: crate::msg::TokenInfo::New {
                code_id: cw20_id,
                label: "DAO DAO voting".to_string(),
                name: "DAO DAO".to_string(),
                symbol: "DAO".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(CREATOR_ADDR).to_string(),
                    amount: Uint128::zero().into(),
                }],
                marketing: None,
                salt: None,
            },
        },
    );
}

#[test]
#[should_panic(expected = "Initial governance token balances must not be empty")]
fn test_instantiate_no_balances() {
    let mut app = App::default();
    let cw20_id = app.store_code(cw20_base_contract());
    let voting_id = app.store_code(dao_voting_cw20_balance_contract());
    instantiate_voting(
        &mut app,
        voting_id,
        InstantiateMsg {
            token_info: crate::msg::TokenInfo::New {
                code_id: cw20_id,
                label: "DAO DAO voting".to_string(),
                name: "DAO DAO".to_string(),
                symbol: "DAO".to_string(),
                decimals: 6,
                initial_balances: vec![],
                marketing: None,
                salt: None,
            },
        },
    );
}

#[test]
fn test_contract_info() {
    let mut app = App::default();
    let cw20_id = app.store_code(cw20_base_contract());
    let voting_id = app.store_code(dao_voting_cw20_balance_contract());

    let voting_addr = instantiate_voting(
        &mut app,
        voting_id,
        InstantiateMsg {
            token_info: crate::msg::TokenInfo::New {
                code_id: cw20_id,
                label: "DAO DAO voting".to_string(),
                name: "DAO DAO".to_string(),
                symbol: "DAO".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(CREATOR_ADDR).to_string(),
                    amount: Uint128::from(2u64).into(),
                }],
                marketing: None,
                salt: None,
            },
        },
    );

    let info: InfoResponse = app
        .wrap()
        .query_wasm_smart(voting_addr, &QueryMsg::Info {})
        .unwrap();
    assert_eq!(
        info,
        InfoResponse {
            info: ContractVersion {
                contract: "crates.io:cw20-balance-voting".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string()
            }
        }
    )
}

#[test]
fn test_new_cw20() {
    let mut app = App::default();
    let cw20_id = app.store_code(cw20_base_contract());
    let voting_id = app.store_code(dao_voting_cw20_balance_contract());

    let voting_addr = instantiate_voting(
        &mut app,
        voting_id,
        InstantiateMsg {
            token_info: crate::msg::TokenInfo::New {
                code_id: cw20_id,
                label: "DAO DAO voting".to_string(),
                name: "DAO DAO".to_string(),
                symbol: "DAO".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(CREATOR_ADDR).to_string(),
                    amount: Uint128::from(2u64).into(),
                }],
                marketing: None,
                salt: None,
            },
        },
    );

    let token_addr: Addr = app
        .wrap()
        .query_wasm_smart(voting_addr.clone(), &QueryMsg::TokenContract {})
        .unwrap();

    let token_info: TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(token_addr.clone(), &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();
    assert_eq!(
        token_info,
        TokenInfoResponse {
            name: "DAO DAO".to_string(),
            symbol: "DAO".to_string(),
            decimals: 6,
            total_supply: Uint128::from(2u64).into()
        }
    );

    let minter_info: Option<MinterResponse> = app
        .wrap()
        .query_wasm_smart(token_addr.clone(), &cw20::Cw20QueryMsg::Minter {})
        .unwrap();
    assert_eq!(
        minter_info,
        Some(MinterResponse {
            minter: MockApi::default().addr_make(DAO_ADDR).to_string(),
            cap: None,
        })
    );

    let creator_voting_power: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            voting_addr.clone(),
            &QueryMsg::VotingPowerAtHeight {
                address: MockApi::default().addr_make(CREATOR_ADDR).to_string(),
                height: None,
            },
        )
        .unwrap();

    assert_eq!(
        creator_voting_power,
        VotingPowerAtHeightResponse {
            power: Uint128::from(2u64).into(),
            height: app.block_info().height,
        }
    );

    app.execute_contract(
        MockApi::default().addr_make(CREATOR_ADDR),
        token_addr,
        &cw20::Cw20ExecuteMsg::Transfer {
            recipient: MockApi::default().addr_make(DAO_ADDR).to_string(),
            amount: Uint128::from(1u64).into(),
        },
        &[],
    )
    .unwrap();

    let creator_voting_power: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            voting_addr.clone(),
            &QueryMsg::VotingPowerAtHeight {
                address: MockApi::default().addr_make(CREATOR_ADDR).to_string(),
                height: None,
            },
        )
        .unwrap();

    assert_eq!(
        creator_voting_power,
        VotingPowerAtHeightResponse {
            power: Uint128::from(1u64).into(),
            height: app.block_info().height,
        }
    );

    let dao_voting_power: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            voting_addr,
            &QueryMsg::VotingPowerAtHeight {
                address: MockApi::default().addr_make(DAO_ADDR).to_string(),
                height: None,
            },
        )
        .unwrap();

    assert_eq!(
        dao_voting_power,
        VotingPowerAtHeightResponse {
            power: Uint128::from(1u64).into(),
            height: app.block_info().height,
        }
    );
}

#[test]
fn test_existing_cw20() {
    let mut app = App::default();
    let cw20_id = app.store_code(cw20_base_contract());
    let voting_id = app.store_code(dao_voting_cw20_balance_contract());

    let token_addr = app
        .instantiate_contract(
            cw20_id,
            MockApi::default().addr_make(CREATOR_ADDR),
            &cw20_base::msg::InstantiateMsg {
                name: "DAO DAO".to_string(),
                symbol: "DAO".to_string(),
                decimals: 3,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(CREATOR_ADDR).to_string(),
                    amount: Uint128::from(2u64).into(),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "voting token",
            None,
        )
        .unwrap();

    let voting_addr = instantiate_voting(
        &mut app,
        voting_id,
        InstantiateMsg {
            token_info: crate::msg::TokenInfo::Existing {
                address: token_addr.to_string(),
            },
        },
    );

    let token_addr: Addr = app
        .wrap()
        .query_wasm_smart(voting_addr.clone(), &QueryMsg::TokenContract {})
        .unwrap();

    let token_info: TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(token_addr.clone(), &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();
    assert_eq!(
        token_info,
        TokenInfoResponse {
            name: "DAO DAO".to_string(),
            symbol: "DAO".to_string(),
            decimals: 3,
            total_supply: Uint128::from(2u64).into()
        }
    );

    let minter_info: Option<MinterResponse> = app
        .wrap()
        .query_wasm_smart(token_addr.clone(), &cw20::Cw20QueryMsg::Minter {})
        .unwrap();
    assert!(minter_info.is_none());

    let creator_voting_power: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            voting_addr.clone(),
            &QueryMsg::VotingPowerAtHeight {
                address: MockApi::default().addr_make(CREATOR_ADDR).to_string(),
                height: None,
            },
        )
        .unwrap();

    assert_eq!(
        creator_voting_power,
        VotingPowerAtHeightResponse {
            power: Uint128::from(2u64).into(),
            height: app.block_info().height,
        }
    );

    app.execute_contract(
        MockApi::default().addr_make(CREATOR_ADDR),
        token_addr,
        &cw20::Cw20ExecuteMsg::Transfer {
            recipient: MockApi::default().addr_make(DAO_ADDR).to_string(),
            amount: Uint128::from(1u64).into(),
        },
        &[],
    )
    .unwrap();

    let creator_voting_power: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            voting_addr.clone(),
            &QueryMsg::VotingPowerAtHeight {
                address: MockApi::default().addr_make(CREATOR_ADDR).to_string(),
                height: None,
            },
        )
        .unwrap();

    assert_eq!(
        creator_voting_power,
        VotingPowerAtHeightResponse {
            power: Uint128::from(1u64).into(),
            height: app.block_info().height,
        }
    );

    let dao_voting_power: VotingPowerAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            voting_addr,
            &QueryMsg::VotingPowerAtHeight {
                address: MockApi::default().addr_make(DAO_ADDR).to_string(),
                height: None,
            },
        )
        .unwrap();

    assert_eq!(
        dao_voting_power,
        VotingPowerAtHeightResponse {
            power: Uint128::from(1u64).into(),
            height: app.block_info().height,
        }
    );
}
