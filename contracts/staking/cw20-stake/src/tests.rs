use anyhow::Result as AnyResult;
use cosmwasm_std::testing::{mock_dependencies, mock_env, message_info, MockApi};
use cosmwasm_std::{to_json_binary, Addr, MessageInfo, Uint128};
use cw20::Cw20Coin;
use cw_controllers::{Claim, ClaimsResponse};
use cw_multi_test::{next_block, App, AppResponse, Executor};
use cw_ownable::{Action, Ownership, OwnershipError};
use cw_utils::Duration;
use cw_utils::Expiration::AtHeight;
use dao_testing::contracts::{cw20_base_contract, cw20_stake_contract};
use dao_voting::duration::UnstakingDurationError;
use std::borrow::BorrowMut;

use crate::msg::{
    ExecuteMsg, ListStakersResponse, QueryMsg, ReceiveMsg, StakedBalanceAtHeightResponse,
    StakedValueResponse, StakerBalanceResponse, TotalStakedAtHeightResponse, TotalValueResponse,
};
use crate::state::{Config, MAX_CLAIMS};
use cw20_stake::ContractError;

// v1 migration not supported in this version
// use cw20_stake_v1 as v1;

const ADDR1: &str = "addr1";
const ADDR2: &str = "addr2";
const ADDR3: &str = "addr3";
const ADDR4: &str = "addr4";
const OWNER: &str = "owner";

struct TestAccounts {
    addr1: Addr,
    addr2: Addr,
    addr3: Addr,
    addr4: Addr,
    owner: Addr,
}

fn test_accounts() -> TestAccounts {
    let api = MockApi::default();
    TestAccounts {
        addr1: api.addr_make(ADDR1),
        addr2: api.addr_make(ADDR2),
        addr3: api.addr_make(ADDR3),
        addr4: api.addr_make(ADDR4),
        owner: api.addr_make(OWNER),
    }
}

fn mock_app() -> App {
    App::default()
}

fn get_balance<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Uint128 {
    let msg = cw20::Cw20QueryMsg::Balance {
        address: address.into(),
    };
    let result: cw20::BalanceResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.balance
}

fn instantiate_cw20(
    app: &mut App,
    initial_balances: Vec<Cw20Coin>,
    accounts: &TestAccounts,
) -> Addr {
    let cw20_id = app.store_code(cw20_base_contract());
    let msg = cw20_base::msg::InstantiateMsg {
        name: String::from("Test"),
        symbol: String::from("TEST"),
        decimals: 6,
        initial_balances,
        mint: None,
        marketing: None,
    };

    app.instantiate_contract(cw20_id, accounts.addr1.clone(), &msg, &[], "cw20", None)
        .unwrap()
}

fn instantiate_staking(
    app: &mut App,
    cw20: Addr,
    unstaking_duration: Option<Duration>,
    accounts: &TestAccounts,
) -> Addr {
    let staking_code_id = app.store_code(cw20_stake_contract());
    let msg = crate::msg::InstantiateMsg {
        owner: Some(accounts.owner.to_string()),
        token_address: cw20.to_string(),
        unstaking_duration,
    };
    app.instantiate_contract(
        staking_code_id,
        accounts.addr1.clone(),
        &msg,
        &[],
        "staking",
        Some("admin".to_string()),
    )
    .unwrap()
}

fn setup_test_case(
    app: &mut App,
    initial_balances: Vec<Cw20Coin>,
    unstaking_duration: Option<Duration>,
    accounts: &TestAccounts,
) -> (Addr, Addr) {
    let cw20_addr = instantiate_cw20(app, initial_balances, accounts);
    app.update_block(next_block);
    let staking_addr = instantiate_staking(app, cw20_addr.clone(), unstaking_duration, accounts);
    app.update_block(next_block);
    (staking_addr, cw20_addr)
}

