use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, MockApi},
    to_json_binary, Addr, Coin, MigrateInfo, Uint256,
};
use cw20::Cw20Coin;
use cw_multi_test::{App, BankSudo, Executor, SudoMsg};
use dao_testing::contracts::{cw20_base_contract, cw_token_swap_contract};

use crate::{
    contract::{migrate, CONTRACT_NAME, CONTRACT_VERSION},
    msg::{
        Counterparty, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, StatusResponse, TokenInfo,
    },
    state::{CheckedCounterparty, CheckedTokenInfo},
};

const DAO1: &str = "dao1";
const DAO2: &str = "dao2";

#[test]
fn test_simple_escrow() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_base_contract());
    let escrow_code = app.store_code(cw_token_swap_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            MockApi::default().addr_make(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    amount: Uint256::from(100u128),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            MockApi::default().addr_make(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
                counterparty_two: Counterparty {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    app.execute_contract(
        MockApi::default().addr_make(DAO2),
        cw20.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: escrow.to_string(),
            amount: Uint256::from(100u128),
            msg: to_json_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: MockApi::default().addr_make(DAO1).to_string(),
        amount: vec![Coin {
            amount: Uint256::from(100u128),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    app.execute_contract(
        MockApi::default().addr_make(DAO1),
        escrow,
        &ExecuteMsg::Fund {},
        &[Coin {
            amount: Uint256::from(100u128),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    let dao1_balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20,
            &cw20::Cw20QueryMsg::Balance {
                address: MockApi::default().addr_make(DAO1).to_string(),
            },
        )
        .unwrap();
    assert_eq!(dao1_balance.balance, Uint256::from(100u128));

    let dao2_balance = app.wrap().query_balance(MockApi::default().addr_make(DAO2), "ujuno").unwrap();
    assert_eq!(dao2_balance.amount, Uint256::from(100u128))
}

#[test]
fn test_withdraw() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_base_contract());
    let escrow_code = app.store_code(cw_token_swap_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            MockApi::default().addr_make(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    amount: Uint256::from(100u128),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            MockApi::default().addr_make(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
                counterparty_two: Counterparty {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    // Can't withdraw before you provide.
    let err = app
        .execute_contract(
            MockApi::default().addr_make(DAO2),
            escrow.clone(),
            &ExecuteMsg::Withdraw {},
            &[],
        )
        .unwrap_err();
    assert!(err.to_string().contains("Must provide funds before withdrawing"));

    app.execute_contract(
        MockApi::default().addr_make(DAO2),
        cw20.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: escrow.to_string(),
            amount: Uint256::from(100u128),
            msg: to_json_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    // Change our minds.
    app.execute_contract(
        MockApi::default().addr_make(DAO2),
        escrow.clone(),
        &ExecuteMsg::Withdraw {},
        &[],
    )
    .unwrap();

    let dao2_balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20.clone(),
            &cw20::Cw20QueryMsg::Balance {
                address: MockApi::default().addr_make(DAO2).to_string(),
            },
        )
        .unwrap();
    assert_eq!(dao2_balance.balance, Uint256::from(100u128));

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: MockApi::default().addr_make(DAO1).to_string(),
        amount: vec![Coin {
            amount: Uint256::from(100u128),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    app.execute_contract(
        MockApi::default().addr_make(DAO1),
        escrow.clone(),
        &ExecuteMsg::Fund {},
        &[Coin {
            amount: Uint256::from(100u128),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    let status: StatusResponse = app
        .wrap()
        .query_wasm_smart(escrow.clone(), &QueryMsg::Status {})
        .unwrap();
    assert_eq!(
        status,
        StatusResponse {
            counterparty_one: CheckedCounterparty {
                address: MockApi::default().addr_make(DAO1),
                promise: CheckedTokenInfo::Native {
                    denom: "ujuno".to_string(),
                    amount: Uint256::from(100u128)
                },
                provided: true,
            },
            counterparty_two: CheckedCounterparty {
                address: MockApi::default().addr_make(DAO2),
                promise: CheckedTokenInfo::Cw20 {
                    contract_addr: cw20.clone(),
                    amount: Uint256::from(100u128)
                },
                provided: false,
            }
        }
    );

    // Change our minds.
    app.execute_contract(
        MockApi::default().addr_make(DAO1),
        escrow.clone(),
        &ExecuteMsg::Withdraw {},
        &[],
    )
    .unwrap();

    let dao1_balance = app.wrap().query_balance(MockApi::default().addr_make(DAO1), "ujuno").unwrap();
    assert_eq!(dao1_balance.amount, Uint256::from(100u128));

    let status: StatusResponse = app
        .wrap()
        .query_wasm_smart(escrow, &QueryMsg::Status {})
        .unwrap();
    assert_eq!(
        status,
        StatusResponse {
            counterparty_one: CheckedCounterparty {
                address: MockApi::default().addr_make(DAO1),
                promise: CheckedTokenInfo::Native {
                    denom: "ujuno".to_string(),
                    amount: Uint256::from(100u128)
                },
                provided: false,
            },
            counterparty_two: CheckedCounterparty {
                address: MockApi::default().addr_make(DAO2),
                promise: CheckedTokenInfo::Cw20 {
                    contract_addr: cw20,
                    amount: Uint256::from(100u128)
                },
                provided: false,
            }
        }
    )
}

#[test]
fn test_withdraw_post_completion() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_base_contract());
    let escrow_code = app.store_code(cw_token_swap_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            MockApi::default().addr_make(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    amount: Uint256::from(100u128),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            MockApi::default().addr_make(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
                counterparty_two: Counterparty {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    app.execute_contract(
        MockApi::default().addr_make(DAO2),
        cw20.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: escrow.to_string(),
            amount: Uint256::from(100u128),
            msg: to_json_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: MockApi::default().addr_make(DAO1).to_string(),
        amount: vec![Coin {
            amount: Uint256::from(100u128),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    app.execute_contract(
        MockApi::default().addr_make(DAO1),
        escrow.clone(),
        &ExecuteMsg::Fund {},
        &[Coin {
            amount: Uint256::from(100u128),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    let dao1_balance: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20,
            &cw20::Cw20QueryMsg::Balance {
                address: MockApi::default().addr_make(DAO1).to_string(),
            },
        )
        .unwrap();
    assert_eq!(dao1_balance.balance, Uint256::from(100u128));

    let dao2_balance = app.wrap().query_balance(MockApi::default().addr_make(DAO2), "ujuno").unwrap();
    assert_eq!(dao2_balance.amount, Uint256::from(100u128));

    let err = app
        .execute_contract(MockApi::default().addr_make(DAO1), escrow, &ExecuteMsg::Withdraw {}, &[])
        .unwrap_err();
    assert!(err.to_string().contains("Escrow funds have already been sent"))
}

#[test]
fn test_invalid_instantiate() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_base_contract());
    let escrow_code = app.store_code(cw_token_swap_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            MockApi::default().addr_make(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    amount: Uint256::from(100u128),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    // Zero amount not allowed for native tokens.
    let err = app
        .instantiate_contract(
            escrow_code,
            MockApi::default().addr_make(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::zero(),
                    },
                },
                counterparty_two: Counterparty {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap_err();

    assert!(err.to_string().contains("Can not create an escrow for zero tokens"));

    // Zero amount not allowed for cw20 tokens.
    let err = app
        .instantiate_contract(
            escrow_code,
            MockApi::default().addr_make(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
                counterparty_two: Counterparty {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint256::zero(),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap_err();

    assert!(err.to_string().contains("Can not create an escrow for zero tokens"))
}

#[test]
fn test_non_distincy_counterparties() {
    let mut app = App::default();

    let escrow_code = app.store_code(cw_token_swap_contract());

    // Zero amount not allowed for native tokens.
    let err = app
        .instantiate_contract(
            escrow_code,
            MockApi::default().addr_make(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::from(110u128),
                    },
                },
                counterparty_two: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::from(10u128),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap_err();

    assert!(err.to_string().contains("Counterparties must have different addresses"));
}

#[test]
fn test_fund_non_counterparty() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_base_contract());
    let escrow_code = app.store_code(cw_token_swap_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            MockApi::default().addr_make(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make("noah").to_string(),
                    amount: Uint256::from(100u128),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            MockApi::default().addr_make(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
                counterparty_two: Counterparty {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    let err = app
        .execute_contract(
            MockApi::default().addr_make("noah"),
            cw20,
            &cw20::Cw20ExecuteMsg::Send {
                contract: escrow.to_string(),
                amount: Uint256::from(100u128),
                msg: to_json_binary("").unwrap(),
            },
            &[],
        )
        .unwrap_err();

    assert!(err.to_string().contains("Unauthorized"));

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: MockApi::default().addr_make("noah").to_string(),
        amount: vec![Coin {
            amount: Uint256::from(100u128),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    let err = app
        .execute_contract(
            MockApi::default().addr_make("noah"),
            escrow,
            &ExecuteMsg::Fund {},
            &[Coin {
                amount: Uint256::from(100u128),
                denom: "ujuno".to_string(),
            }],
        )
        .unwrap_err();

    assert!(err.to_string().contains("Unauthorized"));
}

#[test]
fn test_fund_twice() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_base_contract());
    let escrow_code = app.store_code(cw_token_swap_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            MockApi::default().addr_make(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    amount: Uint256::from(200u128),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            MockApi::default().addr_make(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
                counterparty_two: Counterparty {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    app.execute_contract(
        MockApi::default().addr_make(DAO2),
        cw20.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: escrow.to_string(),
            amount: Uint256::from(100u128),
            msg: to_json_binary("").unwrap(),
        },
        &[],
    )
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: MockApi::default().addr_make(DAO1).to_string(),
        amount: vec![Coin {
            amount: Uint256::from(200u128),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    app.execute_contract(
        MockApi::default().addr_make(DAO1),
        escrow.clone(),
        &ExecuteMsg::Fund {},
        &[Coin {
            amount: Uint256::from(100u128),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    let err = app
        .execute_contract(
            MockApi::default().addr_make(DAO1),
            escrow.clone(),
            &ExecuteMsg::Fund {},
            &[Coin {
                amount: Uint256::from(100u128),
                denom: "ujuno".to_string(),
            }],
        )
        .unwrap_err();

    assert!(err.to_string().contains("Can not provide funds more than once"));

    let err = app
        .execute_contract(
            MockApi::default().addr_make(DAO2),
            cw20,
            &cw20::Cw20ExecuteMsg::Send {
                contract: escrow.into_string(),
                amount: Uint256::from(100u128),
                msg: to_json_binary("").unwrap(),
            },
            &[],
        )
        .unwrap_err();

    assert!(err.to_string().contains("Can not provide funds more than once"));
}

#[test]
fn test_fund_invalid_amount() {
    let mut app = App::default();

    let cw20_code = app.store_code(cw20_base_contract());
    let escrow_code = app.store_code(cw_token_swap_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            MockApi::default().addr_make(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    amount: Uint256::from(200u128),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            MockApi::default().addr_make(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
                counterparty_two: Counterparty {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    let err = app
        .execute_contract(
            MockApi::default().addr_make(DAO2),
            cw20,
            &cw20::Cw20ExecuteMsg::Send {
                contract: escrow.to_string(),
                amount: Uint256::from(10u128),
                msg: to_json_binary("").unwrap(),
            },
            &[],
        )
        .unwrap_err();

    assert!(err.to_string().contains("Invalid amount. Expected (100), got (10)"));

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: MockApi::default().addr_make(DAO1).to_string(),
        amount: vec![Coin {
            amount: Uint256::from(200u128),
            denom: "ujuno".to_string(),
        }],
    }))
    .unwrap();

    let err = app
        .execute_contract(
            MockApi::default().addr_make(DAO1),
            escrow,
            &ExecuteMsg::Fund {},
            &[Coin {
                amount: Uint256::from(200u128),
                denom: "ujuno".to_string(),
            }],
        )
        .unwrap_err();

    assert!(err.to_string().contains("Invalid amount. Expected (100), got (200)"));
}

#[test]
fn test_fund_invalid_denom() {
    let mut app = App::default();

    let escrow_code = app.store_code(cw_token_swap_contract());

    let escrow = app
        .instantiate_contract(
            escrow_code,
            MockApi::default().addr_make(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
                counterparty_two: Counterparty {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    promise: TokenInfo::Native {
                        denom: "uekez".to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    // Coutnerparty one tries to fund in the denom of counterparty
    // two.
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: MockApi::default().addr_make(DAO1).to_string(),
        amount: vec![Coin {
            amount: Uint256::from(100u128),
            denom: "uekez".to_string(),
        }],
    }))
    .unwrap();

    let err = app
        .execute_contract(
            MockApi::default().addr_make(DAO1),
            escrow,
            &ExecuteMsg::Fund {},
            &[Coin {
                amount: Uint256::from(100u128),
                denom: "uekez".to_string(),
            }],
        )
        .unwrap_err();

    assert!(err.to_string().contains("Provided funds do not match promised funds"))
}

#[test]
fn test_fund_invalid_cw20() {
    let mut app = App::default();

    let escrow_code = app.store_code(cw_token_swap_contract());
    let cw20_code = app.store_code(cw20_base_contract());

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            MockApi::default().addr_make(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    amount: Uint256::from(100u128),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let bad_cw20 = app
        .instantiate_contract(
            cw20_code,
            MockApi::default().addr_make(DAO2),
            &cw20_base::msg::InstantiateMsg {
                name: "coin coin".to_string(),
                symbol: "coin".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    amount: Uint256::from(100u128),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "coin",
            None,
        )
        .unwrap();

    let escrow = app
        .instantiate_contract(
            escrow_code,
            MockApi::default().addr_make(DAO1),
            &InstantiateMsg {
                counterparty_one: Counterparty {
                    address: MockApi::default().addr_make(DAO1).to_string(),
                    promise: TokenInfo::Native {
                        denom: "ujuno".to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
                counterparty_two: Counterparty {
                    address: MockApi::default().addr_make(DAO2).to_string(),
                    promise: TokenInfo::Cw20 {
                        contract_addr: cw20.to_string(),
                        amount: Uint256::from(100u128),
                    },
                },
            },
            &[],
            "escrow",
            None,
        )
        .unwrap();

    // Try and fund the contract with the wrong cw20.
    let err = app
        .execute_contract(
            MockApi::default().addr_make(DAO2),
            bad_cw20,
            &cw20::Cw20ExecuteMsg::Send {
                contract: escrow.to_string(),
                amount: Uint256::from(100u128),
                msg: to_json_binary("").unwrap(),
            },
            &[],
        )
        .unwrap_err();

    assert!(err.to_string().contains("Provided funds do not match promised funds"));

    // Try and fund the contract with the correct cw20 but incorrect
    // provider.
    let err = app
        .execute_contract(
            MockApi::default().addr_make(DAO1),
            cw20,
            &cw20::Cw20ExecuteMsg::Send {
                contract: escrow.to_string(),
                amount: Uint256::from(100u128),
                msg: to_json_binary("").unwrap(),
            },
            &[],
        )
        .unwrap_err();

    assert!(err.to_string().contains("Provided funds do not match promised funds"))
}

#[test]
pub fn test_migrate_update_version() {
    let mut deps = mock_dependencies();
    cw2::set_contract_version(&mut deps.storage, "my-contract", "old-version").unwrap();
    migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg {},
        MigrateInfo {
            sender: Addr::unchecked(""),
            old_migrate_version: None,
        },
    )
    .unwrap();
    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(version.version, CONTRACT_VERSION);
    assert_eq!(version.contract, CONTRACT_NAME);
}
