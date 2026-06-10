mod base;
mod cw20_suite;
mod cw4_suite;
mod cw721_suite;
mod dao_suite;
pub mod deploy_data;
mod modules;
mod token_suite;

#[cfg(test)]
mod tests;

pub const OWNER: &str = "cosmwasm1fsgzj6t7udv8zhf6zj32mkqhcjcpv52yph5qsdcl0qt94jgdckqs2g053y";

pub const ADDR0: &str = "cosmwasm1phjtlrk4fw73vay42g4hrdy20cmmpkfn80msl7jjmta9k800n32s3mfntm";
pub const ADDR1: &str = "cosmwasm14ch5q26mhx3jk5cxl88t278nper264ce5fa7agjr4cw0yfjj7c6q56drym";
pub const ADDR2: &str = "cosmwasm1cq2j7y4utseeatek2alfy5ttaphjrtdxqqz0sn820v9jupy0seuqmh8c9s";
pub const ADDR3: &str = "cosmwasm1384tqgn4nknw9dk7rt5u5axd5g6zwrsc4p8qed22t329h803205qhm564r";
pub const ADDR4: &str = "cosmwasm1q5nfz2u8guyfkjnyy2qw8kgdxeryae0jxuyaumze8ygqqxymrres6seka8";

pub const DENOM: &str = "udenom";
pub const GOV_DENOM: &str = "ugovtoken";

pub use cw_multi_test::Executor;

// Old cw-multi-test suite (preserved)
pub use crate::deploy_data::DaoDeployData;
pub use base::*;
pub use cw20_suite::*;
pub use cw4_suite::*;
pub use cw721_suite::*;
pub use dao_suite::*;
pub use modules::*;
pub use token_suite::*;
