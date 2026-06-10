use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{Addr, Binary, StdResult};
use cw_multi_test::{App, AppResponse, Executor};

use cw_utils::Duration;

use crate::msg::{ClaimType, ExecuteMsg};

// Shorthand for a properly made address.
macro_rules! addr {
    ($x:expr ) => {
        MockApi::default().addr_make($x)
    };
}

pub fn send_nft(
    app: &mut App,
    cw721: &Addr,
    sender: &str,
    receiver: &Addr,
    token_id: &str,
    msg: Binary,
) -> StdResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        cw721.clone(),
        &cw721_base::msg::ExecuteMsg::SendNft {
            contract: receiver.to_string(),
            token_id: token_id.to_string(),
            msg,
        },
        &[],
    )
}

pub fn mint_nft(
    app: &mut App,
    cw721: &Addr,
    sender: &str,
    receiver: &str,
    token_id: &str,
) -> StdResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        cw721.clone(),
        &cw721_base::msg::ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: MockApi::default().addr_make(receiver).to_string(),
            token_uri: None,
            extension: None,
        },
        &[],
    )
}

pub fn stake_nft(
    app: &mut App,
    cw721: &Addr,
    module: &Addr,
    sender: &str,
    token_id: &str,
) -> StdResult<AppResponse> {
    send_nft(app, cw721, sender, module, token_id, Binary::default())
}

pub fn mint_and_stake_nft(
    app: &mut App,
    cw721: &Addr,
    module: &Addr,
    sender: &str,
    token_id: &str,
) -> StdResult<()> {
    mint_nft(app, cw721, sender, sender, token_id)?;
    stake_nft(app, cw721, module, sender, token_id)?;
    Ok(())
}

pub fn unstake_nfts(
    app: &mut App,
    module: &Addr,
    sender: &str,
    token_ids: &[&str],
) -> StdResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        module.clone(),
        &ExecuteMsg::Unstake {
            token_ids: token_ids.iter().map(|s| s.to_string()).collect(),
        },
        &[],
    )
}

pub fn update_config(
    app: &mut App,
    module: &Addr,
    sender: &str,
    duration: Option<Duration>,
) -> StdResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        module.clone(),
        &ExecuteMsg::UpdateConfig { duration },
        &[],
    )
}

pub fn claim_nfts(app: &mut App, module: &Addr, sender: &str) -> StdResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        module.clone(),
        &ExecuteMsg::ClaimNfts {
            r#type: ClaimType::All,
        },
        &[],
    )
}

pub fn claim_specific_nfts(
    app: &mut App,
    module: &Addr,
    sender: &str,
    token_ids: &[String],
) -> StdResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        module.clone(),
        &ExecuteMsg::ClaimNfts {
            r#type: ClaimType::Specific(token_ids.to_vec()),
        },
        &[],
    )
}

// pub fn claim_legacy_nfts(app: &mut App, module: &Addr, sender: &str) -> StdResult<AppResponse> {
//     app.execute_contract(
//         addr!(sender),
//         module.clone(),
//         &ExecuteMsg::ClaimNfts {
//             r#type: ClaimType::Legacy,
//         },
//         &[],
//     )
// }

pub fn add_hook(app: &mut App, module: &Addr, sender: &str, hook: &str) -> StdResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        module.clone(),
        &ExecuteMsg::AddHook {
            addr: MockApi::default().addr_make(hook).to_string(),
        },
        &[],
    )
}

pub fn remove_hook(
    app: &mut App,
    module: &Addr,
    sender: &str,
    hook: &str,
) -> StdResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        module.clone(),
        &ExecuteMsg::RemoveHook {
            addr: MockApi::default().addr_make(hook).to_string(),
        },
        &[],
    )
}
