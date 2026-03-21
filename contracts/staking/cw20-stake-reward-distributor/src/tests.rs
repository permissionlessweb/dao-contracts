use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{next_block, App, Executor};
use cw_ownable::{Action, Expiration, Ownership, OwnershipError};
use dao_testing::contracts::{
    cw20_base_contract, cw20_stake_contract, cw20_stake_reward_distributor_contract,
};

use crate::{
    msg::{ExecuteMsg, InfoResponse, InstantiateMsg, QueryMsg},
    state::Config,
};
use cw20_stake_reward_distributor::ContractError;

const OWNER: &str = "owner";
const OWNER2: &str = "owner2";

fn instantiate_cw20(app: &mut App, initial_balances: Vec<Cw20Coin>) -> Addr {
    let cw20_id = app.store_code(cw20_base_contract());
    let msg = cw20_base::msg::InstantiateMsg {
        name: String::from("Test"),
        symbol: String::from("TEST"),
        decimals: 6,
        initial_balances,
        mint: None,
        marketing: None,
    };

    app.instantiate_contract(cw20_id, MockApi::default().addr_make(OWNER), &msg, &[], "cw20", None)
        .unwrap()
}

fn instantiate_staking(app: &mut App, cw20_addr: Addr) -> Addr {
    let staking_id = app.store_code(cw20_stake_contract());
    let msg = cw20_stake::msg::InstantiateMsg {
        owner: Some(MockApi::default().addr_make(OWNER).to_string()),
        token_address: cw20_addr.to_string(),
        unstaking_duration: None,
    };
    app.instantiate_contract(
        staking_id,
        MockApi::default().addr_make(OWNER),
        &msg,
        &[],
        "staking",
        None,
    )
    .unwrap()
}

fn instantiate_distributor(app: &mut App, msg: InstantiateMsg) -> Addr {
    let code_id = app.store_code(cw20_stake_reward_distributor_contract());
    app.instantiate_contract(
        code_id,
        MockApi::default().addr_make(OWNER),
        &msg,
        &[],
        "distributor",
        None,
    )
    .unwrap()
}

fn get_balance_cw20<T: Into<String>, U: Into<String>>(
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

fn get_info<T: Into<String>>(app: &App, distributor_addr: T) -> InfoResponse {
    let result: InfoResponse = app
        .wrap()
        .query_wasm_smart(distributor_addr, &QueryMsg::Info {})
        .unwrap();
    result
}

fn get_owner(app: &App, contract: &Addr) -> Ownership<Addr> {
    app.wrap()
        .query_wasm_smart(contract, &QueryMsg::Ownership {})
        .unwrap()
}

#[test]
fn test_instantiate() {
    let mut app = App::default();

    let cw20_addr = instantiate_cw20(&mut app, vec![]);
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let msg = InstantiateMsg {
        owner: MockApi::default().addr_make(OWNER).to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };

    let distributor_addr = instantiate_distributor(&mut app, msg);
    let response: InfoResponse = app
        .wrap()
        .query_wasm_smart(&distributor_addr, &QueryMsg::Info {})
        .unwrap();

    assert_eq!(
        response.config,
        Config {
            staking_addr,
            reward_rate: Uint128::new(1),
            reward_token: cw20_addr,
        }
    );
    assert_eq!(response.last_payment_block, app.block_info().height);

    let ownership = get_owner(&app, &distributor_addr);
    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(MockApi::default().addr_make(OWNER)),
            pending_owner: None,
            pending_expiry: None
        }
    );
}

