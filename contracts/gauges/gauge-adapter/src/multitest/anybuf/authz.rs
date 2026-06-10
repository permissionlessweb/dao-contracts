use anybuf::Anybuf;
use cosmwasm_std::{coin, coins, to_json_binary};
use cw_denom::UncheckedDenom;
use cosmwasm_std::testing::MockApi;

use crate::{
    msg::{
        AdapterAuthzMsg, AdapterStakingMsg, AllOptionsResponse, AssetUnchecked,
        CheckOptionResponse, PossibleMsg, StargateWire, SubmissionMsg,
    },
    multitest::suite::{native_submission_helper, setup_gauge_adapter, OWNER, TREASURY},
};

#[test]
fn test_authz_anybuf_assertions() {
    let api = MockApi::default();
    let einstein = api.addr_make("einstein").to_string();
    let treasury = api.addr_make(TREASURY).to_string();
    let owner = api.addr_make(OWNER).to_string();

    let mut gauge = setup_gauge_adapter(
        Some(AssetUnchecked {
            denom: UncheckedDenom::Native("juno".into()),
            amount: 1_000u128.into(),
        }),
        Some(vec![
            PossibleMsg {
                stargate: StargateWire::Authz(AdapterAuthzMsg::MsgGrant()),
                max_amount: None,
            },
            PossibleMsg {
                stargate: StargateWire::Authz(AdapterAuthzMsg::MsgRevoke()),
                max_amount: None,
            },
            PossibleMsg {
                stargate: StargateWire::Authz(AdapterAuthzMsg::MsgExec()),
                max_amount: None,
            },
        ]),
    );

    // verify there are 5 possible messages
    assert_eq!(gauge.available_messages().unwrap().len(), 5);

    // submit invalid submission msg (wrong message type).
    native_submission_helper(
        &mut gauge,
        OWNER,
        OWNER,
        Some(coin(1_000u128, "juno")),
        SubmissionMsg {
            stargate: StargateWire::Staking(AdapterStakingMsg::MsgDelegate()),
            msg: to_json_binary(
                &Anybuf::new()
                    .append_string(1, owner.clone())
                    .append_string(2, "validator".to_string())
                    .append_repeated_message(
                        3,
                        &vec![&Anybuf::new()
                            .append_string(1, "juno".to_string())
                            .append_string(2, "1000".to_string())],
                    )
                    .into_vec(),
            )
            .unwrap(),
        },
    )
    .unwrap_err();

    // give einstein a balance
    gauge.give_balance("einstein", coins(1_000u128, "juno"));

    // good authz grant submission
    native_submission_helper(
        &mut gauge,
        "einstein",
        "einstein",
        Some(coin(1_000u128, "juno")),
        SubmissionMsg {
            stargate: crate::msg::StargateWire::Authz(AdapterAuthzMsg::MsgGrant()),
            msg: to_json_binary(
                &Anybuf::new()
                    .append_string(1, einstein.clone())
                    .append_string(2, owner.clone())
                    .into_vec(),
            )
            .unwrap(),
        },
    )
    .unwrap();

    let options: AllOptionsResponse = gauge.query_all_options().unwrap();

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
