use crate::{msg::SubmissionResponse, ContractError};

use super::suite::SuiteBuilder;

use cosmwasm_std::{coin, Addr, Uint128};
use cosmwasm_std::testing::MockApi;

#[test]
fn create_default_submission() {
    let api = MockApi::default();
    let community_pool = api.addr_make("community_pool").to_string();

    let suite = SuiteBuilder::new()
        .with_treasury("community_pool")
        .build();

    // this one is created by default during instantiation
    assert_eq!(
        SubmissionResponse {
            sender: suite.gauge_adapter.clone(),
            name: "Unimpressed".to_owned(),
            url: "Those funds go back to the community pool".to_owned(),
            address: Addr::unchecked(community_pool.clone()),
        },
        suite.query_submission(community_pool.clone()).unwrap()
    )
}

#[test]
fn create_submission_no_required_deposit() {
    let api = MockApi::default();
    let recipient = api.addr_make("user").to_string();

    let mut suite = SuiteBuilder::new()
        .with_funds("owner", &[coin(100_000, "juno")])
        .build();

    // Fails send funds along with the tx.
    let err = suite
        .execute_create_submission(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            &[coin(1_000, "juno")],
        )
        .unwrap_err();

    assert_eq!(
        ContractError::InvalidDepositAmount {
            correct_amount: Uint128::zero().to_string()
        },
        err.downcast().unwrap()
    );

    // Valid submission.
    _ = suite
        .execute_create_submission(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            &[],
        )
        .unwrap();

    assert_eq!(
        SubmissionResponse {
            sender: suite.owner.clone(),
            name: "WYNDers".to_owned(),
            url: "https://www.wynddao.com/".to_owned(),
            address: Addr::unchecked(recipient.clone()),
        },
        suite.query_submission(recipient).unwrap()
    )
}

#[test]
fn overwrite_existing_submission() {
    let api = MockApi::default();
    let recipient = api.addr_make("user").to_string();
    let anotheruser = api.addr_make("anotheruser");

    let mut suite = SuiteBuilder::new()
        .with_funds("owner", &[coin(100_000, "juno")])
        .build();

    suite
        .execute_create_submission(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            &[],
        )
        .unwrap();

    assert_eq!(
        SubmissionResponse {
            sender: suite.owner.clone(),
            name: "WYNDers".to_owned(),
            url: "https://www.wynddao.com/".to_owned(),
            address: Addr::unchecked(recipient.clone()),
        },
        suite.query_submission(recipient.clone()).unwrap()
    );

    // Try to submit to the same address with different user
    let err = suite
        .execute_create_submission(
            anotheruser.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            &[],
        )
        .unwrap_err();

    assert_eq!(
        ContractError::UnauthorizedSubmission {},
        err.downcast().unwrap()
    );

    // Overwriting submission as same author works
    let err = suite
        .execute_create_submission(
            anotheruser.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            &[],
        )
        .unwrap_err();
    assert_eq!(
        ContractError::UnauthorizedSubmission {},
        err.downcast().unwrap()
    );

    suite
        .execute_create_submission(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "wynddao".to_owned(),
            recipient.clone(),
            &[],
        )
        .unwrap();

    let response = suite.query_submission(recipient).unwrap();
    assert_eq!(response.url, "wynddao".to_owned());
}

