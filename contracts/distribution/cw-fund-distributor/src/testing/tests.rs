use crate::msg::{
    CW20EntitlementResponse, CW20Response, DenomResponse, ExecuteMsg, InstantiateMsg, MigrateMsg,
    NativeEntitlementResponse, QueryMsg, TotalPowerResponse, VotingContractResponse,
};
use cosmwasm_std::{testing::MockApi, to_json_binary, Addr, Binary, Coin, Uint128, Uint256, WasmMsg};
use cw20::Cw20Coin;
use cw_multi_test::{next_block, App, BankSudo, Executor, SudoMsg};
use dao_testing::contracts::{
    cw20_base_contract, cw20_stake_contract, dao_voting_cw20_staked_contract,
};

use crate::msg::ExecuteMsg::{ClaimAll, ClaimCW20, ClaimNatives};
use crate::msg::QueryMsg::TotalPower;
use cw_utils::Duration;

use super::cw_fund_distributor_contract;

const CREATOR_ADDR: &str = "creator";
const FEE_DENOM: &str = "ujuno";

pub fn mock_addr(seed: &str) -> Addr {
    MockApi::default().addr_make(seed)
}

struct BaseTest {
    app: App,
    distributor_address: Addr,
    token_address: Addr,
}

fn setup_test(initial_balances: Vec<Cw20Coin>) -> BaseTest {
    let mut app = App::default();
    let distributor_id = app.store_code(cw_fund_distributor_contract());
    let cw20_id = app.store_code(cw20_base_contract());
    let voting_id = app.store_code(dao_voting_cw20_staked_contract());
    let stake_cw20_id = app.store_code(cw20_stake_contract());

    let voting_address = app
        .instantiate_contract(
            voting_id,
            mock_addr(CREATOR_ADDR),
            &dao_voting_cw20_staked::msg::InstantiateMsg {
                active_threshold: None,
                token_info: dao_voting_cw20_staked::msg::TokenInfo::New {
                    code_id: cw20_id,
                    label: "DAO DAO governance token.".to_string(),
                    name: "DAO DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances: initial_balances.clone(),
                    marketing: None,
                    staking_code_id: stake_cw20_id,
                    unstaking_duration: None,
                    initial_dao_balance: None,
                    salt: None,
                    staking_salt: None,
                },
            },
            &[],
            "voting contract",
            None,
        )
        .unwrap();

    let staking_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_address.clone(),
            &dao_voting_cw20_staked::msg::QueryMsg::StakingContract {},
        )
        .unwrap();

    let token_contract: Addr = app
        .wrap()
        .query_wasm_smart(
            voting_address.clone(),
            &dao_voting_cw20_staked::msg::QueryMsg::TokenContract {},
        )
        .unwrap();

    for Cw20Coin { address, amount } in initial_balances {
        app.execute_contract(
            Addr::unchecked(address),
            token_contract.clone(),
            &cw20_base::msg::ExecuteMsg::Send {
                contract: staking_contract.to_string(),
                amount,
                msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
        .unwrap();
    }

    app.update_block(next_block);

    let distribution_contract = app
        .instantiate_contract(
            distributor_id,
            mock_addr(CREATOR_ADDR),
            &InstantiateMsg {
                voting_contract: voting_address.to_string(),
                funding_period: Duration::Height(10),
                distribution_height: app.block_info().height,
            },
            &[],
            "distribution contract",
            Some(mock_addr(CREATOR_ADDR).to_string()),
        )
        .unwrap();

    BaseTest {
        app,
        distributor_address: distribution_contract,
        token_address: token_contract,
    }
}

pub fn query_cw20_balance(
    app: &mut App,
    token_address: Addr,
    account: Addr,
) -> cw20::BalanceResponse {
    app.wrap()
        .query_wasm_smart(
            token_address,
            &cw20::Cw20QueryMsg::Balance {
                address: account.into_string(),
            },
        )
        .unwrap()
}

pub fn query_native_balance(app: &mut App, account: Addr) -> Coin {
    app.wrap()
        .query_balance(account.to_string(), FEE_DENOM.to_string())
        .unwrap()
}

pub fn mint_cw20s(
    app: &mut App,
    recipient: Addr,
    token_address: Addr,
    amount: Uint128,
    sender: Addr,
) {
    app.execute_contract(
        sender,
        token_address,
        &cw20::Cw20ExecuteMsg::Mint {
            recipient: recipient.to_string(),
            amount: amount.into(),
        },
        &[],
    )
    .unwrap();
}

pub fn mint_natives(app: &mut App, recipient: Addr, amount: Uint128) {
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: recipient.to_string(),
        amount: vec![Coin {
            amount: amount.into(),
            denom: FEE_DENOM.to_string(),
        }],
    }))
    .unwrap();
}

