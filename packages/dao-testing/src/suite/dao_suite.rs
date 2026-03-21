use cosmwasm_std::Addr;
use cw_orch::prelude::*;
use dao_cw_orch::DaoDaoCore;

use super::modules::{
    dao_state::{DaoSnapshot, DaoStateRegistry, ModuleRegistry, ProposalModuleEntry},
    distribution::DaoDistributionSuite,
    external::DaoExternalSuite,
    proposal::DaoProposalSuite,
    staking::DaoStakingSuite,
    voting::DaoVotingSuite,
};

// ═══════════════════════════════════════════════════════════════════════
// ContractDoc — static metadata for a single contract interface
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ContractDoc {
    pub key: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub schema_path: &'static str,
}

// ═══════════════════════════════════════════════════════════════════════
// define_suite! — single source of truth for the contract registry
// ═══════════════════════════════════════════════════════════════════════
//
// Generates from one declaration:
//   pub mod keys        — &str constants + ALL slice
//   get_contract()      — &dyn ContractInstance dispatch
//   get_contract_mut()  — &mut dyn ContractInstance dispatch
//   set_contract_addr() — set address by key
//   get_contract_addr() — get address by key
//   load_module_into_interface() — bridge registry → interface
//   available_keys()    — &[&str]
//   contract_manifest() — Vec<ContractDoc> (free fn, no generics)

macro_rules! define_suite {
    (
        $(
            $KEY:ident, $key_str:literal => {
                path: $($field:ident).+,
                name: $name:literal,
                category: $cat:literal,
                description: $desc:literal,
                schema: $schema:literal $(,)?
            }
        ),* $(,)?
    ) => {
        /// Well-known string keys for every contract interface in the suite.
        pub mod keys {
            $( pub const $KEY: &str = $key_str; )*

            /// All keys in declaration order.
            pub const ALL: &[&str] = &[$( $key_str ),*];
        }

        impl<Chain: CwEnv> DaoDaoSuite<Chain> {
            /// Get any contract interface by key.
            pub fn get_contract(&self, key: &str) -> Option<&dyn ContractInstance<Chain>> {
                match key {
                    $( $key_str => Some(&self.$($field).+), )*
                    _ => None,
                }
            }

            /// Mutable variant.
            pub fn get_contract_mut(&mut self, key: &str) -> Option<&mut dyn ContractInstance<Chain>> {
                match key {
                    $( $key_str => Some(&mut self.$($field).+), )*
                    _ => None,
                }
            }

            /// Set address for a contract interface by key.
            pub fn set_contract_addr(&mut self, key: &str, addr: &Addr) -> Result<(), CwOrchError> {
                self.get_contract_mut(key)
                    .ok_or_else(|| CwOrchError::StdErr(format!("unknown contract key '{}'", key)))?
                    .set_address(addr);
                Ok(())
            }

            /// Get address for a contract interface by key.
            pub fn get_contract_addr(&self, key: &str) -> Result<Addr, CwOrchError> {
                self.get_contract(key)
                    .ok_or_else(|| CwOrchError::StdErr(format!("unknown contract key '{}'", key)))?
                    .address()
            }

            /// Load a module from the DAO state registry onto a cw-orch interface.
            pub fn load_module_into_interface(
                &mut self,
                dao_key: &str,
                module_key: &str,
                interface_key: &str,
            ) -> Result<(), CwOrchError> {
                let addr = self.registry
                    .get_module_addr(dao_key, module_key)
                    .ok_or_else(|| CwOrchError::StdErr(format!(
                        "no module '{}' in DAO '{}'", module_key, dao_key
                    )))?.clone();
                self.set_contract_addr(interface_key, &addr)
            }

            /// All registered contract keys.
            pub fn available_keys() -> &'static [&'static str] {
                keys::ALL
            }
        }

        /// Static metadata for every contract in the suite (no generics needed).
        pub fn contract_manifest() -> Vec<ContractDoc> {
            vec![
                $( ContractDoc {
                    key: $key_str,
                    name: $name,
                    category: $cat,
                    description: $desc,
                    schema_path: $schema,
                }, )*
            ]
        }
    };
}

// ═══════════════════════════════════════════════════════════════════════
// DaoDaoSuite struct + constructor + upload
// ═══════════════════════════════════════════════════════════════════════