fn query_staked_balance<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Uint128 {
    let msg = QueryMsg::StakedBalanceAtHeight {
        address: address.into(),
        height: None,
    };
    let result: StakedBalanceAtHeightResponse =
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.balance
}

fn query_config<T: Into<String>>(app: &App, contract_addr: T) -> Config {
    let msg = QueryMsg::GetConfig {};
    app.wrap().query_wasm_smart(contract_addr, &msg).unwrap()
}

fn query_owner<T: Into<String>>(app: &App, contract: T) -> Ownership<Addr> {
    app.wrap()
        .query_wasm_smart(contract, &QueryMsg::Ownership {})
        .unwrap()
}

fn query_total_staked<T: Into<String>>(app: &App, contract_addr: T) -> Uint128 {
    let msg = QueryMsg::TotalStakedAtHeight { height: None };
    let result: TotalStakedAtHeightResponse =
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.total
}

fn query_staked_value<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Uint128 {
    let msg = QueryMsg::StakedValue {
        address: address.into(),
    };
    let result: StakedValueResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.value
}

fn query_total_value<T: Into<String>>(app: &App, contract_addr: T) -> Uint128 {
    let msg = QueryMsg::TotalValue {};
    let result: TotalValueResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.total
}

fn query_claims<T: Into<String>, U: Into<String>>(
    app: &App,
    contract_addr: T,
    address: U,
) -> Vec<Claim> {
    let msg = QueryMsg::Claims {
        address: address.into(),
    };
    let result: ClaimsResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
    result.claims
}

fn stake_tokens(
    app: &mut App,
    staking_addr: &Addr,
    cw20_addr: &Addr,
    info: MessageInfo,
    amount: Uint128,
) -> AnyResult<AppResponse> {
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_addr.to_string(),
        amount,
        msg: to_json_binary(&ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(info.sender, cw20_addr.clone(), &msg, &[])
}

fn update_config(
    app: &mut App,
    staking_addr: &Addr,
    info: MessageInfo,
    duration: Option<Duration>,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::UpdateConfig { duration };
    app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
}

fn unstake_tokens(
    app: &mut App,
    staking_addr: &Addr,
    info: MessageInfo,
    amount: Uint128,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::Unstake { amount };
    app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
}

fn claim_tokens(app: &mut App, staking_addr: &Addr, info: MessageInfo) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::Claim {};
    app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
}

#[test]
#[should_panic(expected = "Invalid unstaking duration, unstaking duration cannot be 0")]
fn test_instantiate_invalid_unstaking_duration() {
    let mut app = mock_app();
    let accts = test_accounts();
    let amount1 = Uint128::from(100u128);
    let initial_balances = vec![Cw20Coin {
        address: accts.addr1.to_string(),
        amount: amount1,
    }];
    let (_staking_addr, _cw20_addr) =
        setup_test_case(&mut app, initial_balances, Some(Duration::Height(0)), &accts);
}

#[test]
#[should_panic(expected = "Provided cw20 errored in response to TokenInfo query")]
fn test_instantiate_with_non_cw20_token() {
    let app = &mut mock_app();
    let accts = test_accounts();
    instantiate_staking(app, MockApi::default().addr_make("ekez"), None, &accts);
}