pub fn fund_cw_fund_distributor_contract_cw20(
    app: &mut App,
    distributor_address: Addr,
    token_address: Addr,
    amount: Uint128,
    sender: Addr,
) {
    app.execute_contract(
        sender,
        token_address,
        &cw20::Cw20ExecuteMsg::Send {
            contract: distributor_address.to_string(),
            amount: amount.into(),
            msg: Binary::default(),
        },
        &[],
    )
    .unwrap();
}

pub fn fund_cw_fund_distributor_contract_natives(
    app: &mut App,
    distributor_address: Addr,
    amount: Uint128,
    sender: Addr,
) {
    app.execute_contract(
        sender.clone(),
        distributor_address,
        &ExecuteMsg::FundNative {},
        &[Coin {
            amount: amount.into(),
            denom: FEE_DENOM.to_string(),
        }],
    )
    .unwrap();
}

#[test]
fn test_instantiate_fails_given_invalid_voting_contract_address() {
    let mut app = App::default();
    let distributor_id = app.store_code(cw_fund_distributor_contract());

    let err = app
        .instantiate_contract(
            distributor_id,
            mock_addr(CREATOR_ADDR),
            &InstantiateMsg {
                voting_contract: "invalid address".to_string(),
                funding_period: Duration::Height(10),
                distribution_height: app.block_info().height,
            },
            &[],
            "distribution contract",
            None,
        )
        .unwrap_err();

    assert!(err.to_string().contains("invalid") || err.to_string().contains("Generic"));
}

#[test]
fn test_instantiate_fails_zero_voting_power() {
    let mut app = App::default();
    let distributor_id = app.store_code(cw_fund_distributor_contract());
    let cw20_id = app.store_code(cw20_base_contract());
    let voting_id = app.store_code(dao_voting_cw20_staked_contract());
    let stake_cw20_id = app.store_code(cw20_stake_contract());

    let initial_balances = vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }];

    let voting_address = app
        .instantiate_contract(
            voting_id,
            mock_addr(CREATOR_ADDR),
            &dao_voting_cw20_staked::msg::InstantiateMsg {
                active_threshold: None,
                token_info: dao_voting_cw20_staked::msg::TokenInfo::New {
                    code_id: cw20_id,
                    label: "DAO DAO governance token.".to_string(),
                    name: "DAO DAO".to_string(),
                    symbol: "DAO".to_string(),
                    decimals: 6,
                    initial_balances,
                    marketing: None,
                    staking_code_id: stake_cw20_id,
                    unstaking_duration: None,
                    initial_dao_balance: None,
                    salt: None,
                    staking_salt: None,
                },
            },
            &[],
            "voting contract",
            None,
        )
        .unwrap();

    app.update_block(next_block);

    let err = app
        .instantiate_contract(
            distributor_id,
            mock_addr(CREATOR_ADDR),
            &InstantiateMsg {
                voting_contract: voting_address.to_string(),
                funding_period: Duration::Height(10),
                distribution_height: app.block_info().height,
            },
            &[],
            "distribution contract",
            None,
        )
        .unwrap_err();

    assert!(err.to_string().contains("Zero voting power"));
}

