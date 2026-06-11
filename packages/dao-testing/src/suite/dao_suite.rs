use cosmwasm_std::{to_json_binary, Addr};
use cw_orch::prelude::*;
use dao_cw_orch::{DaoDaoCore, DaoDaoCoreDeployData};
use dao_interface::state::{Admin, ModuleInstantiateInfo};

use super::modules::{
    dao_state::{DaoSnapshot, DaoStateRegistry, ModuleRegistry, ProposalModuleEntry},
    distribution::{DaoDistributionDeployData, DaoDistributionSuite},
    external::{DaoExternalDeployData, DaoExternalSuite},
    gauges::{DaoGaugeDeployData, DaoGaugeSuite},
    proposal::{CalendarDeployData, DaoProposalDeployData, DaoProposalSuite},
    staking::{DaoStakingDeployData, DaoStakingSuite},
    voting::{DaoVotingDeployData, DaoVotingSuite},
};

pub use dao_calendar::contract::msg::ExecuteExtFns as _;

/// Top-level deploy data for a full DAO suite.
///
/// `Default` yields empty `dao` with all sub-suite data defaulted,
/// so `deploy_on(chain, DaoDaoDeployData::default())` just uploads all codes.
#[derive(Clone, Debug, Default)]
pub struct DaoDaoDeployData {
    /// Singular DAO instance to bootstrap.
    pub dao: DaoConfig,
    pub core: DaoDaoCoreDeployData,
    /// Suite-level deploy data (affects code uploads, not per-DAO).
    pub proposal: DaoProposalDeployData,
    pub voting: DaoVotingDeployData,
    pub staking: DaoStakingDeployData,
    pub distribution: DaoDistributionDeployData,
    pub external: DaoExternalDeployData,
    pub gauges: DaoGaugeDeployData,
}

/// Voting module configuration for a single DAO instance.
#[derive(Clone, Debug, PartialEq)]
pub enum VotingModuleConfig {
    /// CW4 group-based voting.
    Cw4 {
        cw4_group_code_id: u64,
        initial_members: Vec<cw4::Member>,
    },
    /// CW20 token staking-based voting.
    Cw20Staked {
        token_info: dao_voting_cw20_staked::msg::TokenInfo,
        active_threshold: Option<dao_voting::threshold::ActiveThreshold>,
    },
    /// Native/tokenfactory token staking-based voting.
    TokenStaked {
        token_info: dao_voting_token_staked::msg::TokenInfo,
        unstaking_duration: Option<cw_utils::Duration>,
        active_threshold: Option<dao_voting::threshold::ActiveThreshold>,
    },
}

impl Default for VotingModuleConfig {
    fn default() -> Self {
        VotingModuleConfig::Cw4 {
            cw4_group_code_id: 0,
            initial_members: vec![],
        }
    }
}
impl VotingModuleConfig {
    /// Build a `ModuleInstantiateInfo` from uploaded suite code IDs.
    pub fn to_module_info<Chain: CwEnv>(
        &self,
        suite: &DaoDaoSuite<Chain>,
    ) -> Result<ModuleInstantiateInfo, CwOrchError> {
        match self {
            VotingModuleConfig::Cw4 {
                cw4_group_code_id,
                initial_members,
            } => {
                let msg = dao_voting_cw4::msg::InstantiateMsg {
                    group_contract: dao_voting_cw4::msg::GroupContract::New {
                        cw4_group_code_id: match cw4_group_code_id != &0u64 {
                            true => *cw4_group_code_id,
                            false => suite.voting.cw4_group.code_id()?,
                        },
                        cw4_group_salt: None,
                        initial_members: initial_members.clone(),
                    },
                };
                Ok(ModuleInstantiateInfo {
                    code_id: suite.voting.voting_cw4.code_id()?,
                    msg: to_json_binary(&msg).map_err(|e| CwOrchError::StdErr(e.to_string()))?,
                    admin: Some(Admin::CoreModule {}),
                    funds: None,
                    label: "dao_voting_cw4".to_string(),
                    salt: None,
                })
            }
            VotingModuleConfig::Cw20Staked {
                token_info,
                active_threshold,
            } => {
                let msg = dao_voting_cw20_staked::msg::InstantiateMsg {
                    token_info: token_info.clone(),
                    active_threshold: active_threshold.clone(),
                };
                Ok(ModuleInstantiateInfo {
                    code_id: suite.voting.voting_cw20_staked.code_id()?,
                    msg: to_json_binary(&msg).map_err(|e| CwOrchError::StdErr(e.to_string()))?,
                    admin: Some(Admin::CoreModule {}),
                    funds: None,
                    label: "dao_voting_cw20_staked".to_string(),
                    salt: None,
                })
            }
            VotingModuleConfig::TokenStaked {
                token_info,
                unstaking_duration,
                active_threshold,
            } => {
                let msg = dao_voting_token_staked::msg::InstantiateMsg {
                    token_info: token_info.clone(),
                    unstaking_duration: *unstaking_duration,
                    active_threshold: active_threshold.clone(),
                };
                Ok(ModuleInstantiateInfo {
                    code_id: suite.voting.voting_token_staked.code_id()?,
                    msg: to_json_binary(&msg).map_err(|e| CwOrchError::StdErr(e.to_string()))?,
                    admin: Some(Admin::CoreModule {}),
                    funds: None,
                    label: "dao_voting_token_staked".to_string(),
                    salt: None,
                })
            }
        }
    }
}