#[test]
fn test_update_config() {
    let mut app = mock_app();
    let accts = test_accounts();
    let amount1 = Uint128::from(100u128);
    let initial_balances = vec![Cw20Coin {
        address: accts.addr1.to_string(),
        amount: amount1,
    }];
    let (staking_addr, _cw20_addr) = setup_test_case(&mut app, initial_balances, None, &accts);

    // Owner can update configuration.
    let info = message_info(&accts.owner, &[]);
    update_config(&mut app, &staking_addr, info, Some(Duration::Height(1234))).unwrap();
    let config = query_config(&app, &staking_addr);
    assert_eq!(config.unstaking_duration, Some(Duration::Height(1234)));

    // Non owner may not update configuration.
    let info = message_info(&accts.addr1, &[]);
    let err: ContractError = update_config(&mut app, &staking_addr, info, None)
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner));

    // Zero durations not allowed.
    let info = message_info(&accts.owner, &[]);
    let err: ContractError =
        update_config(&mut app, &staking_addr, info, Some(Duration::Height(0)))
            .unwrap_err()
            .downcast()
            .unwrap();
    assert_eq!(
        err,
        ContractError::UnstakingDurationError(UnstakingDurationError::InvalidUnstakingDuration {})
    );

    let info = message_info(&accts.owner, &[]);
    let err: ContractError = update_config(&mut app, &staking_addr, info, Some(Duration::Time(0)))
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::UnstakingDurationError(UnstakingDurationError::InvalidUnstakingDuration {})
    );
}

#[test]
fn test_staking() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let accts = test_accounts();
    let amount1 = Uint128::from(100u128);
    let initial_balances = vec![Cw20Coin {
        address: accts.addr1.to_string(),
        amount: amount1,
    }];
    let (staking_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, None, &accts);

    let info = message_info(&accts.addr1, &[]);
    let _env = mock_env();

    // Successful bond
    let amount = Uint128::new(50);
    stake_tokens(&mut app, &staking_addr, &cw20_addr, info.clone(), amount).unwrap();

    // Very important that this balance is not reflected until
    // the next block. This protects us from flash loan hostile
    // takeovers.
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.to_string()),
        Uint128::zero()
    );

    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.to_string()),
        Uint128::from(50u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(50u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.to_string()),
        Uint128::from(50u128)
    );

    // Can't transfer bonded amount
    let msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: accts.addr2.to_string(),
        amount: Uint128::from(51u128),
    };
    let _err = app
        .borrow_mut()
        .execute_contract(info.sender.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap_err();

    // Successful transfer of unbonded amount
    let msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: accts.addr2.to_string(),
        amount: Uint128::from(20u128),
    };
    let _res = app
        .borrow_mut()
        .execute_contract(info.sender, cw20_addr.clone(), &msg, &[])
        .unwrap();

    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.clone()),
        Uint128::from(30u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr2.clone()),
        Uint128::from(20u128)
    );

    // Addr 2 successful bond
    let info = message_info(&accts.addr2, &[]);
    stake_tokens(&mut app, &staking_addr, &cw20_addr, info, Uint128::new(20)).unwrap();

    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr2.clone()),
        Uint128::from(20u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(70u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr2.clone()),
        Uint128::zero()
    );

    // Can't unstake more than you have staked
    let info = message_info(&accts.addr2, &[]);
    let _err = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(100)).unwrap_err();

    // Successful unstake
    let info = message_info(&accts.addr2, &[]);
    let _res = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(10)).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr2.clone()),
        Uint128::from(10u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(60u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr2.clone()),
        Uint128::from(10u128)
    );

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.clone()),
        Uint128::from(50u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.clone()),
        Uint128::from(30u128)
    );
}

#[test]
fn text_max_claims() {
    let mut app = mock_app();
    let accts = test_accounts();
    let amount1 = Uint128::from(MAX_CLAIMS + 1);
    let unstaking_blocks = 1u64;
    let initial_balances = vec![Cw20Coin {
        address: accts.addr1.to_string(),
        amount: amount1,
    }];
    let (staking_addr, cw20_addr) = setup_test_case(
        &mut app,
        initial_balances,
        Some(Duration::Height(unstaking_blocks)),
        &accts,
    );

    let info = message_info(&accts.addr1, &[]);
    stake_tokens(&mut app, &staking_addr, &cw20_addr, info.clone(), amount1).unwrap();

    // Create the max number of claims
    for _ in 0..MAX_CLAIMS {
        unstake_tokens(&mut app, &staking_addr, info.clone(), Uint128::new(1)).unwrap();
    }

    // Additional unstaking attempts ought to fail.
    unstake_tokens(&mut app, &staking_addr, info.clone(), Uint128::new(1)).unwrap_err();

    // Clear out the claims list.
    app.update_block(next_block);
    claim_tokens(&mut app, &staking_addr, info.clone()).unwrap();

    // Unstaking now allowed again.
    unstake_tokens(&mut app, &staking_addr, info.clone(), Uint128::new(1)).unwrap();
    app.update_block(next_block);
    claim_tokens(&mut app, &staking_addr, info).unwrap();

    assert_eq!(get_balance(&app, &cw20_addr, accts.addr1.clone()), amount1);
}

