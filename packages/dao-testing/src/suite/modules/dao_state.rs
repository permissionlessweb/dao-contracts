use std::collections::BTreeMap;

use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};

/// A module entry stored by key. Tracks address, category, and optional metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleEntry {
    pub addr: Addr,
    pub category: ModuleCategory,
    /// Arbitrary key-value metadata (code_id, label, version, etc.)
    pub metadata: BTreeMap<String, String>,
}

/// Categories for module classification. Enables filtered lookups.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleCategory {
    Proposal,
    PrePropose,
    Voting,
    Staking,
    Distribution,
    External,
    Custom(String),
}

/// Keyed module storage. Save/get/remove modules by a user-chosen key.
/// Supports category-filtered listing.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModuleRegistry {
    modules: BTreeMap<String, ModuleEntry>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Save a module under the given key. Overwrites if key exists.
    pub fn save(&mut self, key: impl Into<String>, entry: ModuleEntry) {
        self.modules.insert(key.into(), entry);
    }

    /// Quick save: just key, addr, and category.
    pub fn save_module(
        &mut self,
        key: impl Into<String>,
        addr: Addr,
        category: ModuleCategory,
    ) {
        self.modules.insert(
            key.into(),
            ModuleEntry {
                addr,
                category,
                metadata: BTreeMap::new(),
            },
        );
    }

    /// Get a module by key.
    pub fn get(&self, key: &str) -> Option<&ModuleEntry> {
        self.modules.get(key)
    }

    /// Get just the address by key.
    pub fn get_addr(&self, key: &str) -> Option<&Addr> {
        self.modules.get(key).map(|e| &e.addr)
    }

    /// Remove a module by key.
    pub fn remove(&mut self, key: &str) -> Option<ModuleEntry> {
        self.modules.remove(key)
    }

    /// List all module keys.
    pub fn keys(&self) -> Vec<&String> {
        self.modules.keys().collect()
    }

    /// List all modules filtered by category.
    pub fn by_category(&self, category: &ModuleCategory) -> Vec<(&String, &ModuleEntry)> {
        self.modules
            .iter()
            .filter(|(_, e)| &e.category == category)
            .collect()
    }

    /// List all module entries.
    pub fn list(&self) -> Vec<(&String, &ModuleEntry)> {
        self.modules.iter().collect()
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

/// Snapshot of a single deployed DAO's contract addresses and modules.
/// Serializable so it can be saved to disk or kept in memory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DaoSnapshot {
    pub key: String,
    pub core_addr: Addr,
    pub voting_module: Option<Addr>,
    pub proposal_modules: Vec<ProposalModuleEntry>,
    /// Per-DAO keyed module storage for granular save/get by key.
    pub modules: ModuleRegistry,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProposalModuleEntry {
    pub pre_propose: Option<Addr>,
    pub proposal: Addr,
    pub prefix: String,
}

/// In-memory registry of DAO snapshots keyed by a user-chosen string.
/// Also holds a global module registry for modules not tied to a specific DAO.
#[derive(Clone, Debug, Default)]
pub struct DaoStateRegistry {
    daos: BTreeMap<String, DaoSnapshot>,
    /// Global module storage — modules that exist outside any specific DAO
    /// (e.g. shared factories, external contracts, test helpers).
    pub global_modules: ModuleRegistry,
}

impl DaoStateRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // ═══════════════════ DAO-level operations ═══════════════════

    /// Save a DAO snapshot under the given key. Overwrites if key exists.
    pub fn save(&mut self, snapshot: DaoSnapshot) {
        self.daos.insert(snapshot.key.clone(), snapshot);
    }

    /// Load a DAO snapshot by key. Returns None if not found.
    pub fn load(&self, key: &str) -> Option<&DaoSnapshot> {
        self.daos.get(key)
    }

    /// Load a mutable DAO snapshot by key for in-place module edits.
    pub fn load_mut(&mut self, key: &str) -> Option<&mut DaoSnapshot> {
        self.daos.get_mut(key)
    }

    /// Remove a DAO snapshot by key.
    pub fn remove(&mut self, key: &str) -> Option<DaoSnapshot> {
        self.daos.remove(key)
    }

    /// List all saved DAO keys.
    pub fn keys(&self) -> Vec<&String> {
        self.daos.keys().collect()
    }

    /// List all saved snapshots.
    pub fn list(&self) -> Vec<&DaoSnapshot> {
        self.daos.values().collect()
    }

    pub fn len(&self) -> usize {
        self.daos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.daos.is_empty()
    }

    // ═══════════════════ Per-DAO module operations ═══════════════════

    /// Save a module under a specific DAO's module registry.
    pub fn save_module(
        &mut self,
        dao_key: &str,
        module_key: impl Into<String>,
        addr: Addr,
        category: ModuleCategory,
    ) -> Result<(), String> {
        let dao = self
            .daos
            .get_mut(dao_key)
            .ok_or_else(|| format!("no DAO saved under key '{}'", dao_key))?;
        dao.modules.save_module(module_key, addr, category);
        Ok(())
    }

    /// Get a module from a specific DAO's registry by key.
    pub fn get_module(&self, dao_key: &str, module_key: &str) -> Option<&ModuleEntry> {
        self.daos
            .get(dao_key)
            .and_then(|dao| dao.modules.get(module_key))
    }

    /// Get just the address of a module from a DAO.
    pub fn get_module_addr(&self, dao_key: &str, module_key: &str) -> Option<&Addr> {
        self.daos
            .get(dao_key)
            .and_then(|dao| dao.modules.get_addr(module_key))
    }

    /// Remove a module from a specific DAO's registry.
    pub fn remove_module(
        &mut self,
        dao_key: &str,
        module_key: &str,
    ) -> Option<ModuleEntry> {
        self.daos
            .get_mut(dao_key)
            .and_then(|dao| dao.modules.remove(module_key))
    }

    /// List all modules for a DAO, optionally filtered by category.
    pub fn list_modules(
        &self,
        dao_key: &str,
        category: Option<&ModuleCategory>,
    ) -> Vec<(&String, &ModuleEntry)> {
        match self.daos.get(dao_key) {
            Some(dao) => match category {
                Some(cat) => dao.modules.by_category(cat),
                None => dao.modules.list(),
            },
            None => vec![],
        }
    }

    // ═══════════════════ Persistence ═══════════════════

    /// Persist the entire registry to a JSON file.
    pub fn save_to_file(&self, path: &str) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(&self.daos)?;
        std::fs::write(path, json)
    }

    /// Load the entire registry from a JSON file.
    pub fn load_from_file(path: &str) -> Result<Self, std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        let daos: BTreeMap<String, DaoSnapshot> = serde_json::from_str(&json)?;
        Ok(Self {
            daos,
            global_modules: ModuleRegistry::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Addr;

    #[test]
    fn test_registry_crud() {
        let mut reg = DaoStateRegistry::new();
        assert!(reg.is_empty());

        let snap = DaoSnapshot {
            key: "dao1".to_string(),
            core_addr: Addr::unchecked("core1"),
            voting_module: Some(Addr::unchecked("voting1")),
            proposal_modules: vec![ProposalModuleEntry {
                pre_propose: Some(Addr::unchecked("pp1")),
                proposal: Addr::unchecked("prop1"),
                prefix: "A".to_string(),
            }],
            modules: ModuleRegistry::new(),
        };

        reg.save(snap.clone());
        assert_eq!(reg.len(), 1);
        assert!(reg.load("dao1").is_some());
        assert_eq!(
            reg.load("dao1").unwrap().core_addr,
            Addr::unchecked("core1")
        );

        // overwrite
        let mut snap2 = snap;
        snap2.core_addr = Addr::unchecked("core2");
        reg.save(snap2);
        assert_eq!(reg.len(), 1);
        assert_eq!(
            reg.load("dao1").unwrap().core_addr,
            Addr::unchecked("core2")
        );

        // remove
        let removed = reg.remove("dao1");
        assert!(removed.is_some());
        assert!(reg.is_empty());
    }

    #[test]
    fn test_registry_multi_dao() {
        let mut reg = DaoStateRegistry::new();

        for i in 0..5 {
            reg.save(DaoSnapshot {
                key: format!("dao{}", i),
                core_addr: Addr::unchecked(format!("core{}", i)),
                voting_module: None,
                proposal_modules: vec![],
                modules: ModuleRegistry::new(),
            });
        }

        assert_eq!(reg.len(), 5);
        assert_eq!(reg.keys().len(), 5);
        assert!(reg.load("dao3").is_some());
        assert!(reg.load("nonexistent").is_none());
    }

    #[test]
    fn test_per_dao_module_registry() {
        let mut reg = DaoStateRegistry::new();

        reg.save(DaoSnapshot {
            key: "dao1".to_string(),
            core_addr: Addr::unchecked("core1"),
            voting_module: None,
            proposal_modules: vec![],
            modules: ModuleRegistry::new(),
        });

        // Save modules to the DAO
        reg.save_module(
            "dao1",
            "my_calendar",
            Addr::unchecked("calendar_addr"),
            ModuleCategory::External,
        )
        .unwrap();

        reg.save_module(
            "dao1",
            "rewards_v1",
            Addr::unchecked("rewards_addr"),
            ModuleCategory::Distribution,
        )
        .unwrap();

        reg.save_module(
            "dao1",
            "cw4_voting",
            Addr::unchecked("voting_addr"),
            ModuleCategory::Voting,
        )
        .unwrap();

        // Get by key
        let cal = reg.get_module("dao1", "my_calendar").unwrap();
        assert_eq!(cal.addr, Addr::unchecked("calendar_addr"));
        assert_eq!(cal.category, ModuleCategory::External);

        // Get addr shorthand
        assert_eq!(
            reg.get_module_addr("dao1", "rewards_v1"),
            Some(&Addr::unchecked("rewards_addr"))
        );

        // Filter by category
        let externals = reg.list_modules("dao1", Some(&ModuleCategory::External));
        assert_eq!(externals.len(), 1);
        assert_eq!(externals[0].0, "my_calendar");

        let all = reg.list_modules("dao1", None);
        assert_eq!(all.len(), 3);

        // Remove
        let removed = reg.remove_module("dao1", "my_calendar");
        assert!(removed.is_some());
        assert_eq!(reg.list_modules("dao1", None).len(), 2);

        // Nonexistent DAO returns empty
        assert!(reg.get_module("nonexistent", "my_calendar").is_none());
        assert!(reg.save_module("nonexistent", "x", Addr::unchecked("y"), ModuleCategory::Custom("test".into())).is_err());
    }

    #[test]
    fn test_global_modules() {
        let mut reg = DaoStateRegistry::new();

        reg.global_modules.save_module(
            "admin_factory",
            Addr::unchecked("factory1"),
            ModuleCategory::External,
        );
        reg.global_modules.save_module(
            "shared_issuer",
            Addr::unchecked("issuer1"),
            ModuleCategory::Custom("tokenfactory".into()),
        );

        assert_eq!(reg.global_modules.len(), 2);
        assert_eq!(
            reg.global_modules.get_addr("admin_factory"),
            Some(&Addr::unchecked("factory1"))
        );

        let externals = reg.global_modules.by_category(&ModuleCategory::External);
        assert_eq!(externals.len(), 1);
    }

    #[test]
    fn test_module_metadata() {
        let mut reg = ModuleRegistry::new();

        let mut meta = BTreeMap::new();
        meta.insert("code_id".to_string(), "42".to_string());
        meta.insert("version".to_string(), "2.8.0".to_string());

        reg.save(
            "my_module",
            ModuleEntry {
                addr: Addr::unchecked("mod_addr"),
                category: ModuleCategory::Proposal,
                metadata: meta,
            },
        );

        let entry = reg.get("my_module").unwrap();
        assert_eq!(entry.metadata.get("code_id").unwrap(), "42");
        assert_eq!(entry.metadata.get("version").unwrap(), "2.8.0");
    }
}
