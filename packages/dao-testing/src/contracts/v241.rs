// V2.4.1 contract wrappers are incompatible with cw-multi-test v2
// because the v241 crates use cosmwasm-std v1 entry point signatures.
// These stubs exist to keep the public API intact; actual v241 migration
// testing requires a different approach (e.g. test-tube).

use cosmwasm_std::Empty;
use cw_multi_test::Contract;

pub fn dao_dao_core_v241_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v241 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn dao_voting_cw4_v241_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v241 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn dao_proposal_single_v241_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v241 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn dao_proposal_multiple_v241_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v241 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn dao_pre_propose_single_v241_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v241 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn dao_pre_propose_approval_single_v241_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v241 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn dao_pre_propose_multiple_v241_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v241 contract wrappers are incompatible with cw-multi-test v2")
}