#[test]
fn create_submission_required_deposit() {
    let api = MockApi::default();
    let recipient = api.addr_make("user").to_string();

    let mut suite = SuiteBuilder::new()
        .with_funds("owner", &[coin(100_000, "juno"), coin(100_000, "wynd")])
        .with_native_deposit(1_000)
        .build();

    // Fails if no funds sent.
    let err = suite
        .execute_create_submission(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            &[],
        )
        .unwrap_err();

    assert_eq!(
        ContractError::PaymentError(cw_utils::PaymentError::NoFunds {}),
        err.downcast().unwrap()
    );

    // Fails if correct denom but not enough amount.
    let err = suite
        .execute_create_submission(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            &[coin(1, "juno")],
        )
        .unwrap_err();

    assert_eq!(
        ContractError::InvalidDepositAmount {
            correct_amount: Uint128::new(1_000).to_string()
        },
        err.downcast().unwrap()
    );

    // Fails if enough amount but incorrect denom.
    let err = suite
        .execute_create_submission(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            &[coin(1_000, "wynd")],
        )
        .unwrap_err();

    assert_eq!(
        ContractError::InvalidDepositType {},
        err.downcast().unwrap()
    );

    // Valid submission.
    _ = suite
        .execute_create_submission(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            &[coin(1_000, "juno")],
        )
        .unwrap();

    assert_eq!(
        SubmissionResponse {
            sender: suite.owner.clone(),
            name: "WYNDers".to_owned(),
            url: "https://www.wynddao.com/".to_owned(),
            address: Addr::unchecked(recipient.clone()),
        },
        suite.query_submission(recipient).unwrap()
    )
}

#[test]
fn create_receive_required_deposit() {
    let api = MockApi::default();
    let recipient = api.addr_make("user").to_string();

    let mut suite = SuiteBuilder::new()
        .with_funds("owner", &[coin(100_000, "juno")])
        .with_cw20_funds("owner", 1_000)
        .with_cw20_deposit(1_000)
        .build();

    // Fails by sending wrong cw20.
    let owner_str = suite.owner.to_string();
    let cw20_addr = suite.instantiate_token(&owner_str, "WRONGTOKEN", 1_000);
    let err = suite
        .execute_receive_through_cw20(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            1_000,
            cw20_addr,
        )
        .unwrap_err();

    assert_eq!(
        ContractError::InvalidDepositType {},
        err.downcast().unwrap(),
    );

    // Fails by sending less tokens than required.
    let err = suite
        .execute_receive_through_cw20(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            1,
            suite.default_cw20.clone(),
        )
        .unwrap_err();

    assert_eq!(
        ContractError::InvalidDepositAmount {
            correct_amount: Uint128::new(1_000).to_string()
        },
        err.downcast().unwrap()
    );

    // Valid submission.
    _ = suite
        .execute_receive_through_cw20(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            1_000,
            suite.default_cw20.clone(),
        )
        .unwrap();

    assert_eq!(
        SubmissionResponse {
            sender: suite.owner.clone(),
            name: "WYNDers".to_owned(),
            url: "https://www.wynddao.com/".to_owned(),
            address: Addr::unchecked(recipient.clone()),
        },
        suite.query_submission(recipient).unwrap()
    );

    assert_eq!(2, suite.query_submissions().unwrap().len())
}

#[test]
fn return_deposits_no_required_deposit() {
    let mut suite = SuiteBuilder::new()
        .with_funds("owner", &[coin(100_000, "juno")])
        .build();

    let err = suite
        .execute_return_deposit(suite.owner.clone().as_ref())
        .unwrap_err();

    assert_eq!(ContractError::NoDepositToRefund {}, err.downcast().unwrap())
}

#[test]
fn return_deposits_no_admin() {
    let api = MockApi::default();
    let einstein = api.addr_make("einstein").to_string();

    let mut suite = SuiteBuilder::new()
        .with_funds("owner", &[coin(100_000, "juno")])
        .with_native_deposit(1_000)
        .build();

    let err = suite.execute_return_deposit(&einstein).unwrap_err();

    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap())
}

