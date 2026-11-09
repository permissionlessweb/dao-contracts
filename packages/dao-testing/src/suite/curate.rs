//! Find one DAO's single-choice proposal entry and build its propose message.
//!
//! Two shapes are supported. A `dao-pre-propose-single` module gets the nested
//! propose message. A module that allows anyone to propose gets the proposal
//! module's own propose message. Other pre-propose contracts are refused.

use cosmwasm_std::{from_json, Addr, CosmosMsg, Empty};
use cw_orch::prelude::{QueryHandler, WasmQuerier};
use dao_interface::state::{ProposalModule, ProposalModuleStatus};
use dao_voting::pre_propose::ProposalCreationPolicy;
use dao_voting::proposal::SingleChoiceProposeMsg;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

pub const PROPOSAL_SINGLE: &str = "crates.io:dao-proposal-single";
pub const PRE_PROPOSE_SINGLE: &str = "crates.io:dao-pre-propose-single";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProposalEntry {
    /// Propose on this pre-propose contract.
    PreProposeSingle { pre_propose: String },
    /// Propose on the proposal module itself.
    Direct,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SingleChoiceRoute {
    pub prefix: String,
    pub proposal: String,
    pub entry: ProposalEntry,
}

impl SingleChoiceRoute {
    /// Contract that receives the propose message.
    pub fn target(&self) -> &str {
        match &self.entry {
            ProposalEntry::PreProposeSingle { pre_propose } => pre_propose,
            ProposalEntry::Direct => &self.proposal,
        }
    }

    pub fn propose_msg(
        &self,
        title: impl Into<String>,
        description: impl Into<String>,
        msgs: Vec<CosmosMsg<Empty>>,
    ) -> serde_json::Value {
        match &self.entry {
            ProposalEntry::PreProposeSingle { .. } => {
                pre_propose_single_body(title, description, msgs)
            }
            ProposalEntry::Direct => direct_propose_body(title, description, msgs),
        }
    }
}

/// Nested propose body accepted by `dao-pre-propose-single`.
pub fn pre_propose_single_body(
    title: impl Into<String>,
    description: impl Into<String>,
    msgs: Vec<CosmosMsg<Empty>>,
) -> serde_json::Value {
    let msg = dao_pre_propose_single::ExecuteMsg::Propose {
        msg: dao_pre_propose_single::ProposeMessage::Propose {
            title: title.into(),
            description: description.into(),
            msgs,
            vote: None,
        },
    };
    let mut body = serde_json::to_value(&msg).expect("pre-propose message");
    drop_absent_options(&mut body);
    body
}

/// Propose body accepted by `dao-proposal-single` when anyone may propose.
pub fn direct_propose_body(
    title: impl Into<String>,
    description: impl Into<String>,
    msgs: Vec<CosmosMsg<Empty>>,
) -> serde_json::Value {
    let msg = dao_proposal_single::msg::ExecuteMsg::Propose(SingleChoiceProposeMsg {
        title: title.into(),
        description: description.into(),
        msgs,
        proposer: None,
        vote: None,
    });
    let mut body = serde_json::to_value(&msg).expect("direct propose message");
    drop_absent_options(&mut body);
    body
}

/// `vote` and `proposer` are filled by the contract when absent. serde writes
/// `None` as null; the dry-run JSON leaves those keys out.
fn drop_absent_options(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.retain(|key, child| {
                if (key == "vote" || key == "proposer") && child.is_null() {
                    return false;
                }
                drop_absent_options(child);
                true
            });
        }
        serde_json::Value::Array(items) => {
            for item in items {
                drop_absent_options(item);
            }
        }
        _ => {}
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CurateError {
    Query(String),
    NoSingleChoice,
    PrefixMissing(String),
    Several(Vec<String>),
    UnknownPrePropose { prefix: String, contract: String },
}

impl std::fmt::Display for CurateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query(err) => write!(f, "{err}"),
            Self::NoSingleChoice => write!(f, "no single-choice proposal module"),
            Self::PrefixMissing(prefix) => {
                write!(f, "no single-choice proposal module with prefix {prefix}")
            }
            Self::Several(prefixes) => write!(
                f,
                "several single-choice modules ({}); pass a prefix",
                prefixes.join(", ")
            ),
            Self::UnknownPrePropose { prefix, contract } => write!(
                f,
                "proposal module {prefix} uses pre-propose {contract}, not {PRE_PROPOSE_SINGLE}"
            ),
        }
    }
}

impl std::error::Error for CurateError {}

