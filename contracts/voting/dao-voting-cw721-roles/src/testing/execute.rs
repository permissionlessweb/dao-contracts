use cosmwasm_std::Addr;
use cw_multi_test::{App, AppResponse, Executor};
use dao_cw721_extensions::roles::{ExecuteExt, MetadataExt};

use anyhow::Result as AnyResult;

/// Note: `sender` and `receiver` must already be valid bech32 addresses.
pub fn mint_nft(
    app: &mut App,
    cw721: &Addr,
    sender: &str,
    receiver: &str,
    token_id: &str,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        Addr::unchecked(sender), // already bech32 from caller
        cw721.clone(),
        &cw721_roles::msg::ExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: receiver.to_string(),
            token_uri: None,
            extension: MetadataExt {
                role: Some("admin".to_string()),
                weight: 1,
            },
        },
        &[],
    )
}
