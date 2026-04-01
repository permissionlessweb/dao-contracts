use cosmwasm_std::Uint256;
use cosmwasm_std::testing::MockApi;
use dao_voting::voting::Vote;

use super::suite::SuiteBuilder;

#[test]
fn multiple_options_one_gauge() {
    let api = MockApi::default();
    let voter1 = api.addr_make("voter1").to_string();
    let voter2 = api.addr_make("voter2").to_string();
    let voter3 = api.addr_make("voter3").to_string();
    let voter4 = api.addr_make("voter4").to_string();
    let voter5 = api.addr_make("voter5").to_string();
    let reward_to_distribute = (1000, "ujuno");
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[
            (voter1.as_str(), 600), // to have majority...
            (voter2.as_str(), 120),
            (voter3.as_str(), 130),
            (voter4.as_str(), 140),
            (voter5.as_str(), 150),
        ])
        .with_core_balance(reward_to_distribute)
        .build();

    suite.next_block();
    suite
        .propose_update_proposal_module(voter1.clone(), None)
        .unwrap();

    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1.as_str(), proposal, Vote::Yes)
        .unwrap();

    suite.next_block();
    suite
        .execute_single_proposal(voter1.clone(), proposal)
        .unwrap();
    let gauge_contract = suite.find_gauge_module();

    suite
        .instantiate_adapter_and_create_gauge(
            gauge_contract.clone(),
            &["option1", "option2", "option3", "option4", "option5"],
            reward_to_distribute,
            None,
            None,
            None,
        )
        .unwrap();
    let gauge_id = 0;

    suite
        .place_vote(
            &gauge_contract,
            voter1.clone(),
            gauge_id,
            Some("option1".into()),
        )
        .unwrap();
    suite
        .place_vote(
            &gauge_contract,
            voter2.clone(),
            gauge_id,
            Some("option2".into()),
        )
        .unwrap();
    suite
        .place_vote(
            &gauge_contract,
            voter3.clone(),
            gauge_id,
            Some("option3".into()),
        )
        .unwrap();
    suite
        .place_vote(
            &gauge_contract,
            voter4.clone(),
            gauge_id,
            Some("option4".into()),
        )
        .unwrap();
    suite
        .place_vote(
            &gauge_contract,
            voter5.clone(),
            gauge_id,
            Some("option5".into()),
        )
        .unwrap();

    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    assert_eq!(
        selected_set,
        vec![
            ("option1".to_owned(), Uint256::new(600)),
            ("option5".to_owned(), Uint256::new(150)),
            ("option4".to_owned(), Uint256::new(140)),
            ("option3".to_owned(), Uint256::new(130)),
            ("option2".to_owned(), Uint256::new(120))
        ]
    );

    suite
        .place_vote(
            &gauge_contract,
            voter1.clone(),
            gauge_id,
            Some("option2".into()),
        )
        .unwrap();

    let selected_set = suite.query_selected_set(&gauge_contract, gauge_id).unwrap();
    assert_eq!(
        selected_set,
        vec![
            ("option2".to_owned(), Uint256::new(720)),
            ("option5".to_owned(), Uint256::new(150)),
            ("option4".to_owned(), Uint256::new(140)),
            ("option3".to_owned(), Uint256::new(130)),
        ]
    );
}