#[test]
fn test_unstaking_with_claims() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let accts = test_accounts();
    let amount1 = Uint128::from(100u128);
    let unstaking_blocks = 10u64;
    let initial_balances = vec![Cw20Coin {
        address: accts.addr1.to_string(),
        amount: amount1,
    }];
    let (staking_addr, cw20_addr) = setup_test_case(
        &mut app,
        initial_balances,
        Some(Duration::Height(unstaking_blocks)),
        &accts,
    );

    let info = message_info(&accts.addr1, &[]);

    // Successful bond
    let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, Uint128::new(50)).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.clone()),
        Uint128::from(50u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(50u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.clone()),
        Uint128::from(50u128)
    );

    // Unstake
    let info = message_info(&accts.addr1, &[]);
    let _res = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(10)).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.clone()),
        Uint128::from(40u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(40u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.clone()),
        Uint128::from(50u128)
    );

    // Cannot claim when nothing is available
    let info = message_info(&accts.addr1, &[]);
    let _err: ContractError = claim_tokens(&mut app, &staking_addr, info)
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(_err, ContractError::NothingToClaim {});

    // Successful claim
    app.update_block(|b| b.height += unstaking_blocks);
    let info = message_info(&accts.addr1, &[]);
    let _res = claim_tokens(&mut app, &staking_addr, info).unwrap();
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.clone()),
        Uint128::from(40u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(40u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.clone()),
        Uint128::from(60u128)
    );

    // Unstake and claim multiple
    let info = message_info(&accts.addr1, &[]);
    let _res = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(5)).unwrap();
    app.update_block(next_block);

    let info = message_info(&accts.addr1, &[]);
    let _res = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(5)).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.clone()),
        Uint128::from(30u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(30u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.clone()),
        Uint128::from(60u128)
    );

    app.update_block(|b| b.height += unstaking_blocks);
    let info = message_info(&accts.addr1, &[]);
    let _res = claim_tokens(&mut app, &staking_addr, info).unwrap();
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.clone()),
        Uint128::from(30u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(30u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.clone()),
        Uint128::from(70u128)
    );
}

#[test]
fn multiple_address_staking() {
    let accts = test_accounts();
    let amount1 = Uint128::from(100u128);
    let initial_balances = vec![
        Cw20Coin {
            address: accts.addr1.to_string(),
            amount: amount1,
        },
        Cw20Coin {
            address: accts.addr2.to_string(),
            amount: amount1,
        },
        Cw20Coin {
            address: accts.addr3.to_string(),
            amount: amount1,
        },
        Cw20Coin {
            address: accts.addr4.to_string(),
            amount: amount1,
        },
    ];
    let mut app = mock_app();
    let unstaking_blocks = 10u64;
    let (staking_addr, cw20_addr) = setup_test_case(
        &mut app,
        initial_balances,
        Some(Duration::Height(unstaking_blocks)),
        &accts,
    );

    let info = message_info(&accts.addr1, &[]);
    let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount1).unwrap();
    app.update_block(next_block);

    let info = message_info(&accts.addr2, &[]);
    let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount1).unwrap();
    app.update_block(next_block);

    let info = message_info(&accts.addr3, &[]);
    let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount1).unwrap();
    app.update_block(next_block);

    let info = message_info(&accts.addr4, &[]);
    let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount1).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.clone()),
        amount1
    );
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr2.clone()),
        amount1
    );
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr3.clone()),
        amount1
    );
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr4.clone()),
        amount1
    );

    assert_eq!(
        query_total_staked(&app, &staking_addr),
        amount1.checked_mul(Uint128::new(4)).unwrap()
    );

    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.clone()),
        Uint128::zero()
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr2.clone()),
        Uint128::zero()
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr3.clone()),
        Uint128::zero()
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr4.clone()),
        Uint128::zero()
    );
}

