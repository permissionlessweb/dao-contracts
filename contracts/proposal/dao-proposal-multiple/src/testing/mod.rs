pub mod adversarial_tests;
pub mod do_votes;
pub mod execute;
pub mod instantiate;
pub mod queries;
pub mod tests;

use cosmwasm_std::{testing::MockApi, Addr};

/// Convert a human-readable name to a bech32 address.
pub(crate) fn addr(name: &str) -> Addr {
    MockApi::default().addr_make(name)
}

/// Convert a human-readable name to a bech32 address string.
pub(crate) fn addr_str(name: &str) -> String {
    addr(name).to_string()
}