#[test]
fn test_update_config() {
    let mut app = App::default();

    let cw20_addr = instantiate_cw20(&mut app, vec![]);
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let msg = InstantiateMsg {
        owner: MockApi::default().addr_make(OWNER).to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    let msg = ExecuteMsg::UpdateConfig {
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(5),
        reward_token: cw20_addr.to_string(),
    };

    app.execute_contract(MockApi::default().addr_make(OWNER), distributor_addr.clone(), &msg, &[])
        .unwrap();

    let response: InfoResponse = app
        .wrap()
        .query_wasm_smart(&distributor_addr, &QueryMsg::Info {})
        .unwrap();

    assert_eq!(
        response.config,
        Config {
            staking_addr: staking_addr.clone(),
            reward_rate: Uint128::new(5),
            reward_token: cw20_addr.clone(),
        }
    );

    let msg = ExecuteMsg::UpdateConfig {
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(7),
        reward_token: cw20_addr.to_string(),
    };

    // non-owner may not update config.
    let err: ContractError = app
        .execute_contract(MockApi::default().addr_make("notowner"), distributor_addr, &msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::Ownership(OwnershipError::NotOwner));
}

#[test]
fn test_distribute() {
    let mut app = App::default();

    let cw20_addr = instantiate_cw20(
        &mut app,
        vec![cw20::Cw20Coin {
            address: MockApi::default().addr_make(OWNER).to_string(),
            amount: Uint128::from(1000u64),
        }],
    );
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let msg = InstantiateMsg {
        owner: MockApi::default().addr_make(OWNER).to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    let msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: distributor_addr.to_string(),
        amount: Uint128::from(1000u128),
    };
    app.execute_contract(MockApi::default().addr_make(OWNER), cw20_addr.clone(), &msg, &[])
        .unwrap();

    app.update_block(|block| block.height += 10);
    app.execute_contract(
        MockApi::default().addr_make(OWNER),
        distributor_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    let staking_balance = get_balance_cw20(&app, cw20_addr.clone(), staking_addr.clone());
    assert_eq!(staking_balance, Uint128::new(10));

    let distributor_info = get_info(&app, distributor_addr.clone());
    assert_eq!(distributor_info.balance, Uint128::new(990));
    assert_eq!(distributor_info.last_payment_block, app.block_info().height);

    app.update_block(|block| block.height += 500);
    app.execute_contract(
        MockApi::default().addr_make(OWNER),
        distributor_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    let staking_balance = get_balance_cw20(&app, cw20_addr.clone(), staking_addr.clone());
    assert_eq!(staking_balance, Uint128::new(510));

    let distributor_info = get_info(&app, distributor_addr.clone());
    assert_eq!(distributor_info.balance, Uint128::new(490));
    assert_eq!(distributor_info.last_payment_block, app.block_info().height);

    app.update_block(|block| block.height += 1000);
    app.execute_contract(
        MockApi::default().addr_make(OWNER),
        distributor_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    let staking_balance = get_balance_cw20(&app, cw20_addr.clone(), staking_addr.clone());
    assert_eq!(staking_balance, Uint128::new(1000));

    let distributor_info = get_info(&app, distributor_addr.clone());
    assert_eq!(distributor_info.balance, Uint128::new(0));
    assert_eq!(distributor_info.last_payment_block, app.block_info().height);
    let last_payment_block = distributor_info.last_payment_block;

    // Pays out nothing
    app.update_block(|block| block.height += 1100);
    let err: ContractError = app
        .execute_contract(
            MockApi::default().addr_make(OWNER),
            distributor_addr.clone(),
            &ExecuteMsg::Distribute {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::ZeroRewards {}));

    let staking_balance = get_balance_cw20(&app, cw20_addr, staking_addr);
    assert_eq!(staking_balance, Uint128::new(1000));

    let distributor_info = get_info(&app, distributor_addr.clone());
    assert_eq!(distributor_info.balance, Uint128::new(0));
    assert_eq!(distributor_info.last_payment_block, last_payment_block);

    // go to a block before the last payment
    app.update_block(|block| block.height -= 2000);
    let err: ContractError = app
        .execute_contract(
            MockApi::default().addr_make(OWNER),
            distributor_addr,
            &ExecuteMsg::Distribute {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::RewardsDistributedForBlock {}));
}

#[test]
fn test_instantiate_invalid_addrs() {
    let mut app = App::default();
    let cw20_addr = instantiate_cw20(
        &mut app,
        vec![cw20::Cw20Coin {
            address: MockApi::default().addr_make(OWNER).to_string(),
            amount: Uint128::from(1000u64),
        }],
    );
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let api = MockApi::default();
    let msg = InstantiateMsg {
        owner: api.addr_make(OWNER).to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: api.addr_make("not_a_cw20").to_string(),
    };

    let code_id = app.store_code(cw20_stake_reward_distributor_contract());
    let err: ContractError = app
        .instantiate_contract(
            code_id,
            api.addr_make(OWNER),
            &msg,
            &[],
            "distributor",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::InvalidCw20 {});

    let msg = InstantiateMsg {
        owner: api.addr_make(OWNER).to_string(),
        staking_addr: api.addr_make("not_a_staking").to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    let err: ContractError = app
        .instantiate_contract(
            code_id,
            MockApi::default().addr_make(OWNER),
            &msg,
            &[],
            "distributor",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::InvalidStakingContract {});
}

#[test]
fn test_update_config_invalid_addrs() {
    let mut app = App::default();
    let api = MockApi::default();

    let cw20_addr = instantiate_cw20(&mut app, vec![]);
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let msg = InstantiateMsg {
        owner: api.addr_make(OWNER).to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    let msg = ExecuteMsg::UpdateConfig {
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(5),
        reward_token: api.addr_make("not_a_cw20").to_string(),
    };

    let err: ContractError = app
        .execute_contract(api.addr_make(OWNER), distributor_addr.clone(), &msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::InvalidCw20 {});

    let msg = ExecuteMsg::UpdateConfig {
        staking_addr: api.addr_make("not_a_staking").to_string(),
        reward_rate: Uint128::new(5),
        reward_token: staking_addr.to_string(),
    };

    let err: ContractError = app
        .execute_contract(api.addr_make(OWNER), distributor_addr, &msg, &[])
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::InvalidStakingContract {});
}

#[test]
fn test_withdraw() {
    let mut app = App::default();

    let cw20_addr = instantiate_cw20(
        &mut app,
        vec![cw20::Cw20Coin {
            address: MockApi::default().addr_make(OWNER).to_string(),
            amount: Uint128::from(1000u64),
        }],
    );
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let msg = InstantiateMsg {
        owner: MockApi::default().addr_make(OWNER).to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    let msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: distributor_addr.to_string(),
        amount: Uint128::from(1000u128),
    };
    app.execute_contract(MockApi::default().addr_make(OWNER), cw20_addr.clone(), &msg, &[])
        .unwrap();

    app.update_block(|block| block.height += 10);
    app.execute_contract(
        MockApi::default().addr_make(OWNER),
        distributor_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    let staking_balance = get_balance_cw20(&app, cw20_addr.clone(), staking_addr);
    assert_eq!(staking_balance, Uint128::new(10));

    let distributor_info = get_info(&app, distributor_addr.clone());
    assert_eq!(distributor_info.balance, Uint128::new(990));
    assert_eq!(distributor_info.last_payment_block, app.block_info().height);

    // Unauthorized user cannot withdraw funds
    let err = app
        .execute_contract(
            MockApi::default().addr_make("notowner"),
            distributor_addr.clone(),
            &ExecuteMsg::Withdraw {},
            &[],
        )
        .unwrap_err();

    assert_eq!(
        ContractError::Ownership(OwnershipError::NotOwner),
        err.downcast().unwrap()
    );

    // Withdraw funds
    app.execute_contract(
        MockApi::default().addr_make(OWNER),
        distributor_addr,
        &ExecuteMsg::Withdraw {},
        &[],
    )
    .unwrap();

    let owner_balance = get_balance_cw20(&app, cw20_addr, MockApi::default().addr_make(OWNER));
    assert_eq!(owner_balance, Uint128::new(990));
}

#[test]
fn test_dao_deploy() {
    // DAOs will deploy this contract with following steps
    // Contract is instantiated by any address with 0 reward rate
    // Dao updates reward rate and funds in same transaction
    let mut app = App::default();

    let cw20_addr = instantiate_cw20(
        &mut app,
        vec![cw20::Cw20Coin {
            address: MockApi::default().addr_make(OWNER).to_string(),
            amount: Uint128::from(1000u64),
        }],
    );
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());

    let msg = InstantiateMsg {
        owner: MockApi::default().addr_make(OWNER).to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(0),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    let msg = ExecuteMsg::UpdateConfig {
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(1),
        reward_token: cw20_addr.to_string(),
    };
    app.execute_contract(MockApi::default().addr_make(OWNER), distributor_addr.clone(), &msg, &[])
        .unwrap();

    let msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: distributor_addr.to_string(),
        amount: Uint128::from(1000u128),
    };
    app.execute_contract(MockApi::default().addr_make(OWNER), cw20_addr.clone(), &msg, &[])
        .unwrap();

    app.update_block(|block| block.height += 10);
    app.execute_contract(
        MockApi::default().addr_make(OWNER),
        distributor_addr.clone(),
        &ExecuteMsg::Distribute {},
        &[],
    )
    .unwrap();

    let staking_balance = get_balance_cw20(&app, cw20_addr, staking_addr);
    assert_eq!(staking_balance, Uint128::new(10));

    let distributor_info = get_info(&app, distributor_addr);
    assert_eq!(distributor_info.balance, Uint128::new(990));
    assert_eq!(distributor_info.last_payment_block, app.block_info().height);
}

#[test]
fn test_ownership() {
    let mut app = App::default();
    let cw20_addr = instantiate_cw20(
        &mut app,
        vec![cw20::Cw20Coin {
            address: MockApi::default().addr_make(OWNER).to_string(),
            amount: Uint128::from(1000u64),
        }],
    );
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());
    let msg = InstantiateMsg {
        owner: MockApi::default().addr_make(OWNER).to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(0),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    app.execute_contract(
        MockApi::default().addr_make(OWNER),
        distributor_addr.clone(),
        &ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
            new_owner: MockApi::default().addr_make(OWNER2).to_string(),
            expiry: None,
        }),
        &[],
    )
    .unwrap();

    let ownership = get_owner(&app, &distributor_addr);
    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(MockApi::default().addr_make(OWNER)),
            pending_owner: Some(MockApi::default().addr_make(OWNER2)),
            pending_expiry: None
        }
    );

    app.execute_contract(
        MockApi::default().addr_make(OWNER2),
        distributor_addr.clone(),
        &ExecuteMsg::UpdateOwnership(Action::AcceptOwnership),
        &[],
    )
    .unwrap();

    let ownership = get_owner(&app, &distributor_addr);
    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(MockApi::default().addr_make(OWNER2)),
            pending_owner: None,
            pending_expiry: None
        }
    );
}

