//! E2E: Foundation-style native delegation + FIAB-SUBDAO in one proposal.
//! Uses the same propose → vote yes → execute helpers as the rest of dao-testing
//! (pre-propose-single → proposal-single).
//!
//! Runnable twin (dao-contracts workspace currently fails rayon 1.10 vs 1.12
//! resolution via cw-orch-core zk): `scripts/ibc-ops/cw-orch-ibc` `cargo test --offline`.

use cosmwasm_std::{
    testing::mock_env, to_json_binary, Coin, CosmosMsg, Decimal, Uint256, Validator, WasmMsg,
};
use cw4::Member;

use crate::suite::modules::native_delegation::{
    instantiate_subdao_msg, NativeDelegationPlan, terp_100k, FIAB_DESCRIPTION, FIAB_NAME,
    FOUNDATION_TITLE, GAS_DENOM,
};
use crate::suite::{DaoTestingSuite, DaoTestingSuiteBase, ADDR0};

const VAL2: &str = "valoper2";
const VAL3: &str = "valoper3";
/// cw-multi-test default bonded denom.
const STAKE: &str = "TOKEN";

fn add_validators(suite: &mut DaoTestingSuiteBase) {
    suite.app.init_modules(|router, api, storage| {
        let block = mock_env().block;
        for addr in [VAL2, VAL3] {
            router
                .staking
                .add_validator(
                    api,
                    storage,
                    &block,
                    Validator::create(
                        addr.to_string(),
                        Decimal::percent(5),
                        Decimal::percent(20),
                        Decimal::percent(1),
                    ),
                )
                .unwrap();
        }
    });
}

fn fiab_instantiate(admin: String, core_id: u64, voting: dao_interface::state::ModuleInstantiateInfo, proposal: dao_interface::state::ModuleInstantiateInfo) -> CosmosMsg {
    instantiate_subdao_msg(
        admin.clone(),
        core_id,
        FIAB_NAME,
        to_json_binary(&dao_interface::msg::InstantiateMsg {
            admin: Some(admin),
            name: FIAB_NAME.into(),
            description: FIAB_DESCRIPTION.into(),
            image_url: None,
            automatically_add_cw20s: true,
            automatically_add_cw721s: true,
            voting_module_instantiate_info: voting,
            proposal_modules_instantiate_info: vec![proposal],
            initial_items: None,
            initial_actions: None,
            dao_uri: None,
        })
        .unwrap()
        .to_vec(),
        vec![
            Coin::new(1_000_000u128, STAKE),
            Coin::new(1_000_000u128, GAS_DENOM),
        ],
    )
}

#[test]
fn foundation_delegation_and_fiab_one_proposal_votes_yes_and_executes() {
    let mut base = DaoTestingSuiteBase::base();
    add_validators(&mut base);
    let mut suite = base.cw4();
    suite.with_members(vec![Member {
        addr: ADDR0.to_string(),
        weight: 1,
    }]);
    let core_id = suite.core_id;
    let proposal_single_id = suite.proposal_single_id;
    let dao = suite.dao();
    suite.advance_blocks(2);

    let mut plan = NativeDelegationPlan::bonded_set(vec![VAL2.into(), VAL3.into()]);
    plan.stake_denom = STAKE.into();
    plan.other_validators = vec![VAL2.into(), VAL3.into()];

    let stake_needed = terp_100k() + terp_100k() + Uint256::from(1_000_000u128);
    suite.mint(&dao.core_addr, stake_needed, STAKE);
    suite.mint(&dao.core_addr, Uint256::from(1_000_000u128), GAS_DENOM);

    let prop_info = dao_interface::state::ModuleInstantiateInfo {
        code_id: proposal_single_id,
        msg: to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
            threshold: dao_voting::threshold::Threshold::AbsoluteCount {
                threshold: Uint256::from(1u64),
            },
            max_voting_period: cw_utils::Duration::Time(86400),
            min_voting_period: None,
            only_members_execute: true,
            allow_revoting: false,
            pre_propose_info: dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
            close_proposal_on_execution_failure: true,
            veto: None,
            delegation_module: None,
        })
        .unwrap(),
        admin: Some(dao_interface::state::Admin::CoreModule {}),
        funds: None,
        label: "fiab-proposal".into(),
        salt: None,
    };
    let fiab = fiab_instantiate(
        dao.core_addr.to_string(),
        core_id,
        suite.get_voting_module_info(),
        prop_info,
    );

    let msgs = plan.with_extra(vec![fiab]);
    assert!(msgs.iter().any(|m| matches!(m, CosmosMsg::Staking(_))));
    assert!(!msgs.iter().any(|m| matches!(m, CosmosMsg::Bank(_))));
    assert!(msgs.iter().any(|m| matches!(
        m,
        CosmosMsg::Wasm(WasmMsg::Instantiate { label, .. }) if label == FIAB_NAME
    )));

    let (_pmod, pid, proposal) = suite.propose_single_choice(&dao, ADDR0, FOUNDATION_TITLE, msgs);
    assert_eq!(proposal.title, FOUNDATION_TITLE);

    suite.vote_single_choice(&dao, ADDR0, pid, dao_voting::voting::Vote::Yes);
    let resp = suite.execute_single_choice(&dao, ADDR0, pid);

    let dels = suite
        .app
        .wrap()
        .query_all_delegations(&dao.core_addr)
        .unwrap();
    let amt = |v: &str| {
        dels.iter()
            .find(|d| d.validator == v)
            .map(|d| d.amount.amount)
            .unwrap_or_default()
    };
    assert_eq!(amt(VAL2), terp_100k());
    assert_eq!(amt(VAL3), terp_100k());
    assert_eq!(dels.len(), 2);

    let cfg = resp
        .events
        .iter()
        .flat_map(|e| e.attributes.iter())
        .filter(|a| a.key == "_contract_address")
        .map(|a| a.value.clone())
        .find_map(|addr| {
            suite
                .querier()
                .query_wasm_smart::<dao_interface::state::Config>(
                    addr,
                    &dao_interface::msg::QueryMsg::Config {},
                )
                .ok()
                .filter(|c| c.name == FIAB_NAME)
        })
        .expect("FIAB-SUBDAO instantiated");
    assert_eq!(cfg.name, FIAB_NAME);
}
