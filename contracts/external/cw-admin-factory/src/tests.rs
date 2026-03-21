use std::vec;

use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, message_info, MockApi},
    to_json_binary, Addr, Binary, Reply, SubMsg, SubMsgResponse, SubMsgResult, WasmMsg,
};
use cw_multi_test::{App, AppResponse, Executor};
use dao_interface::state::{Admin, ModuleInstantiateInfo};
use dao_testing::contracts::{
    cw20_base_contract, cw_admin_factory_contract, dao_dao_core_contract,
};

use crate::{
    contract::{
        instantiate, migrate, reply, CONTRACT_NAME, CONTRACT_VERSION, INSTANTIATE_CONTRACT_REPLY_ID,
    },
    msg::{AdminResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
};
use cw_admin_factory::ContractError;

const ADMIN_ADDR: &str = "admin";

#[test]
pub fn test_set_self_admin() {
    let mut app = App::default();
    let code_id = app.store_code(cw_admin_factory_contract());
    let cw20_code_id = app.store_code(cw20_base_contract());
    let cw20_instantiate = cw20_base::msg::InstantiateMsg {
        name: "DAO".to_string(),
        symbol: "DAO".to_string(),
        decimals: 6,
        initial_balances: vec![],
        mint: None,
        marketing: None,
    };

    let instantiate = InstantiateMsg { admin: None };
    let factory_addr = app
        .instantiate_contract(
            code_id,
            MockApi::default().addr_make("CREATOR"),
            &instantiate,
            &[],
            "cw-admin-factory",
            None,
        )
        .unwrap();

    // Instantiate core contract using factory.
    let cw_core_code_id = app.store_code(dao_dao_core_contract());
    let instantiate_core = dao_interface::msg::InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: cw20_code_id,
            msg: to_json_binary(&cw20_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: None,
            label: "voting module".to_string(),
            salt: None,
        },
        proposal_modules_instantiate_info: vec![
            ModuleInstantiateInfo {
                code_id: cw20_code_id,
                msg: to_json_binary(&cw20_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: None,
                label: "prop module".to_string(),
                salt: None,
            },
            ModuleInstantiateInfo {
                code_id: cw20_code_id,
                msg: to_json_binary(&cw20_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: None,
                label: "prop module 2".to_string(),
                salt: None,
            },
        ],
        initial_items: None,
        initial_actions: None,
    };

    let res: AppResponse = app
        .execute_contract(
            MockApi::default().addr_make("CREATOR"),
            factory_addr,
            &ExecuteMsg::InstantiateContractWithSelfAdmin {
                instantiate_msg: to_json_binary(&instantiate_core).unwrap(),
                code_id: cw_core_code_id,
                label: "my contract".to_string(),
            },
            &[],
        )
        .unwrap();

    // Get the core address from the instantiate event
    let instantiate_event = &res.events[2];
    assert_eq!(instantiate_event.ty, "instantiate");
    let core_addr = instantiate_event.attributes[0].value.clone();

    // Check that admin of core address is itself
    let contract_info = app.wrap().query_wasm_contract_info(&core_addr).unwrap();
    assert_eq!(contract_info.admin, Some(Addr::unchecked(core_addr)))
}

#[test]
pub fn test_authorized_set_self_admin() {
    let mut app = App::default();
    let code_id = app.store_code(cw_admin_factory_contract());
    let cw20_code_id = app.store_code(cw20_base_contract());
    let cw20_instantiate = cw20_base::msg::InstantiateMsg {
        name: "DAO".to_string(),
        symbol: "DAO".to_string(),
        decimals: 6,
        initial_balances: vec![],
        mint: None,
        marketing: None,
    };

    let instantiate = InstantiateMsg {
        admin: Some(MockApi::default().addr_make(ADMIN_ADDR).to_string()),
    };
    let factory_addr = app
        .instantiate_contract(
            code_id,
            MockApi::default().addr_make(ADMIN_ADDR),
            &instantiate,
            &[],
            "cw-admin-factory",
            None,
        )
        .unwrap();

    // Query admin.
    let current_admin: AdminResponse = app
        .wrap()
        .query_wasm_smart(factory_addr.clone(), &QueryMsg::Admin {})
        .unwrap();
    assert_eq!(
        current_admin.admin,
        Some(MockApi::default().addr_make(ADMIN_ADDR))
    );

    // Instantiate core contract using factory.
    let cw_core_code_id = app.store_code(dao_dao_core_contract());
    let instantiate_core = dao_interface::msg::InstantiateMsg {
        dao_uri: None,
        admin: None,
        name: "DAO DAO".to_string(),
        description: "A DAO that builds DAOs.".to_string(),
        image_url: None,
        automatically_add_cw20s: true,
        automatically_add_cw721s: true,
        voting_module_instantiate_info: ModuleInstantiateInfo {
            code_id: cw20_code_id,
            msg: to_json_binary(&cw20_instantiate).unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: None,
            label: "voting module".to_string(),
            salt: None,
        },
        proposal_modules_instantiate_info: vec![
            ModuleInstantiateInfo {
                code_id: cw20_code_id,
                msg: to_json_binary(&cw20_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: None,
                label: "prop module".to_string(),
                salt: None,
            },
            ModuleInstantiateInfo {
                code_id: cw20_code_id,
                msg: to_json_binary(&cw20_instantiate).unwrap(),
                admin: Some(Admin::CoreModule {}),
                funds: None,
                label: "prop module 2".to_string(),
                salt: None,
            },
        ],
        initial_items: None,
        initial_actions: None,
    };

    // Fails when not the admin.
    let err: ContractError = app
        .execute_contract(
            MockApi::default().addr_make("not_admin"),
            factory_addr.clone(),
            &ExecuteMsg::InstantiateContractWithSelfAdmin {
                instantiate_msg: to_json_binary(&instantiate_core).unwrap(),
                code_id: cw_core_code_id,
                label: "my contract".to_string(),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Succeeds as the admin.
    let res: AppResponse = app
        .execute_contract(
            MockApi::default().addr_make(ADMIN_ADDR),
            factory_addr,
            &ExecuteMsg::InstantiateContractWithSelfAdmin {
                instantiate_msg: to_json_binary(&instantiate_core).unwrap(),
                code_id: cw_core_code_id,
                label: "my contract".to_string(),
            },
            &[],
        )
        .unwrap();

    // Get the core address from the instantiate event
    let instantiate_event = &res.events[2];
    assert_eq!(instantiate_event.ty, "instantiate");
    let core_addr = instantiate_event.attributes[0].value.clone();

    // Check that admin of core address is itself
    let contract_info = app.wrap().query_wasm_contract_info(&core_addr).unwrap();
    assert_eq!(contract_info.admin, Some(Addr::unchecked(core_addr)))
}

#[test]
pub fn test_set_self_admin_mock() {
    let mut deps = mock_dependencies();
    // Instantiate factory contract
    let instantiate_msg = InstantiateMsg { admin: None };
    let info = message_info(&MockApi::default().addr_make("creator"), &[]);
    let env = mock_env();
    instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();
    let mock_api = MockApi::default();
    let contract_addr_str = mock_api.addr_make("contract2").to_string();
    let reply_msg: Reply = Reply {
        id: INSTANTIATE_CONTRACT_REPLY_ID,
        payload: Default::default(),
        gas_used: 0,
        result: SubMsgResult::Ok(SubMsgResponse {
            events: vec![cosmwasm_std::Event::new("instantiate")
                .add_attribute("_contract_address", contract_addr_str.clone())],
            data: None,
            msg_responses: vec![],
        }),
    };

    let res = reply(deps.as_mut(), env, reply_msg).unwrap();
    assert_eq!(res.attributes.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(WasmMsg::UpdateAdmin {
            contract_addr: contract_addr_str.clone(),
            admin: contract_addr_str
        })
    )
}

#[test]
pub fn test_migrate_update_version() {
    let mut deps = mock_dependencies();
    cw2::set_contract_version(&mut deps.storage, "my-contract", "old-version").unwrap();
    migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();
    let version = cw2::get_contract_version(&deps.storage).unwrap();
    assert_eq!(version.version, CONTRACT_VERSION);
    assert_eq!(version.contract, CONTRACT_NAME);
}