/// Smart queries plus the cw2 contract name. `QueryHandler` implements this.
pub trait SmartQuery {
    fn smart<T: DeserializeOwned>(
        &self,
        contract: &str,
        msg: &(impl Serialize + Debug),
    ) -> Result<T, String>;

    fn contract_name(&self, contract: &str) -> Result<String, String>;
}

impl<C> SmartQuery for C
where
    C: QueryHandler,
{
    fn smart<T: DeserializeOwned>(
        &self,
        contract: &str,
        msg: &(impl Serialize + Debug),
    ) -> Result<T, String> {
        self.query(msg, &Addr::unchecked(contract))
            .map_err(|err| err.to_string())
    }

    fn contract_name(&self, contract: &str) -> Result<String, String> {
        let raw = self
            .wasm_querier()
            .raw_query(&Addr::unchecked(contract), b"contract_info".to_vec())
            .map_err(|err| err.to_string())?;
        let version: cw2::ContractVersion = from_json(raw).map_err(|err| err.to_string())?;
        Ok(version.contract)
    }
}

/// The single-choice route for `core`.
///
/// `prefix` selects one module when the DAO has several single-choice modules.
/// With one such module, `prefix` may be `None`.
pub fn single_choice_route(
    query: &impl SmartQuery,
    core: &str,
    prefix: Option<&str>,
) -> Result<SingleChoiceRoute, CurateError> {
    let modules: Vec<ProposalModule> = query
        .smart(
            core,
            &dao_interface::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            },
        )
        .map_err(CurateError::Query)?;
    let mut found = Vec::new();
    for module in modules {
        if module.status != ProposalModuleStatus::Enabled {
            continue;
        }
        if let Some(want) = prefix {
            if module.prefix != want {
                continue;
            }
        }
        let name = query
            .contract_name(module.address.as_str())
            .map_err(CurateError::Query)?;
        if name != PROPOSAL_SINGLE {
            continue;
        }
        let policy: ProposalCreationPolicy = query
            .smart(
                module.address.as_str(),
                &dao_proposal_single::msg::QueryMsg::ProposalCreationPolicy {},
            )
            .map_err(CurateError::Query)?;
        let entry = match policy {
            ProposalCreationPolicy::Anyone {} => ProposalEntry::Direct,
            ProposalCreationPolicy::Module { addr } => {
                let pre_name = query
                    .contract_name(addr.as_str())
                    .map_err(CurateError::Query)?;
                if pre_name != PRE_PROPOSE_SINGLE {
                    return Err(CurateError::UnknownPrePropose {
                        prefix: module.prefix,
                        contract: pre_name,
                    });
                }
                ProposalEntry::PreProposeSingle {
                    pre_propose: addr.into_string(),
                }
            }
        };
        found.push(SingleChoiceRoute {
            prefix: module.prefix,
            proposal: module.address.into_string(),
            entry,
        });
    }
    found.sort_by(|a, b| a.prefix.cmp(&b.prefix));
    match (prefix, found.len()) {
        (_, 1) => Ok(found.remove(0)),
        (Some(prefix), 0) => Err(CurateError::PrefixMissing(prefix.to_string())),
        (None, 0) => Err(CurateError::NoSingleChoice),
        (None, _) => Err(CurateError::Several(
            found.into_iter().map(|route| route.prefix).collect(),
        )),
        (Some(prefix), _) => Err(CurateError::Several(vec![prefix.to_string()])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::*;
    use crate::suite::base::DaoTestingSuiteBase;
    use crate::suite::cw4_suite::DaoTestingSuiteCw4;
    use crate::suite::{DaoTestingSuite, ADDR0};
    use cosmwasm_std::{to_json_binary, Addr, Coin};
    use cw_multi_test::{App, Executor};
    use cw_utils::Duration;
    use dao_interface::state::{Admin, ModuleInstantiateInfo};
    use dao_voting::threshold::{PercentageThreshold, Threshold};

    struct AppQuery<'a> {
        app: &'a App,
    }

    impl SmartQuery for AppQuery<'_> {
        fn smart<T: DeserializeOwned>(
            &self,
            contract: &str,
            msg: &(impl Serialize + Debug),
        ) -> Result<T, String> {
            self.app
                .wrap()
                .query_wasm_smart(contract, msg)
                .map_err(|err| err.to_string())
        }

        fn contract_name(&self, contract: &str) -> Result<String, String> {
            let raw = self
                .app
                .wrap()
                .query_wasm_raw(contract, b"contract_info".as_slice())
                .map_err(|err| err.to_string())?
                .ok_or_else(|| format!("no cw2 info at {contract}"))?;
            let version: cw2::ContractVersion = from_json(raw).map_err(|err| err.to_string())?;
            Ok(version.contract)
        }
    }

    #[test]
    fn pre_propose_single_omits_null_vote() {
        let body = pre_propose_single_body("title", "desc", vec![]);
        assert_eq!(body["propose"]["msg"]["propose"]["title"], "title");
        assert_eq!(body["propose"]["msg"]["propose"]["description"], "desc");
        assert!(body["propose"]["msg"]["propose"]["msgs"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(body["propose"]["msg"]["propose"].get("vote").is_none());
        let direct = direct_propose_body("title", "desc", vec![]);
        assert_eq!(direct["propose"]["title"], "title");
        assert!(direct["propose"].get("vote").is_none());
        assert!(direct["propose"].get("proposer").is_none());
        assert!(direct["propose"].get("msg").is_none());
    }

    #[test]
    fn cw4_dao_uses_pre_propose_single_and_skips_multiple() {
        let mut base = DaoTestingSuiteBase::base();
        let mut suite = DaoTestingSuiteCw4::new(&mut base);
        let dao = DaoTestingSuite::dao(&mut suite);
        let query = AppQuery { app: &suite.app };
        let route = single_choice_route(&query, dao.core_addr.as_str(), None).unwrap();
        let pre = dao.proposal_modules[0].0.clone().unwrap();
        assert_eq!(route.prefix, "A");
        assert_eq!(
            route.entry,
            ProposalEntry::PreProposeSingle {
                pre_propose: pre.to_string()
            }
        );
        assert_eq!(route.target(), pre.as_str());
        let missing = single_choice_route(&query, dao.core_addr.as_str(), Some("B")).unwrap_err();
        assert_eq!(missing, CurateError::PrefixMissing("B".into()));
    }

    #[test]
    fn anyone_may_propose_targets_the_proposal_module() {
        let mut app = App::default();
        let core_id = app.store_code(dao_dao_core_contract());
        let proposal_id = app.store_code(dao_proposal_single_contract());
        let voting_id = app.store_code(dao_voting_cw4_contract());
        let group_id = app.store_code(cw4_group_contract());
        let voting = ModuleInstantiateInfo {
            code_id: voting_id,
            msg: to_json_binary(&dao_voting_cw4::msg::InstantiateMsg {
                group_contract: dao_voting_cw4::msg::GroupContract::New {
                    cw4_group_code_id: group_id,
                    cw4_group_salt: None,
                    initial_members: vec![cw4::Member {
                        addr: ADDR0.into(),
                        weight: 1,
                    }],
                },
            })
            .unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: None,
            label: "voting".into(),
            salt: None,
        };
        let proposal = ModuleInstantiateInfo {
            code_id: proposal_id,
            msg: to_json_binary(&dao_proposal_single::msg::InstantiateMsg {
                threshold: Threshold::AbsolutePercentage {
                    percentage: PercentageThreshold::Majority {},
                },
                max_voting_period: Duration::Time(60),
                min_voting_period: None,
                only_members_execute: true,
                allow_revoting: false,
                pre_propose_info: dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
                close_proposal_on_execution_failure: true,
                veto: None,
                delegation_module: None,
            })
            .unwrap(),
            admin: Some(Admin::CoreModule {}),
            funds: None,
            label: "proposal".into(),
            salt: None,
        };
        let core = app
            .instantiate_contract(
                core_id,
                Addr::unchecked(ADDR0),
                &dao_interface::msg::InstantiateMsg {
                    admin: None,
                    name: "anyone".into(),
                    description: "anyone".into(),
                    image_url: None,
                    automatically_add_cw20s: false,
                    automatically_add_cw721s: false,
                    voting_module_instantiate_info: voting,
                    proposal_modules_instantiate_info: vec![proposal],
                    initial_items: None,
                    initial_actions: None,
                    dao_uri: None,
                },
                &[] as &[Coin],
                "anyone",
                None,
            )
            .unwrap();
        let route = single_choice_route(&AppQuery { app: &app }, core.as_str(), None).unwrap();
        assert_eq!(route.entry, ProposalEntry::Direct);
        assert_eq!(route.target(), route.proposal.as_str());
        let body = route.propose_msg("title", "desc", vec![]);
        assert_eq!(body["propose"]["title"], "title");
        assert!(body["propose"].get("msg").is_none());
    }
}