/// Proposal module configuration for a single DAO instance.
#[derive(Clone, Debug)]
pub enum ProposalModuleConfig {
    /// Single-choice proposal module.
    Single {
        msg: dao_proposal_single::msg::InstantiateMsg,
    },
    /// Multiple-choice proposal module.
    Multiple {
        msg: dao_proposal_multiple::msg::InstantiateMsg,
    },
    /// Calendar registered as a proposal module.
    Calendar(CalendarDeployData),
}

impl ProposalModuleConfig {
    /// Build a `ModuleInstantiateInfo` from uploaded suite code IDs.
    pub fn to_module_info<Chain: CwEnv>(
        &self,
        suite: &DaoDaoSuite<Chain>,
    ) -> Result<ModuleInstantiateInfo, CwOrchError> {
        match self {
            ProposalModuleConfig::Single { msg } => Ok(ModuleInstantiateInfo {
                code_id: suite.proposal.prop_single.code_id()?,
                msg: to_json_binary(msg).map_err(|e| CwOrchError::StdErr(e.to_string()))?,
                admin: Some(Admin::CoreModule {}),
                funds: None,
                label: "dao_proposal_single".to_string(),
                salt: None,
            }),
            ProposalModuleConfig::Multiple { msg } => Ok(ModuleInstantiateInfo {
                code_id: suite.proposal.prop_multiple.code_id()?,
                msg: to_json_binary(msg).map_err(|e| CwOrchError::StdErr(e.to_string()))?,
                admin: Some(Admin::CoreModule {}),
                funds: None,
                label: "dao_proposal_multiple".to_string(),
                salt: None,
            }),
            ProposalModuleConfig::Calendar(cal_data) => {
                use super::deploy_data::DaoDeployData;
                Ok(ModuleInstantiateInfo {
                    code_id: suite.proposal.calendar.code_id()?,
                    msg: to_json_binary(&cal_data.clone().into_init())
                        .map_err(|e| CwOrchError::StdErr(e.to_string()))?,
                    admin: Some(Admin::CoreModule {}),
                    funds: None,
                    label: "dao_calendar".to_string(),
                    salt: None,
                })
            }
        }
    }
}

/// Per-DAO instance configuration.
#[derive(Clone, Debug, Default)]
pub struct DaoConfig {
    /// Registry key for this DAO.
    pub key: String,
    /// Optional admin address.
    pub admin: Option<String>,
    /// DAO name.
    pub name: String,
    /// DAO description.
    pub description: String,
    /// Voting module configuration.
    pub voting: VotingModuleConfig,
    /// Proposal modules to register.
    pub proposal_modules: Vec<ProposalModuleConfig>,
}

/// Full DAO DAO cw-orch testing suite.
pub struct DaoDaoSuite<Chain: CwEnv + TxHandler> {
    pub dao_core: DaoDaoCore<Chain>,
    pub proposal: DaoProposalSuite<Chain>,
    pub voting: DaoVotingSuite<Chain>,
    pub staking: DaoStakingSuite<Chain>,
    pub distribution: DaoDistributionSuite<Chain>,
    pub external: DaoExternalSuite<Chain>,
    pub gauges: DaoGaugeSuite<Chain>,
    pub registry: DaoStateRegistry,
}