/// Full DAO DAO cw-orch testing suite.
pub struct DaoDaoSuite<Chain: CwEnv + TxHandler> {
    pub dao_core: DaoDaoCore<Chain>,
    pub proposal: DaoProposalSuite<Chain>,
    pub voting: DaoVotingSuite<Chain>,
    pub staking: DaoStakingSuite<Chain>,
    pub distribution: DaoDistributionSuite<Chain>,
    pub external: DaoExternalSuite<Chain>,
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
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
// THE REGISTRY — add/remove contracts here, everything else auto-updates
// ═══════════════════════════════════════════════════════════════════════

define_suite! {
    // ── Core ─────────────────────────────────────────────────────
    DAO_CORE, "dao_core" => {
        path: dao_core,
        name: "DAO Core",
        category: "Core",
        description: "Core DAO contract — modules, admin, pause, sub-DAOs",
        schema: "contracts/dao-dao-core/schema/dao-dao-core.json",
    },

    // ── Proposal ─────────────────────────────────────────────────
    PROP_SINGLE, "prop_single" => {
        path: proposal.prop_single,
        name: "Proposal Single-Choice",
        category: "Proposal",
        description: "Single-choice (yes/no/abstain) proposal module",
        schema: "contracts/proposal/dao-proposal-single/schema/dao-proposal-single.json",
    },
    PROP_MULTIPLE, "prop_multiple" => {
        path: proposal.prop_multiple,
        name: "Proposal Multiple-Choice",
        category: "Proposal",
        description: "Multiple-choice proposal module with ranked options",
        schema: "contracts/proposal/dao-proposal-multiple/schema/dao-proposal-multiple.json",
    },
    PROP_CONDORCET, "prop_condorcet" => {
        path: proposal.prop_condorcet,
        name: "Proposal Condorcet",
        category: "Proposal",
        description: "Condorcet-method ranked-choice proposal module",
        schema: "contracts/proposal/dao-proposal-condorcet/schema/dao-proposal-condorcet.json",
    },
    PROP_SUDO, "prop_sudo" => {
        path: proposal.prop_sudo,
        name: "Proposal Sudo",
        category: "Proposal",
        description: "Test-only sudo proposal module",
        schema: "",
    },

    // ── Pre-Propose ──────────────────────────────────────────────
    PRE_PROP_SINGLE, "pre_prop_single" => {
        path: proposal.pre_prop_suite.pre_prop_single,
        name: "Pre-Propose Single",
        category: "Pre-Propose",
        description: "Gatekeeper for single-choice proposals — deposits, whitelists",
        schema: "contracts/pre-propose/dao-pre-propose-single/schema/dao-pre-propose-single.json",
    },
    PRE_PROP_MULTIPLE, "pre_prop_multiple" => {
        path: proposal.pre_prop_suite.pre_prop_multiple,
        name: "Pre-Propose Multiple",
        category: "Pre-Propose",
        description: "Gatekeeper for multiple-choice proposals",
        schema: "contracts/pre-propose/dao-pre-propose-multiple/schema/dao-pre-propose-multiple.json",
    },
    PRE_PROP_APPROVAL_SINGLE, "pre_prop_approval_single" => {
        path: proposal.pre_prop_suite.pre_prop_approval_single,
        name: "Pre-Propose Approval Single",
        category: "Pre-Propose",
        description: "Approval-gated pre-propose — proposals need approver sign-off",
        schema: "contracts/pre-propose/dao-pre-propose-approval-single/schema/dao-pre-propose-approval-single.json",
    },
    PRE_PROP_APPROVER, "pre_prop_approver" => {
        path: proposal.pre_prop_suite.pre_prop_approver,
        name: "Pre-Propose Approver",
        category: "Pre-Propose",
        description: "Approver contract for approval-gated pre-propose flow",
        schema: "contracts/pre-propose/dao-pre-propose-approver/schema/dao-pre-propose-approver.json",
    },

    // ── Voting ───────────────────────────────────────────────────
    VOTING_CW4, "voting_cw4" => {
        path: voting.voting_cw4,
        name: "Voting CW4",
        category: "Voting",
        description: "Voting power from CW4 group membership weights",
        schema: "contracts/voting/dao-voting-cw4/schema/dao-voting-cw4.json",
    },
    VOTING_CW20_STAKED, "voting_cw20_staked" => {
        path: voting.voting_cw20_staked,
        name: "Voting CW20 Staked",
        category: "Voting",
        description: "Voting power from staked CW20 tokens",
        schema: "contracts/voting/dao-voting-cw20-staked/schema/dao-voting-cw20-staked.json",
    },
    VOTING_CW721_ROLES, "voting_cw721_roles" => {
        path: voting.voting_cw721_roles,
        name: "Voting CW721 Roles",
        category: "Voting",
        description: "Voting power from CW721 NFTs with role-based weights",
        schema: "contracts/voting/dao-voting-cw721-roles/schema/dao-voting-cw721-roles.json",
    },
    VOTING_CW721_STAKED, "voting_cw721_staked" => {
        path: voting.voting_cw721_staked,
        name: "Voting CW721 Staked",
        category: "Voting",
        description: "Voting power from staked CW721 NFTs (1 NFT = 1 vote)",
        schema: "contracts/voting/dao-voting-cw721-staked/schema/dao-voting-cw721-staked.json",
    },
    VOTING_TOKEN_STAKED, "voting_token_staked" => {
        path: voting.voting_token_staked,
        name: "Voting Token Staked",
        category: "Voting",
        description: "Voting power from staked native/tokenfactory tokens",
        schema: "contracts/voting/dao-voting-token-staked/schema/dao-voting-token-staked.json",
    },

    // ── Staking ──────────────────────────────────────────────────
    CW20_STAKE, "cw20_stake" => {
        path: staking.cw20_stake,
        name: "CW20 Stake",
        category: "Staking",
        description: "CW20 token staking with configurable unbonding",
        schema: "contracts/staking/cw20-stake/schema/cw20-stake.json",
    },
    EXTERNAL_REWARDS, "external_rewards" => {
        path: staking.external_rewards,
        name: "CW20 External Rewards",
        category: "Staking",
        description: "External reward distribution for CW20 stakers",
        schema: "contracts/staking/cw20-stake-external-rewards/schema/cw20-stake-external-rewards.json",
    },
    REWARDS_DISTRIBUTOR, "rewards_distributor" => {
        path: staking.rewards_distributor,
        name: "CW20 Reward Distributor",
        category: "Staking",
        description: "Scheduled reward distribution for CW20 stakers",
        schema: "contracts/staking/cw20-stake-reward-distributor/schema/cw20-stake-reward-distributor.json",
    },

    // ── Distribution ─────────────────────────────────────────────
    FUND_DISTRIBUTOR, "fund_distributor" => {
        path: distribution.fund_distr,
        name: "Fund Distributor",
        category: "Distribution",
        description: "Pro-rata native/CW20 fund distribution to voters",
        schema: "contracts/distribution/cw-fund-distributor/schema/cw-fund-distributor.json",
    },
    REWARD_DISTRIBUTOR, "reward_distributor" => {
        path: distribution.reward_distr,
        name: "Rewards Distributor",
        category: "Distribution",
        description: "Continuous reward streaming to stakers/voters",
        schema: "contracts/distribution/dao-rewards-distributor/schema/dao-rewards-distributor.json",
    },

    // ── External ─────────────────────────────────────────────────
    ADMIN_FACTORY, "admin_factory" => {
        path: external.admin_factory,
        name: "Admin Factory",
        category: "External",
        description: "Factory for self-admin contract instantiation",
        schema: "contracts/external/cw-admin-factory/schema/cw-admin-factory.json",
    },
    BTSG_FT_FACTORY, "btsg_ft_factory" => {
        path: external.btsg_ft_factory,
        name: "BitSong FanToken Factory",
        category: "External",
        description: "BitSong fantoken creation and management",
        schema: "contracts/external/btsg-ft-factory/schema/btsg-ft-factory.json",
    },
    PAYROLL_FACTORY, "payroll_factory" => {
        path: external.payroll_factory,
        name: "Payroll Factory",
        category: "External",
        description: "Factory for vesting/payroll payment streams",
        schema: "contracts/external/cw-payroll-factory/schema/cw-payroll-factory.json",
    },
    CW_TOKENSWAP, "cw_tokenswap" => {
        path: external.cw_tokenswap,
        name: "Token Swap",
        category: "External",
        description: "Escrow-based two-party token swap",
        schema: "contracts/external/cw-token-swap/schema/cw-token-swap.json",
    },
    CW_TOKENFACTORY_ISSUER, "cw_tokenfactory_issuer" => {
        path: external.cw_tokenfactory_issuer,
        name: "TokenFactory Issuer",
        category: "External",
        description: "Tokenfactory denom management — mint, burn, freeze",
        schema: "contracts/external/cw-tokenfactory-issuer/schema/cw-tokenfactory-issuer.json",
    },
    CW_VESTING, "cw_vesting" => {
        path: external.cw_vesting,
        name: "CW Vesting",
        category: "External",
        description: "Token vesting with configurable curves and clawback",
        schema: "contracts/external/cw-vesting/schema/cw-vesting.json",
    },
    CW721_ROLES, "cw721_roles" => {
        path: external.cw721_roles,
        name: "CW721 Roles",
        category: "External",
        description: "CW721 NFT collection with weighted roles for governance",
        schema: "contracts/external/cw721-roles/schema/cw721-roles.json",
    },
    MIGRATOR, "migrator" => {
        path: external.migrator,
        name: "DAO Migrator",
        category: "External",
        description: "Handles DAO contract migrations across versions",
        schema: "contracts/external/dao-migrator/schema/dao-migrator.json",
    },
    CALENDAR, "calendar" => {
        path: external.calendar,
        name: "DAO Calendar",
        category: "External",
        description: "On-chain event calendar with groups, gauges, scheduling",
        schema: "schema/dao-calendar.json",
    },
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
        self.dao_core
            .query(&dao_interface::msg::QueryMsg::Admin {})
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

    pub fn query_pause_info(
        &self,
    ) -> Result<dao_interface::query::PauseInfoResponse, CwOrchError> {
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
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let suite = Self::new(chain);
        suite.upload()?;
        Ok(suite)
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        let mut cs: Vec<Box<&mut dyn ContractInstance<Chain>>> =
            vec![Box::new(&mut self.dao_core)];
        cs.extend(self.proposal.get_contracts_mut());
        cs.extend(self.voting.get_contracts_mut());
        cs.extend(self.staking.get_contracts_mut());
        cs.extend(self.distribution.get_contracts_mut());
        cs.extend(self.external.get_contracts_mut());
        cs
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        Ok(Self::new(chain))
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        Self::store_on(chain)
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
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let props = match section.get("properties").and_then(|v| v.as_object()) {
        Some(p) => p,
        None => return vec![],
    };
    let mut fields: Vec<_> = props
        .iter()
        .map(|(name, prop)| {
            let desc = prop.get("description").and_then(|d| d.as_str()).unwrap_or("");
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

/// Generate a full markdown API reference from the suite manifest + schema files.
///
/// `workspace_root`: path to the repo root containing `contracts/` and `schema/`.
pub fn generate_api_markdown(workspace_root: &std::path::Path) -> String {
    let manifest = contract_manifest();
    let mut md = String::new();

    // Header
    md.push_str("# DAO DAO Suite — API Reference\n\n");
    md.push_str("> Auto-generated from contract schemas and `DaoDaoSuite` registry.\n");
    md.push_str("> Regenerate: `cargo test -p dao-testing generate_suite_api_docs -- --ignored`\n\n");

    // Collect unique categories in declaration order
    let categories: Vec<&str> = {
        let mut cats = Vec::new();
        for doc in &manifest {
            if !cats.contains(&doc.category) {
                cats.push(doc.category);
            }
        }
        cats
    };

    // TOC
    md.push_str("## Table of Contents\n\n");
    for cat in &categories {
        md.push_str(&format!("- [{}](#{})\n", cat, cat.to_lowercase().replace(' ', "-")));
    }
    md.push('\n');

    // Summary table
    md.push_str("## Contract Summary\n\n");
    md.push_str("| Key | Name | Category | Description |\n");
    md.push_str("|-----|------|----------|-------------|\n");
    for doc in &manifest {
        md.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            doc.key, doc.name, doc.category, doc.description
        ));
    }
    md.push('\n');

    // Per-category detailed sections
    for cat in &categories {
        md.push_str(&format!("---\n\n## {}\n\n", cat));

        for doc in manifest.iter().filter(|d| d.category == *cat) {
            md.push_str(&format!("### {} (`{}`)\n\n", doc.name, doc.key));
            md.push_str(&format!("> {}\n\n", doc.description));

            if doc.schema_path.is_empty() {
                md.push_str("*No schema available.*\n\n");
                continue;
            }

            let schema: Option<serde_json::Value> =
                std::fs::read_to_string(workspace_root.join(doc.schema_path))
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok());

            let schema = match schema {
                Some(s) => s,
                None => {
                    md.push_str(&format!("*Schema not found: `{}`*\n\n", doc.schema_path));
                    continue;
                }
            };

            if let Some(ver) = schema.get("contract_version").and_then(|v| v.as_str()) {
                md.push_str(&format!("**Version**: `{}`\n\n", ver));
            }

            // InstantiateMsg
            if let Some(inst) = schema.get("instantiate") {
                let fields = extract_instantiate_fields(inst);
                if !fields.is_empty() {
                    md.push_str("#### InstantiateMsg\n\n");
                    md.push_str("| Field | Required | Description |\n");
                    md.push_str("|-------|----------|-------------|\n");
                    for (name, desc, req) in &fields {
                        md.push_str(&format!(
                            "| `{}` | {} | {} |\n",
                            name,
                            if *req { "yes" } else { "no" },
                            truncate(desc, 120)
                        ));
                    }
                    md.push('\n');
                }
            }

            // ExecuteMsg
            if let Some(exec) = schema.get("execute") {
                let variants = extract_variants(exec);
                if !variants.is_empty() {
                    md.push_str("#### ExecuteMsg\n\n");
                    md.push_str("| Variant | Description |\n");
                    md.push_str("|---------|-------------|\n");
                    for (name, desc) in &variants {
                        md.push_str(&format!(
                            "| `{}` | {} |\n",
                            to_pascal(name),
                            truncate(desc, 140)
                        ));
                    }
                    md.push('\n');
                }
            }

            // QueryMsg
            if let Some(query) = schema.get("query") {
                let variants = extract_variants(query);
                if !variants.is_empty() {
                    md.push_str("#### QueryMsg\n\n");
                    md.push_str("| Variant | Description |\n");
                    md.push_str("|---------|-------------|\n");
                    for (name, desc) in &variants {
                        md.push_str(&format!(
                            "| `{}` | {} |\n",
                            to_pascal(name),
                            truncate(desc, 140)
                        ));
                    }
                    md.push('\n');
                }
            }

            // MigrateMsg
            if let Some(mig) = schema.get("migrate") {
                if !mig.is_null() {
                    let variants = extract_variants(mig);
                    if !variants.is_empty() {
                        md.push_str("#### MigrateMsg\n\n");
                        md.push_str("| Variant | Description |\n");
                        md.push_str("|---------|-------------|\n");
                        for (name, desc) in &variants {
                            md.push_str(&format!("| `{}` | {} |\n", to_pascal(name), desc));
                        }
                        md.push('\n');
                    }
                }
            }
        }
    }

    md.push_str("---\n\n");
    md.push_str(&format!(
        "*{} contracts across {} categories.*\n",
        manifest.len(),
        categories.len()
    ));
    md
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_match_manifest() {
        let manifest = contract_manifest();
        let all = keys::ALL;
        assert_eq!(manifest.len(), all.len());
        for (doc, key) in manifest.iter().zip(all.iter()) {
            assert_eq!(doc.key, *key, "manifest/keys order mismatch at '{}'", key);
        }
    }

    #[test]
    #[ignore] // run explicitly: cargo test -p dao-testing generate_suite_api_docs -- --ignored
    fn generate_suite_api_docs() {
        let ws = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let md = generate_api_markdown(ws);
        let out = ws.join("SUITE_API.md");
        std::fs::write(&out, &md).expect("failed to write SUITE_API.md");
        assert!(md.contains("# DAO DAO Suite"));
        assert!(md.contains("dao_core"));
        eprintln!("wrote {} bytes → {}", md.len(), out.display());
    }
}