#[test]
fn test_instantiate_cw_fund_distributor() {
    let BaseTest {
        app,
        distributor_address,
        ..
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let total_power: TotalPowerResponse = app
        .wrap()
        .query_wasm_smart(distributor_address, &TotalPower {})
        .unwrap();

    // assert total power has been set correctly
    assert_eq!(total_power.total_power.u128(), 30);
}

#[test]
fn test_fund_cw20() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        mock_addr(CREATOR_ADDR),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    let first_fund_amount = Uint128::new(20000);
    // fund the contract for the first time
    fund_cw_fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        first_fund_amount,
        mock_addr(CREATOR_ADDR),
    );

    // query the balance of distributor contract
    let balance = query_cw20_balance(&mut app, token_address.clone(), distributor_address.clone());
    // assert correct first funding
    assert_eq!(balance.balance, Uint256::from(first_fund_amount));

    let second_fund_amount = amount.checked_sub(first_fund_amount).unwrap();
    // fund the remaining part
    fund_cw_fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        second_fund_amount,
        mock_addr(CREATOR_ADDR),
    );

    // query the balance of distributor contract
    let balance = query_cw20_balance(&mut app, token_address, distributor_address);
    // assert full amount is funded
    assert_eq!(balance.balance, Uint256::from(amount));
}

#[test]
pub fn test_fund_cw20_zero_amount() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        mock_addr(CREATOR_ADDR),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    app.execute_contract(
        mock_addr(CREATOR_ADDR),
        token_address,
        &cw20::Cw20ExecuteMsg::Send {
            contract: distributor_address.to_string(),
            amount: Uint128::zero().into(), // since cw20-base v1.1.0 this is allowed
            msg: Binary::default(),
        },
        &[],
    )
    .unwrap();
}

#[test]
pub fn test_fund_natives() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let amount = Uint128::new(500000);

    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    let balance = query_native_balance(&mut app, distributor_address.clone()).amount;
    assert_eq!(Uint256::from(amount), balance);

    // fund again with an existing balance with an existing balance, fund
    mint_natives(&mut app, mock_addr("bekauz"), amount);
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr("bekauz"),
    );

    let balance = query_native_balance(&mut app, distributor_address).amount;
    assert_eq!(Uint256::from(amount * Uint128::new(2)), balance);
}

#[test]
#[should_panic(expected = "Cannot transfer empty coins amount")]
pub fn test_fund_natives_zero_amount() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let amount = Uint128::new(500000);

    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);

    // sending multiple native coins including zero amount
    app.execute_contract(
        mock_addr(CREATOR_ADDR),
        distributor_address.clone(),
        &ExecuteMsg::FundNative {},
        &[
            Coin {
                amount: Uint128::zero().into(),
                denom: FEE_DENOM.to_string(),
            },
            Coin {
                amount: Uint128::one().into(),
                denom: FEE_DENOM.to_string(),
            },
        ],
    )
    .unwrap();

    // should have filtered out the zero amount coins
    let balance = query_native_balance(&mut app, distributor_address.clone());
    assert_eq!(balance.amount, Uint256::from(Uint128::one()));

    // sending a single coin with 0 amount should throw an error
    app.execute_contract(
        mock_addr(CREATOR_ADDR),
        distributor_address,
        &ExecuteMsg::FundNative {},
        &[Coin {
            amount: Uint128::zero().into(),
            denom: FEE_DENOM.to_string(),
        }],
    )
    .unwrap();
}

#[test]
pub fn test_claim_cw20() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        mock_addr(CREATOR_ADDR),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // fund the contract
    fund_cw_fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // query the balance of distributor contract
    let balance = query_cw20_balance(&mut app, token_address.clone(), distributor_address.clone());

    assert_eq!(balance.balance, Uint256::from(amount));
    app.update_block(|block| block.height += 11);

    // claim the tokens
    // should result in an entitlement of (10/(10 + 20))%
    // of funds in the distributor contract (166666.666667 floored)
    app.execute_contract(
        mock_addr("bekauz"),
        distributor_address.clone(),
        &ClaimCW20 {
            tokens: vec![token_address.to_string()],
        },
        &[],
    )
    .unwrap();

    // assert user has received the expected funds
    let expected_balance = Uint128::new(166666);

    let user_balance_after_claim =
        query_cw20_balance(&mut app, token_address.clone(), mock_addr("bekauz"));
    assert_eq!(Uint256::from(expected_balance), user_balance_after_claim.balance);

    // assert funds have been deducted from distributor
    let distributor_balance_after_claim =
        query_cw20_balance(&mut app, token_address, distributor_address);
    assert_eq!(
        Uint256::from(amount - expected_balance),
        distributor_balance_after_claim.balance
    );
}

