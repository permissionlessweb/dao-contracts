use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, message_info, MockApi},
    Addr,
};
use dao_hooks::nft_stake::{stake_nft_hook_msgs, unstake_nft_hook_msgs};

use crate::{
    contract::execute,
    state::{Config, CONFIG, DAO, HOOKS},
};

#[test]
fn test_hooks() {
    let mut deps = mock_dependencies();

    let messages = stake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        MockApi::default().addr_make("ekez"),
        "ekez-token".to_string(),
    )
    .unwrap();
    assert_eq!(messages.len(), 0);

    let messages = unstake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        MockApi::default().addr_make("ekez"),
        vec!["ekez-token".to_string()],
    )
    .unwrap();
    assert_eq!(messages.len(), 0);

    // Save a DAO address for the execute messages we're testing.
    DAO.save(deps.as_mut().storage, &MockApi::default().addr_make("ekez"))
        .unwrap();

    // Save a config for the execute messages we're testing.
    CONFIG
        .save(
            deps.as_mut().storage,
            &Config {
                nft_address: MockApi::default().addr_make("ekez-token"),
                unstaking_duration: None,
            },
        )
        .unwrap();

    let env = mock_env();
    let info = message_info(&MockApi::default().addr_make("ekez"), &[]);

    execute(
        deps.as_mut(),
        env,
        info,
        crate::msg::ExecuteMsg::AddHook {
            addr: MockApi::default().addr_make("ekez").to_string(),
        },
    )
    .unwrap();

    let messages = stake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        MockApi::default().addr_make("ekez"),
        "ekez-token".to_string(),
    )
    .unwrap();
    assert_eq!(messages.len(), 1);

    let messages = unstake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        MockApi::default().addr_make("ekez"),
        vec!["ekez-token".to_string()],
    )
    .unwrap();
    assert_eq!(messages.len(), 1);

    let env = mock_env();
    let info = message_info(&MockApi::default().addr_make("ekez"), &[]);

    execute(
        deps.as_mut(),
        env,
        info,
        crate::msg::ExecuteMsg::RemoveHook {
            addr: MockApi::default().addr_make("ekez").to_string(),
        },
    )
    .unwrap();

    let messages = stake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        MockApi::default().addr_make("ekez"),
        "ekez-token".to_string(),
    )
    .unwrap();
    assert_eq!(messages.len(), 0);

    let messages = unstake_nft_hook_msgs(
        HOOKS,
        &deps.storage,
        MockApi::default().addr_make("ekez"),
        vec!["ekez-token".to_string()],
    )
    .unwrap();
    assert_eq!(messages.len(), 0);
}
