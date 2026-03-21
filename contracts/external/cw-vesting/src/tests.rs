use cosmwasm_std::testing::{mock_dependencies, mock_env, message_info, MockApi};
use cosmwasm_std::{coins, to_json_binary, Addr, Coin, Decimal, Uint128, Validator};
use cw20::{Cw20Coin, Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_multi_test::{App, AppBuilder, BankSudo, Executor, StakingInfo, SudoMsg};
use cw_ownable::Action;
use dao_testing::contracts::{cw20_base_contract, cw_vesting_contract};

use crate::contract::{execute, execute_receive_cw20};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::PAYMENT;
use crate::vesting::{Schedule, Status, Vest, VestInit};
use crate::ContractError;

const ALICE: &str = "alice";
const BOB: &str = "bob";
const INITIAL_BALANCE: u128 = 1000000000;
const TOTAL_VEST: u128 = 1000000;
const OWNER: &str = "owner";
const NATIVE_DENOM: &str = "ujuno";

fn get_vesting_payment(app: &App, cw_vesting_addr: Addr) -> Vest {
    app.wrap()
        .query_wasm_smart(cw_vesting_addr, &QueryMsg::Info {})
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

fn get_balance_native<T: Into<String>, U: Into<String>>(
    app: &App,
    address: T,
    denom: U,
) -> Uint128 {
    app.wrap().query_balance(address, denom).unwrap().amount
}

pub fn setup_app() -> App {
    let mut app = App::default();

    // Mint Alice and Bob native tokens
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: MockApi::default().addr_make(ALICE).to_string(),
            amount: coins(INITIAL_BALANCE, NATIVE_DENOM),
        }
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: MockApi::default().addr_make(BOB).to_string(),
            amount: coins(INITIAL_BALANCE, NATIVE_DENOM),
        }
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: MockApi::default().addr_make(OWNER).to_string(),
            amount: coins(INITIAL_BALANCE, NATIVE_DENOM),
        }
    }))
    .unwrap();

    app
}

pub fn setup_contracts(app: &mut App) -> (Addr, u64, u64) {
    let cw20_code_id = app.store_code(cw20_base_contract());
    let cw_vesting_code_id = app.store_code(cw_vesting_contract());

    // Instantiate cw20 contract with balances for Alice and Bob
    let cw20_addr = app
        .instantiate_contract(
            cw20_code_id,
            MockApi::default().addr_make(OWNER),
            &cw20_base::msg::InstantiateMsg {
                name: "cw20 token".to_string(),
                symbol: "cwtwenty".to_string(),
                decimals: 6,
                initial_balances: vec![
                    Cw20Coin {
                        address: MockApi::default().addr_make(ALICE).to_string(),
                        amount: Uint128::new(INITIAL_BALANCE),
                    },
                    Cw20Coin {
                        address: MockApi::default().addr_make(BOB).to_string(),
                        amount: Uint128::new(INITIAL_BALANCE),
                    },
                    Cw20Coin {
                        address: MockApi::default().addr_make(OWNER).to_string(),
                        amount: Uint128::new(INITIAL_BALANCE),
                    },
                ],
                mint: None,
                marketing: None,
            },
            &[],
            "cw20-base",
            None,
        )
        .unwrap();

    (cw20_addr, cw20_code_id, cw_vesting_code_id)
}

#[cfg(test)]
impl Default for InstantiateMsg {
    fn default() -> Self {
        Self {
            owner: Some(MockApi::default().addr_make(OWNER).to_string()),
            recipient: MockApi::default().addr_make(BOB).to_string(),
            title: "title".to_string(),
            description: Some("desc".to_string()),
            total: Uint128::new(TOTAL_VEST),
            // cw20 normally first contract instantaited
            denom: UncheckedDenom::Cw20("contract0".to_string()),
            schedule: Schedule::SaturatingLinear,
            start_time: None,
            vesting_duration_seconds: 604800,    // one week
            unbonding_duration_seconds: 2592000, // 30 days
        }
    }
}

struct TestCase {
    cw20_addr: Addr,
    cw_vesting_addr: Addr,
    recipient: Addr,
    vesting_payment: Vest,
}