/// create one in instantiate, other later via create
#[test]
fn multiple_options_two_gauges() {
    let api = MockApi::default();
    let voter1 = api.addr_make("voter1").to_string();
    let voter2 = api.addr_make("voter2").to_string();
    let voter3 = api.addr_make("voter3").to_string();
    let voter4 = api.addr_make("voter4").to_string();
    let voter5 = api.addr_make("voter5").to_string();
    let reward_to_distribute = (1000, "ujuno");
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[
            (voter1.as_str(), 600), // to have majority
            (voter2.as_str(), 120),
            (voter3.as_str(), 130),
            (voter4.as_str(), 140),
            (voter5.as_str(), 150),
        ])
        .with_core_balance(reward_to_distribute)
        .build();

    suite.next_block();
    let gauge_config = suite
        .instantiate_adapter_and_return_config(
            &["option1", "option2"],
            reward_to_distribute,
            None,
            None,
            None,
        )
        .unwrap();
    suite
        .propose_update_proposal_module(voter1.clone(), vec![gauge_config])
        .unwrap();

    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1.as_str(), proposal, Vote::Yes)
        .unwrap();

    suite.next_block();
    suite
        .execute_single_proposal(voter1.clone(), proposal)
        .unwrap();
    let gauge_contract = suite.find_gauge_module();

    let first_gauge_id = 0;
    suite
        .instantiate_adapter_and_create_gauge(
            gauge_contract.clone(),
            &["option3", "option4", "option5"],
            reward_to_distribute,
            None,
            None,
            None,
        )
        .unwrap();
    let second_gauge_id = 1;

    suite
        .place_vote(
            &gauge_contract,
            voter1.clone(),
            first_gauge_id,
            Some("option2".into()),
        )
        .unwrap();
    suite
        .place_vote(
            &gauge_contract,
            voter2.clone(),
            first_gauge_id,
            Some("option2".into()),
        )
        .unwrap();
    suite
        .place_vote(
            &gauge_contract,
            voter3.clone(),
            second_gauge_id,
            Some("option3".into()),
        )
        .unwrap();
    suite
        .place_vote(
            &gauge_contract,
            voter4.clone(),
            second_gauge_id,
            Some("option5".into()),
        )
        .unwrap();
    suite
        .place_vote(
            &gauge_contract,
            voter5.clone(),
            second_gauge_id,
            Some("option5".into()),
        )
        .unwrap();

    let selected_set = suite
        .query_selected_set(&gauge_contract, first_gauge_id)
        .unwrap();
    assert_eq!(
        selected_set,
        vec![("option2".to_owned(), Uint256::new(720))]
    );

    let selected_set = suite
        .query_selected_set(&gauge_contract, second_gauge_id)
        .unwrap();
    assert_eq!(
        selected_set,
        vec![
            ("option5".to_owned(), Uint256::new(290)),
            ("option3".to_owned(), Uint256::new(130)),
        ]
    );
}

#[test]
fn not_voted_options_are_not_selected() {
    let api = MockApi::default();
    let voter1 = api.addr_make("voter1").to_string();
    let voter2 = api.addr_make("voter2").to_string();
    let reward_to_distribute = (1000, "ujuno");
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[
            (voter1.as_str(), 600), // to have majority
            (voter2.as_str(), 120),
        ])
        .with_core_balance(reward_to_distribute)
        .build();

    suite.next_block();
    suite
        .propose_update_proposal_module(voter1.clone(), None)
        .unwrap();

    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1.as_str(), proposal, Vote::Yes)
        .unwrap();

    suite.next_block();
    suite
        .execute_single_proposal(voter1.clone(), proposal)
        .unwrap();
    let gauge_contract = suite.find_gauge_module();

    suite
        .instantiate_adapter_and_create_gauge(
            gauge_contract.clone(),
            &["option1", "option2", "option3", "option4"],
            reward_to_distribute,
            None,
            None,
            None,
        )
        .unwrap();
    let first_gauge_id = 0;

    suite
        .place_vote(
            &gauge_contract,
            voter1.clone(),
            first_gauge_id,
            Some("option1".into()),
        )
        .unwrap();
    suite
        .place_vote(
            &gauge_contract,
            voter2.clone(),
            first_gauge_id,
            Some("option2".into()),
        )
        .unwrap();

    let selected_set = suite
        .query_selected_set(&gauge_contract, first_gauge_id)
        .unwrap();
    assert_eq!(
        selected_set,
        vec![
            ("option1".to_owned(), Uint256::new(600)),
            ("option2".to_owned(), Uint256::new(120)),
        ]
    );

    // first voter changes vote to option2
    suite
        .place_vote(
            &gauge_contract,
            voter1.clone(),
            first_gauge_id,
            Some("option2".into()),
        )
        .unwrap();
    let selected_set = suite
        .query_selected_set(&gauge_contract, first_gauge_id)
        .unwrap();
    assert_eq!(
        selected_set,
        vec![("option2".to_owned(), Uint256::new(720)),]
    );
}
