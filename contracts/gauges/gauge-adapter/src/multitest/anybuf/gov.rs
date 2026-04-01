use anybuf::Anybuf;
use cosmwasm_std::{coin, coins, to_json_binary, BankMsg, Uint256};
use cw_denom::UncheckedDenom;
use cosmwasm_std::testing::MockApi;

use crate::{
    msg::{
        AdapterAuthzMsg, AdapterGovMsg, AllOptionsResponse, AssetUnchecked,
        CheckOptionResponse, PossibleMsg, StargateWire, SubmissionMsg,
    },
    multitest::suite::{native_submission_helper, setup_gauge_adapter, OWNER, TREASURY},
};

#[test]
fn test_gov_anybuf_assertions() {
    let api = MockApi::default();
    let einstein = api.addr_make("einstein").to_string();
    let treasury = api.addr_make(TREASURY).to_string();
    let owner = api.addr_make(OWNER).to_string();

    let mut gauge = setup_gauge_adapter(
        Some(AssetUnchecked {
            denom: UncheckedDenom::Native("juno".into()),
            amount: 1_000u128.into(),
        }),
        Some(vec![PossibleMsg {
            stargate: StargateWire::Gov(AdapterGovMsg::MsgSendGovProp()),
            max_amount: Some(Uint256::from(1_000u128)),
        }]),
    );

    // verify there are 3 possible messages
    assert_eq!(gauge.available_messages().unwrap().len(), 3);

    // submit invalid submission msg.
    native_submission_helper(
        &mut gauge,
        OWNER,
        OWNER,
        Some(coin(1_000u128, "juno")),
        SubmissionMsg {
            stargate: StargateWire::Authz(AdapterAuthzMsg::MsgGrant()),
            msg: to_json_binary(
                &Anybuf::new()
                    .append_string(1, owner.clone())
                    .append_string(2, "grantee".to_string())
                    .into_vec(),
            )
            .unwrap(),
        },
    )
    .unwrap_err();

    // give einstein a balance
    gauge.give_balance("einstein", coins(1_000u128, "juno"));

    // good gov proposal submission
    native_submission_helper(
        &mut gauge,
        "einstein",
        "einstein",
        Some(coin(1_000u128, "juno")),
        SubmissionMsg {
            stargate: crate::msg::StargateWire::Gov(AdapterGovMsg::MsgSendGovProp()),
            msg: to_json_binary(
                &Anybuf::new()
                    .append_repeated_bytes(
                        1,
                        &vec![&to_json_binary(&BankMsg::Send {
                            to_address: einstein.clone(),
                            amount: coins(1_000_000u128, "juno"),
                        })
                        .unwrap()],
                    )
                    .append_repeated_bytes(2, &vec![&Anybuf::new().into_vec()])
                    .append_string(3, einstein.clone())
                    .append_string(4, "metadata".to_string())
                    .append_string(5, "title".to_string())
                    .append_string(6, "summary".to_string())
                    .into_vec(),
            )
            .unwrap(),
        },
    )
    .unwrap();

    let options: AllOptionsResponse = gauge.query_all_options().unwrap();

    println!("{:#?}", options);
    assert_eq!(
        options,
        AllOptionsResponse {
            options: vec![einstein.clone(), treasury.clone()]
        },
    );

    let option: CheckOptionResponse = gauge.query_check_option(einstein.clone()).unwrap();
    assert!(option.valid);

    let option: CheckOptionResponse = gauge.query_check_option(owner.clone()).unwrap();
    assert!(!option.valid);
}