#[test]
fn test_auto_compounding_staking() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let accts = test_accounts();
    let amount1 = Uint128::from(1000u128);
    let initial_balances = vec![Cw20Coin {
        address: accts.addr1.to_string(),
        amount: amount1,
    }];
    let (staking_addr, cw20_addr) = setup_test_case(&mut app, initial_balances, None, &accts);

    let info = message_info(&accts.addr1, &[]);
    let _env = mock_env();

    // Successful bond
    let amount = Uint128::new(100);
    stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount).unwrap();
    app.update_block(next_block);
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.to_string()),
        Uint128::from(100u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(100u128)
    );
    assert_eq!(
        query_staked_value(&app, &staking_addr, accts.addr1.to_string()),
        Uint128::from(100u128)
    );
    assert_eq!(query_total_value(&app, &staking_addr), Uint128::from(100u128));
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.to_string()),
        Uint128::from(900u128)
    );

    // Add compounding rewards
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_addr.to_string(),
        amount: Uint128::from(100u128),
        msg: to_json_binary(&ReceiveMsg::Fund {}).unwrap(),
    };
    let _res = app
        .borrow_mut()
        .execute_contract(accts.addr1.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap();
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.to_string()),
        Uint128::from(100u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(100u128)
    );
    assert_eq!(
        query_staked_value(&app, &staking_addr, accts.addr1.to_string()),
        Uint128::from(200u128)
    );
    assert_eq!(query_total_value(&app, &staking_addr), Uint128::from(200u128));
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.to_string()),
        Uint128::from(800u128)
    );

    // Successful transfer of unbonded amount
    let msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: accts.addr2.to_string(),
        amount: Uint128::from(100u128),
    };
    let _res = app
        .borrow_mut()
        .execute_contract(accts.addr1.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap();

    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.clone()),
        Uint128::from(700u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr2.clone()),
        Uint128::from(100u128)
    );

    // Addr 2 successful bond
    let info = message_info(&accts.addr2, &[]);
    stake_tokens(&mut app, &staking_addr, &cw20_addr, info, Uint128::new(100)).unwrap();

    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr2.clone()),
        Uint128::from(50u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(150u128)
    );
    assert_eq!(
        query_staked_value(&app, &staking_addr, accts.addr2.to_string()),
        Uint128::from(100u128)
    );
    assert_eq!(query_total_value(&app, &staking_addr), Uint128::from(300u128));
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr2.clone()),
        Uint128::zero()
    );

    // Can't unstake more than you have staked
    let info = message_info(&accts.addr2, &[]);
    let _err = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(51)).unwrap_err();

    // Add compounding rewards
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_addr.to_string(),
        amount: Uint128::from(90u128),
        msg: to_json_binary(&ReceiveMsg::Fund {}).unwrap(),
    };
    let _res = app
        .borrow_mut()
        .execute_contract(accts.addr1.clone(), cw20_addr.clone(), &msg, &[])
        .unwrap();

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.to_string()),
        Uint128::from(100u128)
    );
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr2.clone()),
        Uint128::from(50u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(150u128)
    );
    assert_eq!(
        query_staked_value(&app, &staking_addr, accts.addr1.to_string()),
        Uint128::from(260u128)
    );
    assert_eq!(
        query_staked_value(&app, &staking_addr, accts.addr2.to_string()),
        Uint128::from(130u128)
    );
    assert_eq!(query_total_value(&app, &staking_addr), Uint128::from(390u128));
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.to_string()),
        Uint128::from(610u128)
    );

    // Successful unstake
    let info = message_info(&accts.addr2, &[]);
    let _res = unstake_tokens(&mut app, &staking_addr, info, Uint128::new(25)).unwrap();
    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr2.clone()),
        Uint128::from(25u128)
    );
    assert_eq!(
        query_total_staked(&app, &staking_addr),
        Uint128::from(125u128)
    );
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr2.clone()),
        Uint128::from(65u128)
    );
}