fn setup_test_case(app: &mut App, msg: InstantiateMsg, funds: &[Coin]) -> TestCase {
    let (cw20_addr, _, cw_vesting_code_id) = setup_contracts(app);

    // Instantiate cw-vesting contract
    let mut msg = msg;
    if let UncheckedDenom::Cw20(s) = &mut msg.denom {
        if *s == "contract0" {
            *s = cw20_addr.to_string();
        }
    }
    let cw_vesting_addr = app
        .instantiate_contract(
            cw_vesting_code_id,
            MockApi::default().addr_make(OWNER),
            &msg,
            funds,
            "cw-vesting",
            None,
        )
        .unwrap();

    let vesting_payment = match msg.denom {
        UncheckedDenom::Cw20(ref cw20_addr) => {
            let msg = Cw20ExecuteMsg::Send {
                contract: cw_vesting_addr.to_string(),
                amount: msg.total,
                msg: to_json_binary(&ReceiveMsg::Fund {}).unwrap(),
            };
            app.execute_contract(
                MockApi::default().addr_make(OWNER),
                Addr::unchecked(cw20_addr.clone()),
                &msg,
                &[],
            )
            .unwrap();

            get_vesting_payment(app, cw_vesting_addr.clone())
        }
        UncheckedDenom::Native(_) => get_vesting_payment(app, cw_vesting_addr.clone()),
    };

    TestCase {
        cw20_addr,
        cw_vesting_addr,
        recipient: Addr::unchecked(msg.recipient),
        vesting_payment,
    }
}

#[test]
fn test_happy_cw20_path() {
    let mut app = setup_app();

    let TestCase {
        cw20_addr,
        cw_vesting_addr,
        recipient: bob,
        vesting_payment,
        ..
    } = setup_test_case(&mut app, InstantiateMsg::default(), &[]);

    // Check Vesting Payment was created correctly
    assert_eq!(vesting_payment.status, Status::Funded);
    assert_eq!(vesting_payment.claimed, Uint128::zero());
    assert_eq!(
        vesting_payment.vested(app.block_info().time),
        Uint128::zero()
    );

    // No time has passed, so nothing is withdrawable.
    let err: cw_vesting::ContractError = app
        .execute_contract(
            bob.clone(),
            cw_vesting_addr.clone(),
            &ExecuteMsg::Distribute { amount: None },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        cw_vesting::ContractError::InvalidWithdrawal {
            request: Uint128::zero(),
            claimable: Uint128::zero()
        }
    );

    // Advance the clock by 1/2 the vesting period.
    app.update_block(|block| {
        block.time = block.time.plus_seconds(604800 / 2);
    });

    // Distribute, expect to receive 50% of funds.
    app.execute_contract(
        bob,
        cw_vesting_addr,
        &ExecuteMsg::Distribute { amount: None },
        &[],
    )
    .unwrap();

    // Owner has funded the contract and down
    assert_eq!(
        get_balance_cw20(&app, cw20_addr.clone(), MockApi::default().addr_make(OWNER)),
        Uint128::new(INITIAL_BALANCE - TOTAL_VEST)
    );

    // Bob has claimed vested funds and is up
    assert_eq!(
        get_balance_cw20(&app, cw20_addr, MockApi::default().addr_make(BOB)),
        Uint128::new(INITIAL_BALANCE) + Uint128::new(TOTAL_VEST / 2)
    );
}

