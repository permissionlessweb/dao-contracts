use cosmwasm_std::Addr;
use cw_multi_test::{App, Executor};
use dao_testing::contracts::cw721_roles_contract;

/// Note: `sender` and `minter` must already be valid bech32 addresses.
pub fn instantiate_cw721_roles(app: &mut App, sender: &str, minter: &str) -> (Addr, u64) {
    let cw721_id = app.store_code(cw721_roles_contract());

    let cw721_addr = app
        .instantiate_contract(
            cw721_id,
            Addr::unchecked(sender), // already bech32 from caller
            &cw721_base::msg::InstantiateMsg {
                name: "bad kids".to_string(),
                symbol: "bad kids".to_string(),
                minter: Some(minter.to_string()),
                collection_info_extension: None,
                creator: None,
                withdraw_address: None,
            },
            &[],
            "cw721_roles".to_string(),
            None,
        )
        .unwrap();

    (cw721_addr, cw721_id)
}