#[test]
fn test_simple_unstaking_with_duration() {
    let _deps = mock_dependencies();

    let mut app = mock_app();
    let accts = test_accounts();
    let amount1 = Uint128::from(100u128);
    let initial_balances = vec![
        Cw20Coin {
            address: accts.addr1.to_string(),
            amount: amount1,
        },
        Cw20Coin {
            address: accts.addr2.to_string(),
            amount: amount1,
        },
    ];
    let (staking_addr, cw20_addr) =
        setup_test_case(&mut app, initial_balances, Some(Duration::Height(1)), &accts);

    // Bond Address 1
    let info = message_info(&accts.addr1, &[]);
    let _env = mock_env();
    let amount = Uint128::new(100);
    stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount).unwrap();

    // Bond Address 2
    let info = message_info(&accts.addr2, &[]);
    let _env = mock_env();
    let amount = Uint128::new(100);
    stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount).unwrap();
    app.update_block(next_block);
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.to_string()),
        Uint128::from(100u128)
    );
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.to_string()),
        Uint128::from(100u128)
    );

    // Unstake Addr1
    let info = message_info(&accts.addr1, &[]);
    let _env = mock_env();
    let amount = Uint128::new(100);
    unstake_tokens(&mut app, &staking_addr, info, amount).unwrap();

    // Unstake Addr2
    let info = message_info(&accts.addr2, &[]);
    let _env = mock_env();
    let amount = Uint128::new(100);
    unstake_tokens(&mut app, &staking_addr, info, amount).unwrap();

    app.update_block(next_block);

    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr1.to_string()),
        Uint128::from(0u128)
    );
    assert_eq!(
        query_staked_balance(&app, &staking_addr, accts.addr2.to_string()),
        Uint128::from(0u128)
    );

    // Claim
    assert_eq!(
        query_claims(&app, &staking_addr, accts.addr1.clone()),
        vec![Claim {
            amount: Uint128::new(100),
            release_at: AtHeight(12349)
        }]
    );
    assert_eq!(
        query_claims(&app, &staking_addr, accts.addr2.clone()),
        vec![Claim {
            amount: Uint128::new(100),
            release_at: AtHeight(12349)
        }]
    );

    let info = message_info(&accts.addr1, &[]);
    claim_tokens(&mut app, &staking_addr, info).unwrap();
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr1.clone()),
        Uint128::from(100u128)
    );

    let info = message_info(&accts.addr2, &[]);
    claim_tokens(&mut app, &staking_addr, info).unwrap();
    assert_eq!(
        get_balance(&app, &cw20_addr, accts.addr2.clone()),
        Uint128::from(100u128)
    );
}