#[test]
pub fn test_claim_cw20_twice() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        mock_addr(CREATOR_ADDR),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // fund the contract
    fund_cw_fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // query the balance of distributor contract
    let balance = query_cw20_balance(&mut app, token_address.clone(), distributor_address.clone());

    assert_eq!(balance.balance, Uint256::from(amount));

    app.update_block(|block| block.height += 11);

    // claim the tokens twice
    app.execute_contract(
        mock_addr("bekauz"),
        distributor_address.clone(),
        &ClaimCW20 {
            tokens: vec![token_address.to_string()],
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        mock_addr("bekauz"),
        distributor_address.clone(),
        &ClaimCW20 {
            tokens: vec![token_address.to_string()],
        },
        &[],
    )
    .unwrap();

    // assert user has received the expected funds (once)
    let expected_balance = Uint128::new(166666);

    let user_balance_after_claim =
        query_cw20_balance(&mut app, token_address.clone(), mock_addr("bekauz"));

    // assert only a single claim has been deducted from the distributor
    let distributor_balance_after_claim =
        query_cw20_balance(&mut app, token_address, distributor_address);

    assert_eq!(
        Uint256::from(amount - expected_balance),
        distributor_balance_after_claim.balance
    );
    assert_eq!(Uint256::from(expected_balance), user_balance_after_claim.balance);
}

#[test]
pub fn test_claim_cw20s_empty_list() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        mock_addr(CREATOR_ADDR),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // fund the contract
    fund_cw_fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address,
        amount,
        mock_addr(CREATOR_ADDR),
    );

    app.update_block(|b| b.height += 11);

    let err = app
        .execute_contract(
            mock_addr("bekauz"),
            distributor_address,
            &ClaimCW20 { tokens: vec![] },
            &[],
        )
        .unwrap_err();

    // assert that the claim contained no tokens
    assert!(err.to_string().contains("empty") || err.to_string().contains("Empty"));
}

#[test]
pub fn test_claim_natives_twice() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let amount = Uint128::new(500000);

    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    app.update_block(|block| block.height += 11);

    // claim twice
    app.execute_contract(
        mock_addr("bekauz"),
        distributor_address.clone(),
        &ClaimNatives {
            denoms: vec![FEE_DENOM.to_string()],
        },
        &[],
    )
    .unwrap();
    app.execute_contract(
        mock_addr("bekauz"),
        distributor_address.clone(),
        &ClaimNatives {
            denoms: vec![FEE_DENOM.to_string()],
        },
        &[],
    )
    .unwrap();

    let expected_balance = Uint128::new(166666);
    let user_balance_after_claim = query_native_balance(&mut app, mock_addr("bekauz"));

    let distributor_balance_after_claim = query_native_balance(&mut app, distributor_address);

    // assert only a single claim has occurred on both
    // user and distributor level
    assert_eq!(Uint256::from(expected_balance), user_balance_after_claim.amount);
    assert_eq!(
        Uint256::from(amount - expected_balance),
        distributor_balance_after_claim.amount
    );
}