impl<Chain: CwEnv> DaoDaoSuite<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            dao_core: DaoDaoCore::new("dao_dao_core", chain.clone()),
            proposal: DaoProposalSuite::new(chain.clone()),
            voting: DaoVotingSuite::new(chain.clone()),
            staking: DaoStakingSuite::new(chain.clone()),
            distribution: DaoDistributionSuite::new(chain.clone()),
            external: DaoExternalSuite::new(chain.clone()),
            gauges: DaoGaugeSuite::new(chain.clone()),
            registry: DaoStateRegistry::new(),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.dao_core.upload()?;
        self.proposal.upload()?;
        self.voting.upload()?;
        self.staking.upload()?;
        self.distribution.upload()?;
        self.external.upload()?;
        self.gauges.upload()?;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
// DAO State Management
// ═══════════════════════════════════════════════════════════════════════
impl<Chain: CwEnv> DaoDaoSuite<Chain> {
    pub fn save_dao(&mut self, key: impl Into<String>, core_addr: Addr) {
        self.registry.save(DaoSnapshot {
            key: key.into(),
            core_addr,
            voting_module: None,
            proposal_modules: vec![],
            modules: ModuleRegistry::default(),
        });
    }

    pub fn save_dao_full(
        &mut self,
        key: impl Into<String>,
        core_addr: Addr,
        voting_module: Option<Addr>,
        proposal_modules: Vec<ProposalModuleEntry>,
    ) {
        self.registry.save(DaoSnapshot {
            key: key.into(),
            core_addr,
            voting_module,
            proposal_modules,
            modules: ModuleRegistry::default(),
        });
    }

    pub fn load_dao(&self, key: &str) -> Option<&DaoSnapshot> {
        self.registry.load(key)
    }

    pub fn remove_dao(&mut self, key: &str) -> Option<DaoSnapshot> {
        self.registry.remove(key)
    }

    pub fn hotswap_dao(&mut self, key: &str) -> Result<&DaoSnapshot, CwOrchError> {
        let snapshot = self
            .registry
            .load(key)
            .ok_or_else(|| CwOrchError::StdErr(format!("no DAO saved under key '{}'", key)))?;
        self.dao_core.set_address(&snapshot.core_addr);
        Ok(snapshot)
    }

    pub fn list_daos(&self) -> Vec<&String> {
        self.registry.keys()
    }

    pub fn save_registry_to_file(&self, path: &str) -> Result<(), std::io::Error> {
        self.registry.save_to_file(path)
    }

