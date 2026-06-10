use cosmwasm_std::{
    from_json,
    testing::{message_info, mock_dependencies, mock_env, MockApi},
    to_json_binary, Addr, Binary, ContractResult, Empty, Response, SubMsg, WasmMsg,
};

fn addr(name: &str) -> Addr {
    MockApi::default().addr_make(name)
}
fn addr_str(name: &str) -> String {
    addr(name).to_string()
}
use cw_hooks::HooksResponse;
use dao_voting::{pre_propose::PreProposeSubmissionPolicy, status::Status};

use crate::{
    error::PreProposeError,
    msg::{ExecuteMsg, QueryMsg},
    state::{Config, PreProposeContract},
};

type Contract = PreProposeContract<Empty, Empty, Empty, Empty, Empty>;

#[test]
fn test_completed_hook_status_invariant() {
    let mut deps = mock_dependencies();
    let info = message_info(&addr("pm"), &[]);

    let module = Contract::default();

    module
        .proposal_module
        .save(&mut deps.storage, &addr("pm"))
        .unwrap();

    let res = module.execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::ProposalCompletedHook {
            proposal_id: 1,
            new_status: Status::Passed,
        },
    );

    assert_eq!(
        res.unwrap_err(),
        PreProposeError::NotCompleted {
            status: Status::Passed
        }
    );
}

#[test]
fn test_completed_hook_auth() {
    let mut deps = mock_dependencies();
    let info = message_info(&addr("evil"), &[]);
    let module = Contract::default();

    module
        .proposal_module
        .save(&mut deps.storage, &addr("pm"))
        .unwrap();

    let res = module.execute(
        deps.as_mut(),
        mock_env(),
        info,
        ExecuteMsg::ProposalCompletedHook {
            proposal_id: 1,
            new_status: Status::Passed,
        },
    );

    assert_eq!(res.unwrap_err(), PreProposeError::NotModule {});
}

#[test]
fn test_proposal_submitted_hooks() {
    let mut deps = mock_dependencies();
    let module = Contract::default();

    module.dao.save(&mut deps.storage, &addr("d")).unwrap();
    module
        .proposal_module
        .save(&mut deps.storage, &addr("pm"))
        .unwrap();
    module
        .config
        .save(
            &mut deps.storage,
            &Config {
                deposit_info: None,
                submission_policy: PreProposeSubmissionPolicy::Anyone { denylist: vec![] },
            },
        )
        .unwrap();

    // The DAO can add a hook.
    let info = message_info(&addr("d"), &[]);
    module
        .execute_add_proposal_submitted_hook(deps.as_mut(), info, addr_str("one"))
        .unwrap();
    let hooks: HooksResponse = from_json(
        module
            .query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ProposalSubmittedHooks {},
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(hooks.hooks, vec![addr_str("one")]);

    // Non-DAO addresses can not add hooks.
    let info = message_info(&addr("n"), &[]);
    let err = module
        .execute_add_proposal_submitted_hook(deps.as_mut(), info, addr_str("two"))
        .unwrap_err();
    assert_eq!(err, PreProposeError::NotDao {});

    deps.querier.update_wasm(|_| {
        // for responding to the next proposal ID query that gets fired by propose.
        cosmwasm_std::SystemResult::Ok(ContractResult::Ok(to_json_binary(&1u64).unwrap()))
    });

    // The hooks fire when a proposal is created.
    let res = module
        .execute(
            deps.as_mut(),
            mock_env(),
            message_info(&addr("a"), &[]),
            ExecuteMsg::Propose {
                msg: Empty::default(),
            },
        )
        .unwrap();
    assert_eq!(
        res.messages[1],
        SubMsg::new(WasmMsg::Execute {
            contract_addr: addr_str("one"),
            msg: to_json_binary(&Empty::default()).unwrap(),
            funds: vec![],
        })
    );

    // Non-DAO addresses can not remove hooks.
    let info = message_info(&addr("n"), &[]);
    let err = module
        .execute_remove_proposal_submitted_hook(deps.as_mut(), info, addr_str("one"))
        .unwrap_err();
    assert_eq!(err, PreProposeError::NotDao {});

    // The DAO can remove a hook.
    let info = message_info(&addr("d"), &[]);
    module
        .execute_remove_proposal_submitted_hook(deps.as_mut(), info, addr_str("one"))
        .unwrap();
    let hooks: HooksResponse = from_json(
        module
            .query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ProposalSubmittedHooks {},
            )
            .unwrap(),
    )
    .unwrap();
    assert!(hooks.hooks.is_empty());
}

#[test]
fn test_query_ext_does_nothing() {
    let deps = mock_dependencies();
    let module = Contract::default();

    let res = module
        .query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::QueryExtension {
                msg: Empty::default(),
            },
        )
        .unwrap();
    assert_eq!(res, Binary::default())
}

#[test]
fn test_execute_ext_does_nothing() {
    let mut deps = mock_dependencies();
    let module = Contract::default();

    let res = module
        .execute(
            deps.as_mut(),
            mock_env(),
            message_info(&addr("addr"), &[]),
            ExecuteMsg::Extension {
                msg: Empty::default(),
            },
        )
        .unwrap();
    assert_eq!(res, Response::default())
}