#[test]
fn test_double_unstake_at_height() {
    let mut app = App::default();
    let accts = test_accounts();
    let ekez = MockApi::default().addr_make("ekez");

    let (staking_addr, cw20_addr) = setup_test_case(
        &mut app,
        vec![Cw20Coin {
            address: ekez.to_string(),
            amount: Uint128::new(10),
        }],
        None,
        &accts,
    );

    stake_tokens(
        &mut app,
        &staking_addr,
        &cw20_addr,
        message_info(&ekez, &[]),
        Uint128::new(10),
    )
    .unwrap();

    app.update_block(next_block);

    unstake_tokens(
        &mut app,
        &staking_addr,
        message_info(&ekez, &[]),
        Uint128::new(1),
    )
    .unwrap();

    unstake_tokens(
        &mut app,
        &staking_addr,
        message_info(&ekez, &[]),
        Uint128::new(9),
    )
    .unwrap();

    app.update_block(next_block);

    // Unstaked balances are not reflected until the following
    // block. Same behavior as staked balances. This is important
    // because otherwise weird things could happen like:
    //
    // 1. I create a proposal (and am allowed to because I have a
    //    staked balance)
    // 2. I unstake all my tokens in the same block.
    //
    // Now there is some strangeness as for part of the block I had a
    // staked balance and was allowed to take actions as if I did, and
    // part of it I did not.
    let balance: StakedBalanceAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            staking_addr.clone(),
            &QueryMsg::StakedBalanceAtHeight {
                address: ekez.to_string(),
                height: Some(app.block_info().height - 1),
            },
        )
        .unwrap();

    assert_eq!(balance.balance, Uint128::new(10));

    let balance: StakedBalanceAtHeightResponse = app
        .wrap()
        .query_wasm_smart(
            staking_addr,
            &QueryMsg::StakedBalanceAtHeight {
                address: ekez.to_string(),
                height: Some(app.block_info().height),
            },
        )
        .unwrap();

    assert_eq!(balance.balance, Uint128::zero())
}

#[test]
fn test_query_list_stakers() {
    let mut app = App::default();
    let accts = test_accounts();
    let api = MockApi::default();
    let s1 = api.addr_make("1");
    let s2 = api.addr_make("2");
    let s3 = api.addr_make("3");
    let s4 = api.addr_make("4");

    let (staking_addr, cw20_addr) = setup_test_case(
        &mut app,
        vec![
            Cw20Coin {
                address: s1.to_string(),
                amount: Uint128::new(10),
            },
            Cw20Coin {
                address: s2.to_string(),
                amount: Uint128::new(20),
            },
            Cw20Coin {
                address: s3.to_string(),
                amount: Uint128::new(30),
            },
            Cw20Coin {
                address: s4.to_string(),
                amount: Uint128::new(40),
            },
        ],
        None,
        &accts,
    );

    stake_tokens(
        &mut app,
        &staking_addr,
        &cw20_addr,
        message_info(&s1, &[]),
        Uint128::new(10),
    )
    .unwrap();

    app.update_block(next_block);

    stake_tokens(
        &mut app,
        &staking_addr,
        &cw20_addr,
        message_info(&s2, &[]),
        Uint128::new(20),
    )
    .unwrap();

    app.update_block(next_block);

    stake_tokens(
        &mut app,
        &staking_addr,
        &cw20_addr,
        message_info(&s3, &[]),
        Uint128::new(30),
    )
    .unwrap();

    app.update_block(next_block);

    stake_tokens(
        &mut app,
        &staking_addr,
        &cw20_addr,
        message_info(&s4, &[]),
        Uint128::new(40),
    )
    .unwrap();

    app.update_block(next_block);

    app.update_block(next_block);

    // check first 2
    let stakers: ListStakersResponse = app
        .wrap()
        .query_wasm_smart(
            staking_addr.clone(),
            &QueryMsg::ListStakers {
                start_after: None,
                limit: Some(2),
            },
        )
        .unwrap();

    println!("First page stakers: {:?}", stakers.stakers);

    assert_eq!(stakers.stakers.len(), 2);
    assert_eq!(stakers.stakers[1].balance, Uint128::new(10));
    assert_eq!(stakers.stakers[0].balance, Uint128::new(20));

    // skip first and grab 2
    let stakers: ListStakersResponse = app
        .wrap()
        .query_wasm_smart(
            staking_addr,
            &QueryMsg::ListStakers {
                start_after: Some(stakers.stakers[0].address.clone()),
                limit: Some(2),
            },
        )
        .unwrap();

    assert_eq!(stakers.stakers.len(), 2);
    assert_eq!(stakers.stakers[0].balance, Uint128::new(10));
    assert_eq!(stakers.stakers[1].balance, Uint128::new(30));
}