    pub fn load_registry_from_file(&mut self, path: &str) -> Result<(), std::io::Error> {
        self.registry = DaoStateRegistry::load_from_file(path)?;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Execute As DAO
// ═══════════════════════════════════════════════════════════════════════
impl<Chain: CwEnv> DaoDaoSuite<Chain>
where
    Addr: Into<Chain::Sender>,
{
    pub fn execute_as_dao(
        &self,
        msgs: Vec<cosmwasm_std::CosmosMsg<cosmwasm_std::Empty>>,
    ) -> Result<Chain::Response, CwOrchError> {
        let dao_addr = self.dao_core.address()?;
        self.dao_core.call_as(&dao_addr.into()).execute(
            &dao_interface::msg::ExecuteMsg::ExecuteAdminMsgs { msgs },
            &[],
        )
    }

    pub fn execute_as_dao_single(
        &self,
        msg: cosmwasm_std::CosmosMsg<cosmwasm_std::Empty>,
    ) -> Result<Chain::Response, CwOrchError> {
        self.execute_as_dao(vec![msg])
    }

    pub fn execute_as_dao_by_key(
        &mut self,
        key: &str,
        msgs: Vec<cosmwasm_std::CosmosMsg<cosmwasm_std::Empty>>,
    ) -> Result<Chain::Response, CwOrchError> {
        let snapshot = self
            .registry
            .load(key)
            .ok_or_else(|| CwOrchError::StdErr(format!("no DAO saved under key '{}'", key)))?
            .clone();
        let prev_addr = self.dao_core.address().ok();
        self.dao_core.set_address(&snapshot.core_addr);
        let result = self
            .dao_core
            .call_as(&snapshot.core_addr.clone().into())
            .execute(
                &dao_interface::msg::ExecuteMsg::ExecuteAdminMsgs { msgs },
                &[],
            );
        if let Some(addr) = prev_addr {
            self.dao_core.set_address(&addr);
        }
        result
    }

    pub fn execute_proposal_hook(
        &self,
        proposal_module_addr: Addr,
        msgs: Vec<cosmwasm_std::CosmosMsg<cosmwasm_std::Empty>>,
    ) -> Result<Chain::Response, CwOrchError> {
        self.dao_core.call_as(&proposal_module_addr.into()).execute(
            &dao_interface::msg::ExecuteMsg::ExecuteProposalHook { msgs },
            &[],
        )
    }

    pub fn execute_proposal_hook_by_key(
        &mut self,
        key: &str,
        proposal_module_addr: Addr,
        msgs: Vec<cosmwasm_std::CosmosMsg<cosmwasm_std::Empty>>,
    ) -> Result<Chain::Response, CwOrchError> {
        let snapshot = self
            .registry
            .load(key)
            .ok_or_else(|| CwOrchError::StdErr(format!("no DAO saved under key '{}'", key)))?
            .clone();
        let prev_addr = self.dao_core.address().ok();
        self.dao_core.set_address(&snapshot.core_addr);
        let result = self.dao_core.call_as(&proposal_module_addr.into()).execute(
            &dao_interface::msg::ExecuteMsg::ExecuteProposalHook { msgs },
            &[],
        );
        if let Some(addr) = prev_addr {
            self.dao_core.set_address(&addr);
        }
        result
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Query Helpers
// ═══════════════════════════════════════════════════════════════════════
impl<Chain: CwEnv> DaoDaoSuite<Chain> {
    pub fn query_config(&self) -> Result<dao_interface::state::Config, CwOrchError> {
        self.dao_core
            .query(&dao_interface::msg::QueryMsg::Config {})
    }

    pub fn query_voting_module(&self) -> Result<Addr, CwOrchError> {
        self.dao_core
            .query(&dao_interface::msg::QueryMsg::VotingModule {})
    }

    pub fn query_proposal_modules(
        &self,
    ) -> Result<Vec<dao_interface::state::ProposalModule>, CwOrchError> {
        self.dao_core
            .query(&dao_interface::msg::QueryMsg::ProposalModules {
                start_after: None,
                limit: None,
            })
    }

    pub fn query_admin(&self) -> Result<Addr, CwOrchError> {
        self.dao_core.query(&dao_interface::msg::QueryMsg::Admin {})
    }

    pub fn query_total_power(
        &self,
    ) -> Result<dao_interface::voting::TotalPowerAtHeightResponse, CwOrchError> {
        self.dao_core
            .query(&dao_interface::msg::QueryMsg::TotalPowerAtHeight { height: None })
    }

    pub fn query_voting_power(
        &self,
        address: impl Into<String>,
    ) -> Result<dao_interface::voting::VotingPowerAtHeightResponse, CwOrchError> {
        self.dao_core
            .query(&dao_interface::msg::QueryMsg::VotingPowerAtHeight {
                address: address.into(),
                height: None,
            })
    }

    pub fn query_pause_info(&self) -> Result<dao_interface::query::PauseInfoResponse, CwOrchError> {
        self.dao_core
            .query(&dao_interface::msg::QueryMsg::PauseInfo {})
    }

    pub fn snapshot_current_dao(&mut self, key: impl Into<String>) -> Result<(), CwOrchError> {
        let core_addr = self.dao_core.address()?;
        let voting_module = self.query_voting_module().ok();
        let proposal_modules = self
            .query_proposal_modules()
            .unwrap_or_default()
            .into_iter()
            .map(|pm| ProposalModuleEntry {
                pre_propose: None,
                proposal: pm.address,
                prefix: pm.prefix,
            })
            .collect();
        self.save_dao_full(key, core_addr, voting_module, proposal_modules);
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Deploy trait
// ═══════════════════════════════════════════════════════════════════════
impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoDaoSuite<Chain> {
    type Error = CwOrchError;
    type DeployData = DaoDaoDeployData;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let suite = Self::new(chain);
        suite.upload()?;
        Ok(suite)
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        let mut cs: Vec<Box<&mut dyn ContractInstance<Chain>>> = vec![Box::new(&mut self.dao_core)];
        cs.extend(self.proposal.get_contracts_mut());
        cs.extend(self.voting.get_contracts_mut());
        cs.extend(self.staking.get_contracts_mut());
        cs.extend(self.distribution.get_contracts_mut());
        cs.extend(self.external.get_contracts_mut());
        cs.extend(self.gauges.get_contracts_mut());
        cs
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        Ok(Self::new(chain))
    }

    fn deploy_on(chain: Chain, data: Self::DeployData) -> Result<Self, Self::Error> {
        let dao_cfg = data.dao.clone();
        let dao_core = DaoDaoCore::new("dao_dao_core", chain.clone());
        dao_core.upload()?;
        let mut suite = Self {
            dao_core,
            proposal: DaoProposalSuite::deploy_on(chain.clone(), data.proposal)?,
            voting: DaoVotingSuite::deploy_on(chain.clone(), data.voting)?,
            staking: DaoStakingSuite::deploy_on(chain.clone(), data.staking)?,
            distribution: DaoDistributionSuite::deploy_on(chain.clone(), data.distribution)?,
            external: DaoExternalSuite::deploy_on(chain.clone(), data.external)?,
            gauges: DaoGaugeSuite::deploy_on(chain, data.gauges)?,
            registry: DaoStateRegistry::new(),
        };

        // onlt instantiate if voting module differs from default
        match dao_cfg.voting == Default::default() {
            false => {
                let proposal_infos: Vec<ModuleInstantiateInfo> = dao_cfg
                    .proposal_modules
                    .iter()
                    .map(|pm| pm.to_module_info(&suite))
                    .collect::<Result<_, _>>()?;

                let init_msg = dao_interface::msg::InstantiateMsg {
                    admin: dao_cfg.admin.clone(),
                    name: dao_cfg.name,
                    description: dao_cfg.description,
                    image_url: None,
                    automatically_add_cw20s: true,
                    automatically_add_cw721s: true,
                    voting_module_instantiate_info: dao_cfg.voting.to_module_info(&suite)?,
                    proposal_modules_instantiate_info: proposal_infos,
                    initial_items: None,
                    initial_actions: None,
                    dao_uri: None,
                };

                suite.dao_core.instantiate(
                    &init_msg,
                    dao_cfg.admin.as_deref().map(Addr::unchecked).as_ref(),
                    &[],
                )?;
                let core_addr = suite.dao_core.address()?;
                suite.save_dao(&dao_cfg.key, core_addr);
                Ok(suite)
            }
            true => Ok(suite),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Markdown API doc generator — reads cosmwasm-schema JSON from disk
// ═══════════════════════════════════════════════════════════════════════

/// Extract enum variant names + descriptions from a schema section's `oneOf`.
fn extract_variants(section: &serde_json::Value) -> Vec<(String, String)> {
    let one_of = match section.get("oneOf").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return vec![],
    };
    one_of
        .iter()
        .filter_map(|v| {
            let name = v
                .get("required")
                .and_then(|r| r.as_array())
                .and_then(|a| a.first())
                .and_then(|s| s.as_str())
                .or_else(|| {
                    v.get("enum")
                        .and_then(|e| e.as_array())
                        .and_then(|a| a.first())
                        .and_then(|s| s.as_str())
                })?;
            let desc = v.get("description").and_then(|d| d.as_str()).unwrap_or("");
            Some((name.to_string(), desc.to_string()))
        })
        .collect()
}

/// Extract instantiate fields: (name, description, required).
fn extract_instantiate_fields(section: &serde_json::Value) -> Vec<(String, String, bool)> {
    let required: Vec<String> = section
        .get("required")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let props = match section.get("properties").and_then(|v| v.as_object()) {
        Some(p) => p,
        None => return vec![],
    };
    let mut fields: Vec<_> = props
        .iter()
        .map(|(name, prop)| {
            let desc = prop
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            (name.clone(), desc.to_string(), required.contains(name))
        })
        .collect();
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    fields
}

/// `snake_case` → `PascalCase`.
fn to_pascal(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn keys_match_manifest() {
    //     let manifest = contract_manifest();
    //     let all = keys::ALL;
    //     assert_eq!(manifest.len(), all.len());
    //     for (doc, key) in manifest.iter().zip(all.iter()) {
    //         assert_eq!(doc.key, *key, "manifest/keys order mismatch at '{}'", key);
    //     }
    // }

    // #[test]
    // #[ignore] // run explicitly: cargo test -p dao-testing generate_suite_api_docs -- --ignored
    // fn generate_suite_api_docs() {
    //     let ws = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
    //         .parent()
    //         .unwrap()
    //         .parent()
    //         .unwrap();
    //     let md = generate_api_markdown(ws);
    //     let out = ws.join("SUITE_API.md");
    //     std::fs::write(&out, &md).expect("failed to write SUITE_API.md");
    //     assert!(md.contains("# DAO DAO Suite"));
    //     assert!(md.contains("dao_core"));
    //     eprintln!("wrote {} bytes → {}", md.len(), out.display());
    // }
}
