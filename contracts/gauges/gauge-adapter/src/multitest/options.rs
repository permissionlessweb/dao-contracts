use cosmwasm_std::coin;
use cosmwasm_std::testing::MockApi;

use crate::multitest::suite::SuiteBuilder;

#[test]
fn option_queries() {
    let api = MockApi::default();
    let community_pool = api.addr_make("community_pool").to_string();
    let einstein = api.addr_make("einstein");
    let recipient = api.addr_make("user").to_string();

    let mut suite = SuiteBuilder::new()
        .with_treasury("community_pool")
        .with_funds("owner", &[coin(100_000, "juno")])
        .with_funds("einstein", &[coin(100_000, "juno")])
        .with_native_deposit(1_000)
        .build();

    let options = suite.query_all_options().unwrap();
    // account for a default option
    assert_eq!(options.len(), 1);

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
    suite
        .execute_create_submission(
            einstein.clone(),
            "MIBers".to_owned(),
            "https://www.mib.tech/".to_owned(),
            einstein.to_string(),
            &[coin(1_000, "juno")],
        )
        .unwrap();

    let mut options = suite.query_all_options().unwrap();
    options.sort();
    let mut expected = vec![community_pool.clone(), einstein.to_string(), recipient.clone()];
    expected.sort();
    assert_eq!(options, expected);

    let option = suite.query_check_option(einstein.to_string()).unwrap();
    assert!(option);

    let option = suite.query_check_option(api.addr_make("newton").to_string()).unwrap();
    assert!(!option);
}