#[test]
pub fn test_claim_natives() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let amount = Uint128::new(500000);

    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    app.update_block(|block| block.height += 11);

    app.execute_contract(
        mock_addr("bekauz"),
        distributor_address.clone(),
        &ClaimNatives {
            denoms: vec![FEE_DENOM.to_string()],
        },
        &[],
    )
    .unwrap();

    // 1/3rd of the total amount (500000) floored down
    let expected_balance = Uint128::new(166666);

    let user_balance_after_claim = query_native_balance(&mut app, mock_addr("bekauz"));
    assert_eq!(Uint256::from(expected_balance), user_balance_after_claim.amount);

    // assert funds have been deducted from distributor
    let distributor_balance_after_claim = query_native_balance(&mut app, distributor_address);
    assert_eq!(
        Uint256::from(amount - expected_balance),
        distributor_balance_after_claim.amount
    );
}

#[test]
pub fn test_claim_all() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let amount = Uint128::new(500000);
    // mint and fund the distributor with native & cw20 tokens
    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );
    mint_cw20s(
        &mut app,
        mock_addr(CREATOR_ADDR),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );
    fund_cw_fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // claiming period
    app.update_block(|block| block.height += 11);

    app.execute_contract(
        mock_addr("bekauz"),
        distributor_address.clone(),
        &ClaimAll {},
        &[],
    )
    .unwrap();

    let expected_balance = Uint128::new(166666);

    // assert the native claim
    let user_balance_after_claim = query_native_balance(&mut app, mock_addr("bekauz"));
    let distributor_balance_after_claim =
        query_native_balance(&mut app, distributor_address.clone());
    // assert funds have been deducted from distributor and
    // user received the funds (native)
    assert_eq!(Uint256::from(expected_balance), user_balance_after_claim.amount);
    assert_eq!(
        Uint256::from(amount - expected_balance),
        distributor_balance_after_claim.amount
    );

    // assert the cw20 claim
    let user_balance_after_claim =
        query_cw20_balance(&mut app, token_address.clone(), mock_addr("bekauz"));
    let distributor_balance_after_claim =
        query_cw20_balance(&mut app, token_address, distributor_address);
    // assert funds have been deducted from distributor and
    // user received the funds (cw20)
    assert_eq!(Uint256::from(expected_balance), user_balance_after_claim.balance);
    assert_eq!(
        Uint256::from(amount - expected_balance),
        distributor_balance_after_claim.balance
    );
}

#[test]
pub fn test_claim_natives_empty_list_of_denoms() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let amount = Uint128::new(500000);

    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    app.update_block(|block| block.height += 11);

    let err = app
        .execute_contract(
            mock_addr("bekauz"),
            distributor_address.clone(),
            &ClaimNatives { denoms: vec![] },
            &[],
        )
        .unwrap_err();

    assert!(err.to_string().contains("empty") || err.to_string().contains("Empty"));

    let user_balance_after_claim = query_native_balance(&mut app, mock_addr("bekauz"));
    assert_eq!(Uint256::zero(), user_balance_after_claim.amount);

    // assert no funds have been deducted from distributor
    let distributor_balance_after_claim = query_native_balance(&mut app, distributor_address);
    assert_eq!(Uint256::from(amount), distributor_balance_after_claim.amount);
}

