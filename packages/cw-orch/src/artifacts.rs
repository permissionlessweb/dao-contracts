//! Which wasm file a chain should store.
//!
//! Terp's tokenfactory is the Osmosis module. The Thorchain builds of the
//! issuer and of staked-token voting are a different chain interface.

use cw_orch::prelude::ChainInfoOwned;

pub fn is_terp_chain(chain: &ChainInfoOwned) -> bool {
    chain
        .network_info
        .chain_name
        .eq_ignore_ascii_case("terp")
        || chain
            .network_info
            .pub_address_prefix
            .eq_ignore_ascii_case("terp")
}

/// File name inside the dao-contracts artifacts directory.
///
/// On Terp, tokenfactory uses the Osmosis wasm, staked-token voting uses the
/// default (non-Thorchain) wasm, and vesting uses the staking build. Every
/// other contract keeps `{logical}.wasm`.
pub fn terp_wasm_file(logical_name: &str) -> String {
    match logical_name {
        "cw_tokenfactory_issuer" => "cw_tokenfactory_issuer-osmosis.wasm".to_string(),
        "dao_voting_token_staked" => "dao_voting_token_staked-default.wasm".to_string(),
        "cw_vesting" => "cw_vesting-staking.wasm".to_string(),
        other => format!("{other}.wasm"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_orch::prelude::*;

    fn terp() -> ChainInfoOwned {
        networks::TERP_MAINNET.into()
    }

    #[test]
    fn terp_skips_thorchain_builds() {
        assert!(is_terp_chain(&terp()));
        assert_eq!(
            terp_wasm_file("cw_tokenfactory_issuer"),
            "cw_tokenfactory_issuer-osmosis.wasm"
        );
        assert_eq!(
            terp_wasm_file("dao_voting_token_staked"),
            "dao_voting_token_staked-default.wasm"
        );
        assert_eq!(terp_wasm_file("cw_vesting"), "cw_vesting-staking.wasm");
        assert!(!terp_wasm_file("cw_tokenfactory_issuer").contains("thorchain"));
        assert!(!terp_wasm_file("dao_voting_token_staked").contains("thorchain"));
    }
}