#[test]
fn test_happy_native_path() {
    let mut app = setup_app();

    let msg = InstantiateMsg {
        denom: UncheckedDenom::Native(NATIVE_DENOM.to_string()),
        ..Default::default()
    };

    let TestCase {
        cw_vesting_addr,
        recipient: bob,
        vesting_payment,
        ..
    } = setup_test_case(&mut app, msg, &coins(TOTAL_VEST, NATIVE_DENOM));

    assert_eq!(vesting_payment.status, Status::Funded);
    assert_eq!(vesting_payment.claimed, Uint128::zero());
    assert_eq!(
        vesting_payment.vested(app.block_info().time),
        Uint128::zero()
    );

    // No time has passed, so nothing is withdrawable.
    let err: cw_vesting::ContractError = app
        .execute_contract(
            bob.clone(),
            cw_vesting_addr.clone(),
            &ExecuteMsg::Distribute { amount: None },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        cw_vesting::ContractError::InvalidWithdrawal {
            request: Uint128::zero(),
            claimable: Uint128::zero()
        }
    );

    // Advance the clock by 1/2 the vesting period.
    app.update_block(|block| {
        block.time = block.time.plus_seconds(604800 / 2);
    });

    // Distribute, expect to receive 50% of funds.
    app.execute_contract(
        bob,
        cw_vesting_addr,
        &ExecuteMsg::Distribute { amount: None },
        &[],
    )
    .unwrap();

    // Owner has funded the contract and down 1000
    assert_eq!(
        get_balance_native(&app, MockApi::default().addr_make(OWNER), NATIVE_DENOM),
        Uint128::new(INITIAL_BALANCE - TOTAL_VEST)
    );
    // Bob has claimed vested funds and is up 250
    assert_eq!(
        get_balance_native(&app, MockApi::default().addr_make(BOB), NATIVE_DENOM),
        Uint128::new(INITIAL_BALANCE) + Uint128::new(TOTAL_VEST / 2)
    );
}

#[test]
fn test_staking_rewards_go_to_receiver() {
    let validator = Validator::create(
        "testvaloper1".to_string(),
        Decimal::percent(1),
        Decimal::percent(100),
        Decimal::percent(1),
    );

    let mut app = AppBuilder::default().build(|router, api, storage| {
        router
            .staking
            .setup(
                storage,
                StakingInfo {
                    bonded_denom: NATIVE_DENOM.to_string(),
                    unbonding_time: 60,
                    // Interest rate per year (60 * 60 * 24 * 365 seconds)
                    apr: Decimal::percent(10),
                },
            )
            .unwrap();
        router
            .staking
            .add_validator(api, storage, &mock_env().block, validator)
            .unwrap();
    });

    let vesting_id = app.store_code(cw_vesting_contract());
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: MockApi::default().addr_make(OWNER).to_string(),
        amount: coins(100, NATIVE_DENOM),
    }))
    .unwrap();

    let msg = InstantiateMsg {
        denom: UncheckedDenom::Native(NATIVE_DENOM.to_string()),
        total: Uint128::new(100),
        ..Default::default()
    };

    let vesting = app
        .instantiate_contract(
            vesting_id,
            MockApi::default().addr_make(OWNER),
            &msg,
            &coins(100, NATIVE_DENOM),
            "cw-vesting",
            None,
        )
        .unwrap();

    // delegate all of the tokens to the validaor.
    app.execute_contract(
        MockApi::default().addr_make(BOB),
        vesting.clone(),
        &ExecuteMsg::Delegate {
            validator: "testvaloper1".to_string(),
            amount: Uint128::new(100),
        },
        &[],
    )
    .unwrap();

    let balance = get_balance_native(&app, MockApi::default().addr_make(BOB), NATIVE_DENOM);
    assert_eq!(balance.u128(), 0);

    // A year passes.
    app.update_block(|block| block.time = block.time.plus_seconds(60 * 60 * 24 * 365));

    app.execute_contract(
        MockApi::default().addr_make(BOB),
        vesting,
        &ExecuteMsg::WithdrawDelegatorReward {
            validator: "testvaloper1".to_string(),
        },
        &[],
    )
    .unwrap();

    let balance = get_balance_native(&app, MockApi::default().addr_make(BOB), NATIVE_DENOM);
    assert_eq!(balance.u128(), 9); // 10% APY, 1% comission, 100 staked, one year elapsed.
}

