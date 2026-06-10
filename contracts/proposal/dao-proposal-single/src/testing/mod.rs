mod adversarial_tests;
mod do_votes;
mod execute;
mod instantiate;
#[cfg(feature = "v1")]
mod migration_tests;
mod queries;
mod tests;

pub(crate) const CREATOR_ADDR: &str = "creator";

use cosmwasm_std::{testing::MockApi, Addr};

/// Convert a human-readable name to a bech32 address.
pub(crate) fn addr(name: &str) -> Addr {
    MockApi::default().addr_make(name)
}

/// Convert a human-readable name to a bech32 address string.
pub(crate) fn addr_str(name: &str) -> String {
    addr(name).to_string()
}