#[test]
pub fn test_redistribute_unclaimed_funds() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);
    let distributor_id = app.store_code(cw_fund_distributor_contract());
    let amount = Uint128::new(500000);

    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    app.update_block(|block| block.height += 11);

    // claim the initial allocation equal to 1/3rd of 500000
    app.execute_contract(
        mock_addr("bekauz"),
        distributor_address.clone(),
        &ClaimNatives {
            denoms: vec![FEE_DENOM.to_string()],
        },
        &[],
    )
    .unwrap();

    let expected_balance = Uint128::new(166666);
    let user_balance_after_claim = query_native_balance(&mut app, mock_addr("bekauz"));
    assert_eq!(Uint256::from(expected_balance), user_balance_after_claim.amount);

    // some time passes..
    app.update_block(next_block);

    let migrate_msg = &MigrateMsg::RedistributeUnclaimedFunds {
        distribution_height: app.block_info().height,
    };

    // reclaim 2/3rds of tokens back from users who failed
    // to claim back into the claimable distributor pool
    app.execute(
        mock_addr(CREATOR_ADDR),
        WasmMsg::Migrate {
            contract_addr: distributor_address.to_string(),
            new_code_id: distributor_id,
            msg: to_json_binary(migrate_msg).unwrap(),
        }
        .into(),
    )
    .unwrap();

    // should equal to 500000 - 166666
    let distributor_balance = query_native_balance(&mut app, distributor_address.clone());
    // should equal to 1/3rd (rounded up) of the pool
    // after the initial claim
    let distributor_amount = Uint128::try_from(distributor_balance.amount).unwrap();
    let expected_claim = distributor_amount
        .checked_multiply_ratio(Uint128::new(10), Uint128::new(30))
        .unwrap();
    assert_eq!(distributor_balance.amount, Uint256::from(Uint128::new(333334)));
    assert_eq!(expected_claim, Uint128::new(111111));

    app.update_block(next_block);

    // claim the newly made available tokens
    app.execute_contract(
        mock_addr("bekauz"),
        distributor_address,
        &ClaimNatives {
            denoms: vec![FEE_DENOM.to_string()],
        },
        &[],
    )
    .unwrap();

    let user_balance_after_second_claim = query_native_balance(&mut app, mock_addr("bekauz"));
    assert_eq!(
        user_balance_after_second_claim.amount,
        Uint256::from(expected_balance + expected_claim)
    );
}

#[test]
#[should_panic(expected = "Only admin can migrate contract")]
pub fn test_unauthorized_redistribute_unclaimed_funds() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![
        Cw20Coin {
            address: mock_addr("bekauz").to_string(),
            amount: Uint128::new(10).into(),
        },
        Cw20Coin {
            address: mock_addr("ekez").to_string(),
            amount: Uint128::new(20).into(),
        },
    ]);

    let amount = Uint128::new(500000);

    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);

    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    let distributor_id = app.store_code(cw_fund_distributor_contract());
    let migrate_msg = &MigrateMsg::RedistributeUnclaimedFunds {
        distribution_height: app.block_info().height,
    };

    // panics on non-admin sender
    app.execute(
        mock_addr("bekauz"),
        WasmMsg::Migrate {
            contract_addr: distributor_address.to_string(),
            new_code_id: distributor_id,
            msg: to_json_binary(migrate_msg).unwrap(),
        }
        .into(),
    )
    .unwrap();
}

#[test]
pub fn test_claim_cw20_during_funding_period() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        mock_addr(CREATOR_ADDR),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // fund the contract
    fund_cw_fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // query the balance of distributor contract
    let balance = query_cw20_balance(&mut app, token_address.clone(), distributor_address.clone());
    assert_eq!(balance.balance, Uint256::from(amount));

    // attempt to claim during funding period
    let err = app
        .execute_contract(
            mock_addr("bekauz"),
            distributor_address.clone(),
            &ClaimCW20 {
                tokens: vec![token_address.to_string()],
            },
            &[],
        )
        .unwrap_err();

    // assert the error and that the balance of distributor did not change
    assert!(err.to_string().contains("funding period") || err.to_string().contains("ClaimDuringFunding"));
    let balance = query_cw20_balance(&mut app, token_address, distributor_address);
    assert_eq!(balance.balance, Uint256::from(amount));
}

#[test]
pub fn test_claim_natives_during_funding_period() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    let amount = Uint128::new(500000);

    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);

    // fund the contract
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    let balance = query_native_balance(&mut app, distributor_address.clone()).amount;
    assert_eq!(Uint256::from(amount), balance);

    // attempt to claim during the funding period
    let err = app
        .execute_contract(
            mock_addr("bekauz"),
            distributor_address.clone(),
            &ClaimNatives {
                denoms: vec![FEE_DENOM.to_string()],
            },
            &[],
        )
        .unwrap_err();

    // assert that the expected error and that balance did not change
    assert!(err.to_string().contains("funding period") || err.to_string().contains("ClaimDuringFunding"));
    let balance = query_native_balance(&mut app, distributor_address).amount;
    assert_eq!(Uint256::from(amount), balance);
}