#[test]
fn test_ownership_transfer() {
    let mut app = App::default();
    let accts = test_accounts();
    let cw20_addr = instantiate_cw20(
        &mut app,
        vec![cw20::Cw20Coin {
            address: accts.owner.to_string(),
            amount: Uint128::from(1000u64),
        }],
        &accts,
    );
    let staking_addr = instantiate_staking(&mut app, cw20_addr, None, &accts);

    app.execute_contract(
        accts.owner.clone(),
        staking_addr.clone(),
        &ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
            new_owner: accts.addr1.to_string(),
            expiry: None,
        }),
        &[],
    )
    .unwrap();

    let ownership = query_owner(&app, &staking_addr);
    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(accts.owner.clone()),
            pending_owner: Some(accts.addr1.clone()),
            pending_expiry: None
        }
    );

    app.execute_contract(
        accts.addr1.clone(),
        staking_addr.clone(),
        &ExecuteMsg::UpdateOwnership(Action::AcceptOwnership),
        &[],
    )
    .unwrap();

    let ownership = query_owner(&app, &staking_addr);
    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(accts.addr1.clone()),
            pending_owner: None,
            pending_expiry: None
        }
    );
}

// v1 migration test disabled - not supported in this version
// #[test]
// fn test_migrate_from_v1() {
//     let mut app = App::default();
//     let cw20_addr = instantiate_cw20(
//         &mut app,
//         vec![cw20::Cw20Coin {
//             address: OWNER.to_string(),
//             amount: Uint128::from(1000u64),
//         }],
//     );
//
//     let v1_code = app.store_code(cw20_stake_v1_contract());
//     let v2_code = app.store_code(cw20_stake_contract());
//
//     let staking = app
//         .instantiate_contract(
//             v1_code,
//             Addr::unchecked(OWNER),
//             &v1::msg::InstantiateMsg {
//                 owner: Some(OWNER.to_string()),
//                 manager: Some(OWNER.to_string()),
//                 token_address: cw20_addr.to_string(),
//                 unstaking_duration: None,
//             },
//             &[],
//             "staking".to_string(),
//             Some(OWNER.to_string()),
//         )
//         .unwrap();
//
//     app.execute(
//         Addr::unchecked(OWNER),
//         WasmMsg::Migrate {
//             contract_addr: staking.to_string(),
//             new_code_id: v2_code,
//             msg: to_json_binary(&MigrateMsg::FromV1 {}).unwrap(),
//         }
//         .into(),
//     )
//     .unwrap();
//
//     // can not migrate more than once.
//     let err: ContractError = app
//         .execute(
//             Addr::unchecked(OWNER),
//             WasmMsg::Migrate {
//                 contract_addr: staking.to_string(),
//                 new_code_id: v2_code,
//                 msg: to_json_binary(&MigrateMsg::FromV1 {}).unwrap(),
//             }
//             .into(),
//         )
//         .unwrap_err()
//         .downcast()
//         .unwrap();
//     assert_eq!(err, ContractError::AlreadyMigrated {});
//
//     // owner is moved into cw_ownable.
//     let ownership = query_owner(&app, &staking);
//     assert_eq!(
//         ownership,
//         Ownership::<Addr> {
//             owner: Some(Addr::unchecked(OWNER)),
//             pending_owner: None,
//             pending_expiry: None
//         }
//     );
//
//     // config is loadable and has no manager, but is otherwise
//     // unchanged.
//     let config = query_config(&app, &staking);
//     assert_eq!(
//         config,
//         Config {
//             token_address: cw20_addr,
//             unstaking_duration: None,
//         }
//     );
// }