#[test]
fn test_ownership_expiry() {
    let mut app = App::default();
    let cw20_addr = instantiate_cw20(
        &mut app,
        vec![cw20::Cw20Coin {
            address: MockApi::default().addr_make(OWNER).to_string(),
            amount: Uint128::from(1000u64),
        }],
    );
    let staking_addr = instantiate_staking(&mut app, cw20_addr.clone());
    let msg = InstantiateMsg {
        owner: MockApi::default().addr_make(OWNER).to_string(),
        staking_addr: staking_addr.to_string(),
        reward_rate: Uint128::new(0),
        reward_token: cw20_addr.to_string(),
    };
    let distributor_addr = instantiate_distributor(&mut app, msg);

    app.execute_contract(
        MockApi::default().addr_make(OWNER),
        distributor_addr.clone(),
        &ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
            new_owner: MockApi::default().addr_make(OWNER2).to_string(),
            expiry: Some(Expiration::AtHeight(app.block_info().height + 1)),
        }),
        &[],
    )
    .unwrap();

    let ownership = get_owner(&app, &distributor_addr);
    assert_eq!(
        ownership,
        Ownership::<Addr> {
            owner: Some(MockApi::default().addr_make(OWNER)),
            pending_owner: Some(MockApi::default().addr_make(OWNER2)),
            pending_expiry: Some(Expiration::AtHeight(app.block_info().height + 1)),
        }
    );

    app.update_block(next_block);

    let err: ContractError = app
        .execute_contract(
            MockApi::default().addr_make(OWNER2),
            distributor_addr,
            &ExecuteMsg::UpdateOwnership(Action::AcceptOwnership),
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::Ownership(OwnershipError::TransferExpired)
    )
}

// v1 migration test removed: MigrateMsg::FromV1 variant was removed along
// with the v1 migration code path due to cosmwasm-std v1/v2 type conflicts.