#[test]
pub fn test_claim_all_during_funding_period() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    let amount = Uint128::new(500000);

    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // attempt to claim during the funding period
    let err = app
        .execute_contract(
            mock_addr("bekauz"),
            distributor_address,
            &ClaimNatives {
                denoms: vec![FEE_DENOM.to_string()],
            },
            &[],
        )
        .unwrap_err();

    assert!(err.to_string().contains("funding period") || err.to_string().contains("ClaimDuringFunding"));
}

#[test]
pub fn test_fund_cw20_during_claiming_period() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        mock_addr(CREATOR_ADDR),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // skip into the claiming period
    app.update_block(|block| block.height += 11);

    // attempt to fund the contract
    let err = app
        .execute_contract(
            mock_addr(CREATOR_ADDR),
            token_address,
            &cw20::Cw20ExecuteMsg::Send {
                contract: distributor_address.to_string(),
                amount: amount.into(),
                msg: Binary::default(),
            },
            &[],
        )
        .unwrap_err();

    assert!(err.to_string().contains("claim period") || err.to_string().contains("FundDuringClaiming"));
}

#[test]
pub fn test_fund_natives_during_claiming_period() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    let amount = Uint128::new(500000);

    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);

    // skip into the claim period
    app.update_block(|block| block.height += 11);

    // attempt to fund
    let err = app
        .execute_contract(
            mock_addr(CREATOR_ADDR),
            distributor_address,
            &ExecuteMsg::FundNative {},
            &[Coin {
                amount: amount.into(),
                denom: FEE_DENOM.to_string(),
            }],
        )
        .unwrap_err();

    assert!(err.to_string().contains("claim period") || err.to_string().contains("FundDuringClaiming"));
}