#[test]
fn test_cancel_vesting() {
    let mut app = setup_app();

    let TestCase {
        cw20_addr,
        cw_vesting_addr,
        ..
    } = setup_test_case(&mut app, InstantiateMsg::default(), &[]);

    // Non-owner can't cancel
    let err: cw_vesting::ContractError = app
        .execute_contract(
            MockApi::default().addr_make(ALICE),
            cw_vesting_addr.clone(),
            &ExecuteMsg::Cancel {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        cw_vesting::ContractError::Ownable(cw_ownable::OwnershipError::NotOwner)
    );

    // Advance the clock by 1/2 the vesting period.
    app.update_block(|block| {
        block.time = block.time.plus_seconds(604800 / 2);
    });

    // Owner DAO cancels vesting contract. All tokens are liquid so
    // everything settles instantly.
    app.execute_contract(
        MockApi::default().addr_make(OWNER),
        cw_vesting_addr.clone(),
        &ExecuteMsg::Cancel {},
        &[],
    )
    .unwrap();

    // Can't distribute as tokens are already distributed.
    let err: cw_vesting::ContractError = app
        .execute_contract(
            MockApi::default().addr_make(BOB),
            cw_vesting_addr,
            &ExecuteMsg::Distribute { amount: None },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(
        err,
        cw_vesting::ContractError::InvalidWithdrawal { .. }
    ));

    // Unvested funds have been returned to contract owner
    assert_eq!(
        get_balance_cw20(
            &app,
            cw20_addr.to_string(),
            MockApi::default().addr_make(OWNER)
        ),
        Uint128::new(INITIAL_BALANCE - TOTAL_VEST / 2)
    );
    // Bob has gets the funds vest up until cancelation
    assert_eq!(
        get_balance_cw20(
            &app,
            cw20_addr.to_string(),
            MockApi::default().addr_make(BOB)
        ),
        Uint128::new(INITIAL_BALANCE + TOTAL_VEST / 2)
    );
}

#[test]
fn test_catch_imposter_cw20() {
    let mut app = setup_app();
    let (_, cw20_code_id, _) = setup_contracts(&mut app);

    let TestCase {
        cw20_addr,
        cw_vesting_addr,
        ..
    } = setup_test_case(&mut app, InstantiateMsg::default(), &[]);

    // Create imposter cw20
    let cw20_imposter_addr = app
        .instantiate_contract(
            cw20_code_id,
            MockApi::default().addr_make(OWNER),
            &cw20_base::msg::InstantiateMsg {
                name: "cw20 token".to_string(),
                symbol: "cwtwenty".to_string(),
                decimals: 6,
                initial_balances: vec![Cw20Coin {
                    address: MockApi::default().addr_make(OWNER).to_string(),
                    amount: Uint128::new(INITIAL_BALANCE),
                }],
                mint: None,
                marketing: None,
            },
            &[],
            "cw20-base",
            None,
        )
        .unwrap();

    let msg = Cw20ExecuteMsg::Send {
        contract: cw_vesting_addr.to_string(),
        amount: Uint128::new(TOTAL_VEST),
        msg: to_json_binary(&ReceiveMsg::Fund {}).unwrap(),
    };

    // Errors that cw20 does not match what was expected
    let error: cw_vesting::ContractError = app
        .execute_contract(
            MockApi::default().addr_make(OWNER),
            Addr::unchecked(cw20_imposter_addr),
            &msg,
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(error, cw_vesting::ContractError::WrongCw20);
}

#[test]
fn test_incorrect_native_funding_amount() {
    let mut app = setup_app();

    let unchecked_denom = UncheckedDenom::Native(NATIVE_DENOM.to_string());

    let msg = InstantiateMsg {
        denom: unchecked_denom,
        ..Default::default()
    };

    let alice = MockApi::default().addr_make(ALICE);

    let (_, _, cw_vesting_code_id) = setup_contracts(&mut app);

    // Instantiate cw-vesting contract errors with incorrect amount
    let error: cw_vesting::ContractError = app
        .instantiate_contract(
            cw_vesting_code_id,
            alice,
            &msg,
            &coins(100, NATIVE_DENOM),
            "cw-vesting",
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        error,
        cw_vesting::ContractError::WrongFundAmount {
            sent: Uint128::new(100),
            expected: Uint128::new(TOTAL_VEST)
        }
    )
}

/// should reject funding if the token is wrong, or the token amount is wrong.
#[test]
fn test_execution_rejection_recv() {
    let env = mock_env;
    let mut deps = mock_dependencies();

    PAYMENT
        .initialize(
            deps.as_mut().storage,
            VestInit {
                total: Uint128::new(100),
                schedule: Schedule::SaturatingLinear,
                start_time: env().block.time,
                duration_seconds: 60 * 60 * 24 * 7,
                denom: CheckedDenom::Cw20(MockApi::default().addr_make("cw20")),
                recipient: MockApi::default().addr_make("recipient"),
                title: "title".to_string(),
                description: Some("description".to_string()),
            },
        )
        .unwrap();
    let mut deps = deps.as_mut();
    cw_ownable::initialize_owner(
        deps.storage,
        deps.api,
        Some(MockApi::default().addr_make("owner").as_str()),
    )
    .unwrap();

    let err = execute_receive_cw20(
        env(),
        deps.branch(),
        message_info(&MockApi::default().addr_make("notcw20"), &[]),
        Cw20ReceiveMsg {
            sender: "random".to_string(),
            amount: Uint128::new(100),
            msg: to_json_binary(&ReceiveMsg::Fund {}).unwrap(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::WrongCw20);

    let err = execute_receive_cw20(
        env(),
        deps.branch(),
        message_info(&MockApi::default().addr_make("cw20"), &[]),
        Cw20ReceiveMsg {
            sender: "random".to_string(),
            amount: Uint128::new(101),
            msg: to_json_binary(&ReceiveMsg::Fund {}).unwrap(),
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::WrongFundAmount {
            sent: Uint128::new(101),
            expected: Uint128::new(100)
        }
    );
}

/// Should report zero distributable tokens when the contract is
/// unfunded.
#[test]
fn test_illiquid_when_unfunfed() {
    let env = mock_env;
    let mut deps = mock_dependencies();

    PAYMENT
        .initialize(
            deps.as_mut().storage,
            VestInit {
                total: Uint128::new(100),
                schedule: Schedule::SaturatingLinear,
                start_time: env().block.time,
                duration_seconds: 60 * 60 * 24 * 7,
                denom: CheckedDenom::Cw20(MockApi::default().addr_make("cw20")),
                recipient: MockApi::default().addr_make("recipient"),
                title: "title".to_string(),
                description: Some("description".to_string()),
            },
        )
        .unwrap();
    let deps = deps.as_mut();
    cw_ownable::initialize_owner(
        deps.storage,
        deps.api,
        Some(MockApi::default().addr_make("owner").as_str()),
    )
    .unwrap();

    // nothing is liquid in the unfunded state.
    assert_eq!(
        PAYMENT
            .distributable(
                deps.storage,
                &PAYMENT.get_vest(deps.storage).unwrap(),
                env().block.time
            )
            .unwrap(),
        Uint128::zero()
    );
}

/// Ownership can not be renounced while the contract is canceled and
/// there are funds withdrawable by the owner as this would lock those
/// funds.
#[test]
fn test_update_owner() {
    let env = mock_env;
    let mut deps = mock_dependencies();
    PAYMENT
        .initialize(
            deps.as_mut().storage,
            VestInit {
                total: Uint128::new(100),
                schedule: Schedule::SaturatingLinear,
                start_time: env().block.time,
                duration_seconds: 60 * 60 * 24 * 7,
                denom: CheckedDenom::Cw20(MockApi::default().addr_make("cw20")),
                recipient: MockApi::default().addr_make("recipient"),
                title: "title".to_string(),
                description: Some("description".to_string()),
            },
        )
        .unwrap();
    let deps = deps.as_mut();
    cw_ownable::initialize_owner(
        deps.storage,
        deps.api,
        Some(MockApi::default().addr_make("owner").as_str()),
    )
    .unwrap();
    PAYMENT
        .on_delegate(
            deps.storage,
            env().block.time,
            "validator".to_string(),
            Uint128::new(10),
        )
        .unwrap();
    PAYMENT
        .cancel(
            deps.storage,
            env().block.time,
            &MockApi::default().addr_make("owner"),
        )
        .unwrap();
    let err = execute(
        deps,
        env(),
        message_info(&MockApi::default().addr_make("owner"), &[]),
        ExecuteMsg::UpdateOwnership(Action::RenounceOwnership),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Cancelled);
}

#[test]
#[should_panic(expected = "can not vest a constant amount, specifiy two or more points")]
fn test_constant_piecewise_not_allowed() {
    let mut app = setup_app();
    let instantiate = InstantiateMsg {
        schedule: Schedule::PiecewiseLinear(vec![(1, Uint128::new(10))]),
        ..Default::default()
    };

    setup_test_case(&mut app, instantiate, &[]);
}
