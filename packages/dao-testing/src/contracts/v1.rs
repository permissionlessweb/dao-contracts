// V1 contract wrappers are incompatible with cw-multi-test v2
// because the v1 crates use cosmwasm-std v1 entry point signatures.
// These stubs exist to keep the public API intact; actual v1 migration
// testing requires a different approach (e.g. test-tube).

use cosmwasm_std::Empty;
use cw_multi_test::Contract;

pub fn cw_proposal_single_v1_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v1 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn cw_core_v1_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v1 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn cw4_voting_v1_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v1 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn cw20_stake_v1_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v1 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn stake_cw20_v03_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v1 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn cw20_stake_external_rewards_v1_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v1 contract wrappers are incompatible with cw-multi-test v2")
}

pub fn cw20_stake_reward_distributor_v1_contract() -> Box<dyn Contract<Empty>> {
    unimplemented!("v1 contract wrappers are incompatible with cw-multi-test v2")
}