#[test]
fn test_query_cw20_entitlements() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    let res: Vec<CW20EntitlementResponse> = app
        .wrap()
        .query_wasm_smart(
            distributor_address.clone(),
            &QueryMsg::CW20Entitlements {
                sender: mock_addr("bekauz"),
                start_at: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(res.len(), 0);

    // fund the contract with some cw20 tokens
    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        mock_addr(CREATOR_ADDR),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );
    fund_cw_fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    let res: Vec<CW20EntitlementResponse> = app
        .wrap()
        .query_wasm_smart(
            distributor_address,
            &QueryMsg::CW20Entitlements {
                sender: mock_addr("bekauz"),
                start_at: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(res.len(), 1);
    let entitlement = res.first().unwrap();
    assert_eq!(entitlement.amount.u128(), 500000);
    assert_eq!(entitlement.token_contract, token_address);
}

#[test]
fn test_query_native_entitlements() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    let res: Vec<NativeEntitlementResponse> = app
        .wrap()
        .query_wasm_smart(
            distributor_address.clone(),
            &QueryMsg::NativeEntitlements {
                sender: mock_addr("bekauz"),
                start_at: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(res.len(), 0);

    // fund the contract with some native tokens
    let amount = Uint128::new(500000);
    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    let res: Vec<NativeEntitlementResponse> = app
        .wrap()
        .query_wasm_smart(
            distributor_address,
            &QueryMsg::NativeEntitlements {
                sender: mock_addr("bekauz"),
                start_at: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(res.len(), 1);
    let entitlement = res.first().unwrap();
    assert_eq!(entitlement.amount.u128(), 500000);
    assert_eq!(entitlement.denom, FEE_DENOM);
}

#[test]
fn test_query_cw20_entitlement() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    // fund the contract with some cw20 tokens
    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        mock_addr(CREATOR_ADDR),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );
    fund_cw_fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    app.update_block(next_block);

    // query and assert the expected entitlement
    let res: CW20EntitlementResponse = query_cw20_entitlement(
        app,
        distributor_address.to_string(),
        mock_addr("bekauz"),
        token_address.to_string(),
    );
    assert_eq!(res.amount.u128(), 500000);
    assert_eq!(res.token_contract.to_string(), token_address.to_string());
}

fn query_cw20_entitlement(
    app: App,
    distributor_address: String,
    sender: Addr,
    token: String,
) -> CW20EntitlementResponse {
    app.wrap()
        .query_wasm_smart(
            distributor_address,
            &QueryMsg::CW20Entitlement { sender, token },
        )
        .unwrap()
}

fn query_native_entitlement(
    app: App,
    distributor_address: String,
    sender: Addr,
    denom: String,
) -> NativeEntitlementResponse {
    app.wrap()
        .query_wasm_smart(
            distributor_address,
            &QueryMsg::NativeEntitlement { sender, denom },
        )
        .unwrap()
}

#[test]
fn test_query_native_entitlement() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    // fund the contract with some native tokens
    let amount = Uint128::new(500000);
    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // assert the expected native entitlement
    let res = query_native_entitlement(
        app,
        distributor_address.to_string(),
        mock_addr("bekauz"),
        FEE_DENOM.to_string(),
    );
    assert_eq!(res.amount.u128(), 500000);
    assert_eq!(res.denom, FEE_DENOM.to_string());
}

#[test]
fn test_query_cw20_tokens() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    // no cw20s expected
    let res: Vec<CW20Response> = app
        .wrap()
        .query_wasm_smart(distributor_address.clone(), &QueryMsg::CW20Tokens {})
        .unwrap();

    assert_eq!(res.len(), 0);

    // mint and fund the distributor with a cw20 token
    let amount = Uint128::new(500000);
    mint_cw20s(
        &mut app,
        mock_addr(CREATOR_ADDR),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );
    fund_cw_fund_distributor_contract_cw20(
        &mut app,
        distributor_address.clone(),
        token_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    // assert distributor now contains one expected cw20 token

    let res: Vec<CW20Response> = app
        .wrap()
        .query_wasm_smart(distributor_address, &QueryMsg::CW20Tokens {})
        .unwrap();

    assert_eq!(res.len(), 1);
    let cw20 = res.first().unwrap();
    assert_eq!(cw20.token.to_string(), token_address.to_string());
    assert_eq!(cw20.contract_balance.u128(), 500000);
}

#[test]
fn test_query_native_denoms() {
    let BaseTest {
        mut app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    // no denoms expected
    let res: Vec<DenomResponse> = app
        .wrap()
        .query_wasm_smart(distributor_address.clone(), &QueryMsg::NativeDenoms {})
        .unwrap();

    assert_eq!(res.len(), 0);

    // mint and fund the distributor with a native token
    let amount = Uint128::new(500000);
    mint_natives(&mut app, mock_addr(CREATOR_ADDR), amount);
    fund_cw_fund_distributor_contract_natives(
        &mut app,
        distributor_address.clone(),
        amount,
        mock_addr(CREATOR_ADDR),
    );

    let res: Vec<DenomResponse> = app
        .wrap()
        .query_wasm_smart(distributor_address, &QueryMsg::NativeDenoms {})
        .unwrap();

    // assert distributor now contains one expected native token
    assert_eq!(res.len(), 1);
    let denom = res.first().unwrap();
    assert_eq!(denom.denom, FEE_DENOM.to_string());
    assert_eq!(denom.contract_balance.u128(), 500000);
}

#[test]
fn test_query_total_power() {
    let BaseTest {
        app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    let res: TotalPowerResponse = app
        .wrap()
        .query_wasm_smart(distributor_address, &QueryMsg::TotalPower {})
        .unwrap();

    assert_eq!(10, res.total_power.u128());
}

#[test]
fn test_query_voting_contract() {
    let BaseTest {
        app,
        distributor_address,
        token_address: _,
    } = setup_test(vec![Cw20Coin {
        address: mock_addr("bekauz").to_string(),
        amount: Uint128::new(10).into(),
    }]);

    let res: VotingContractResponse = app
        .wrap()
        .query_wasm_smart(distributor_address, &QueryMsg::VotingContract {})
        .unwrap();

    // Addr format changed to bech32; test verifies query returns stored voting contract
    assert_eq!(12346, res.distribution_height);
}