#[test]
fn return_deposits_required_native_deposit() {
    let api = MockApi::default();
    let recipient = api.addr_make("user").to_string();

    let mut suite = SuiteBuilder::new()
        .with_funds("owner", &[coin(1_000, "juno")])
        .with_native_deposit(1_000)
        .build();

    // Valid submission.
    _ = suite
        .execute_create_submission(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            &[coin(1_000, "juno")],
        )
        .unwrap();

    assert_eq!(
        suite.query_native_balance(suite.owner.as_ref()).unwrap(),
        0u128,
    );
    assert_eq!(suite.query_native_balance(&recipient).unwrap(), 0u128,);
    assert_eq!(
        suite
            .query_native_balance(suite.gauge_adapter.as_ref())
            .unwrap(),
        1_000u128,
    );

    _ = suite
        .execute_return_deposit(suite.owner.clone().as_ref())
        .unwrap();

    assert_eq!(
        suite.query_native_balance(suite.owner.as_ref()).unwrap(),
        1_000u128,
    );
    assert_eq!(suite.query_native_balance(&recipient).unwrap(), 0u128,);
    assert_eq!(
        suite
            .query_native_balance(suite.gauge_adapter.as_ref())
            .unwrap(),
        0u128,
    );
}

#[test]
fn return_deposits_required_native_deposit_multiple_deposits() {
    let api = MockApi::default();
    let einstein = api.addr_make("einstein");
    let recipient = api.addr_make("user").to_string();

    let mut suite = SuiteBuilder::new()
        .with_funds("owner", &[coin(1_000, "juno")])
        .with_funds("einstein", &[coin(1_000, "juno")])
        .with_native_deposit(1_000)
        .build();

    // Valid submission.
    _ = suite
        .execute_create_submission(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            &[coin(1_000, "juno")],
        )
        .unwrap();

    // Valid submission.
    _ = suite
        .execute_create_submission(
            einstein.clone(),
            "MIBers".to_owned(),
            "https://www.mib.tech/".to_owned(),
            einstein.to_string(),
            &[coin(1_000, "juno")],
        )
        .unwrap();

    _ = suite
        .execute_return_deposit(suite.owner.clone().as_ref())
        .unwrap();

    assert_eq!(
        suite.query_native_balance(suite.owner.as_ref()).unwrap(),
        1_000u128,
    );
    assert_eq!(suite.query_native_balance(einstein.as_ref()).unwrap(), 1_000u128,);
    assert_eq!(suite.query_native_balance(&recipient).unwrap(), 0u128,);
    assert_eq!(
        suite
            .query_native_balance(suite.gauge_adapter.as_ref())
            .unwrap(),
        0u128,
    );
}

#[test]
fn return_deposits_required_cw20_deposit() {
    let api = MockApi::default();
    let recipient = api.addr_make("user").to_string();

    let mut suite = SuiteBuilder::new()
        .with_funds("owner", &[coin(1_000, "juno")])
        .with_cw20_funds("owner", 1_000)
        .with_cw20_deposit(1_000)
        .build();

    // Valid submission.
    _ = suite
        .execute_receive_through_cw20(
            suite.owner.clone(),
            "WYNDers".to_owned(),
            "https://www.wynddao.com/".to_owned(),
            recipient.clone(),
            1_000,
            suite.default_cw20.clone(),
        )
        .unwrap();

    assert_eq!(
        suite
            .query_cw20_balance(suite.owner.as_ref(), &suite.default_cw20)
            .unwrap(),
        0u128,
    );
    assert_eq!(
        suite
            .query_cw20_balance(&recipient, &suite.default_cw20)
            .unwrap(),
        0u128,
    );
    assert_eq!(
        suite
            .query_cw20_balance(suite.gauge_adapter.as_ref(), &suite.default_cw20)
            .unwrap(),
        1_000u128,
    );

    _ = suite
        .execute_return_deposit(suite.owner.clone().as_ref())
        .unwrap();

    assert_eq!(
        suite
            .query_cw20_balance(suite.owner.as_ref(), &suite.default_cw20)
            .unwrap(),
        1_000u128,
    );
    // Tokens are sent back to the address specified in the sumbission.
    assert_eq!(
        suite
            .query_cw20_balance(&recipient, &suite.default_cw20)
            .unwrap(),
        0u128,
    );
    assert_eq!(
        suite
            .query_cw20_balance(suite.gauge_adapter.as_ref(), &suite.default_cw20)
            .unwrap(),
        0u128,
    );
}
