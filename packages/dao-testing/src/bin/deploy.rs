//! Lab DAO matrix CLI — plan / bind checklist / export-state.
//!
//! Full chain `stack` execute is G4 (see docs/DESIGN-dao-orch-ecosystem-stack.md).
//! Green spawn today: shell recipes documented in docs/DEMO-WORKFLOW.md.
//!
//! Usage:
//!   cargo run -p dao-testing --bin deploy -- stack-plan --profile packages/dao-testing/profiles/open-lab.stack.toml
//!   cargo run -p dao-testing --bin deploy -- export-state --scenario open-lab
//!   cargo run -p dao-testing --bin deploy -- bind --profile …
//!   cargo run -p dao-testing --bin deploy -- stack --profile …   # prints deferred checklist

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return;
    }

    // allow `deploy -- stack-plan …` style (cargo sometimes leaves `--`)
    if args.first().map(|s| s.as_str()) == Some("--") {
        args.remove(0);
    }

    let cmd = args[0].as_str();
    let rest = &args[1..];
    let result = match cmd {
        "stack-plan" | "plan" => cmd_stack_plan(rest),
        "export-state" | "export" => cmd_export_state(rest),
        "bind" => cmd_bind(rest),
        "stack" => cmd_stack(rest),
        "list-profiles" => cmd_list_profiles(),
        other => {
            eprintln!("unknown command: {other}");
            print_help();
            Err(1)
        }
    };

    if let Err(code) = result {
        process::exit(code);
    }
}

fn print_help() {
    eprintln!(
        r#"dao-testing deploy — lab DAO matrix CLI

Commands:
  stack-plan   --profile <path.toml>     Offline plan JSON (ports, params, steps)
  export-state --scenario <name>         Merge profile + APPSTATE + state.json
               [--appstate PATH] [--cw-state PATH] [-o PATH]
  bind         --profile <path.toml>     Bind checklist (no chain writes)
  stack        --profile <path.toml>     Deferred: prints G4 execute checklist
  list-profiles                          Built-in profile paths under package

Docs:
  packages/dao-testing/docs/DEMO-WORKFLOW.md
  packages/dao-testing/docs/DESIGN-dao-orch-ecosystem-stack.md

Groot2 default ports: RPC 36657 · LCD 1617 · gRPC 9390 · faucet 5000
"#
    );
}

// ── Profile schema ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
struct StackProfile {
    schema: Option<String>,
    name: String,
    description: Option<String>,
    chain: ChainSection,
    dao: DaoSection,
    governance: GovernanceSection,
    modules: Option<ModulesSection>,
    apps: Option<AppsSection>,
    code_ids: Option<BTreeMap<String, u64>>,
    export: Option<ExportSection>,
    hooks: Option<HooksSection>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ChainSection {
    chain_id: String,
    rpc: String,
    lcd: String,
    grpc: Option<String>,
    faucet: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DaoSection {
    label: String,
    recreate: Option<bool>,
    core_addr: Option<String>,
    admin_mode: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct GovernanceSection {
    anyone_may_propose: Option<bool>,
    only_members_execute: Option<bool>,
    threshold: Option<String>,
    max_voting_period_seconds: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ModulesSection {
    calendar: Option<bool>,
    marketplace: Option<bool>,
    widgets: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AppsSection {
    dao_calendar: Option<String>,
    marketplace_mirror: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ExportSection {
    appstate: Option<String>,
    scenario_key: Option<String>,
    open_dao_ptr: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct HooksSection {
    seed: Option<String>,
    fe_setup: Option<String>,
    spawn_shell: Option<String>,
}

fn load_profile(path: &Path) -> Result<StackProfile, i32> {
    let raw = fs::read_to_string(path).map_err(|e| {
        eprintln!("read profile {}: {e}", path.display());
        2
    })?;
    toml::from_str(&raw).map_err(|e| {
        eprintln!("parse profile {}: {e}", path.display());
        2
    })
}

fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == name {
            return args.get(i + 1).map(|s| s.as_str());
        }
        if let Some(rest) = args[i].strip_prefix(&format!("{name}=")) {
            return Some(rest);
        }
        i += 1;
    }
    None
}

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    // packages/dao-testing → packages → dao-contracts
    package_root()
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(package_root)
}

fn terp_core_root() -> PathBuf {
    // dao-contracts → crates → terp-core
    workspace_root()
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(workspace_root)
}

fn resolve_profile_arg(args: &[String]) -> Result<PathBuf, i32> {
    let p = flag_value(args, "--profile").ok_or_else(|| {
        eprintln!("missing --profile <path.toml>");
        2
    })?;
    let path = PathBuf::from(p);
    if path.is_file() {
        return Ok(path);
    }
    // try relative to package / cwd
    let candidates = [
        package_root().join(p),
        package_root().join("profiles").join(p),
        workspace_root().join(p),
        env::current_dir().unwrap_or_default().join(p),
    ];
    for c in candidates {
        if c.is_file() {
            return Ok(c);
        }
    }
    eprintln!("profile not found: {p}");
    Err(2)
}

fn profile_for_scenario(scenario: &str) -> Result<PathBuf, i32> {
    let name = match scenario {
        "open-lab" | "open_lab" | "open-lab-dao" => "open-lab.stack.toml",
        "community-core" | "community_core" | "community-core-local" => {
            "community-core-local.stack.toml"
        }
        other => {
            eprintln!("unknown scenario '{other}' (open-lab | community-core)");
            return Err(2);
        }
    };
    let p = package_root().join("profiles").join(name);
    if p.is_file() {
        Ok(p)
    } else {
        eprintln!("missing profile {}", p.display());
        Err(2)
    }
}

// ── Commands ────────────────────────────────────────────────────────────────

fn cmd_stack_plan(args: &[String]) -> Result<(), i32> {
    let path = resolve_profile_arg(args)?;
    let profile = load_profile(&path)?;
    let plan = build_plan(&profile, &path);
    println!("{}", serde_json::to_string_pretty(&plan).unwrap());
    Ok(())
}

fn build_plan(profile: &StackProfile, path: &Path) -> Value {
    let recreate = profile.dao.recreate.unwrap_or(false);
    let mut steps = vec![
        json!({
            "id": "env",
            "action": "export_chain_env",
            "env": {
                "CHAIN_ID": profile.chain.chain_id,
                "RPC": profile.chain.rpc,
                "LCD": profile.chain.lcd,
                "NODE": profile.chain.rpc.replace("http://", "tcp://").replace("https://", "tcp://"),
                "GRPC": profile.chain.grpc,
                "FAUCET": profile.chain.faucet,
            }
        }),
        json!({
            "id": "codes",
            "action": "ensure_code_ids",
            "source": "~/.cw-orchestrator/state.json",
            "code_ids": profile.code_ids,
        }),
    ];

    if recreate || profile.dao.core_addr.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        steps.push(json!({
            "id": "recreate_or_create",
            "action": "spawn_dao",
            "shell": profile.hooks.as_ref().and_then(|h| h.spawn_shell.clone()),
            "governance": profile.governance,
            "label": profile.dao.label,
        }));
    } else {
        steps.push(json!({
            "id": "bind_existing",
            "action": "bind_core",
            "core_addr": profile.dao.core_addr,
        }));
    }

    steps.push(json!({
        "id": "apps",
        "action": "attach_apps",
        "apps": profile.apps,
        "modules": profile.modules,
    }));
    steps.push(json!({
        "id": "seed",
        "action": "populate_and_fe_setup",
        "hooks": profile.hooks,
    }));
    steps.push(json!({
        "id": "export",
        "action": "export_appstate",
        "export": profile.export,
    }));

    json!({
        "schema": "dao-testing/stack-plan@1",
        "profile": profile.name,
        "profile_path": path.display().to_string(),
        "description": profile.description,
        "chain": profile.chain,
        "dao": profile.dao,
        "governance": profile.governance,
        "steps": steps,
        "notes": [
            "stack-plan is offline. Chain execute remains shell (G4: deploy stack).",
            "Ports SoR: groot2 36657/1617/9390/5000 — see DEMO-WORKFLOW.md",
        ]
    })
}

fn cmd_bind(args: &[String]) -> Result<(), i32> {
    let path = resolve_profile_arg(args)?;
    let profile = load_profile(&path)?;
    let out = json!({
        "schema": "dao-testing/bind-checklist@1",
        "profile": profile.name,
        "bind": {
            "chain_id": profile.chain.chain_id,
            "rpc": profile.chain.rpc,
            "lcd": profile.chain.lcd,
            "core_addr": profile.dao.core_addr,
            "apps": profile.apps,
            "code_ids": profile.code_ids,
        },
        "checklist": [
            "curl $RPC/status → network == chain_id",
            "state.json has code_ids for core/proposal/voting/calendar/marketplace",
            "core_addr contracts exist on LCD",
            "APPSTATE addresses match profile (or export after recreate)",
            "FE CHAIN_ENDPOINTS + default_dao point at this core",
        ],
        "no_chain_writes": true,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    Ok(())
}

fn cmd_stack(args: &[String]) -> Result<(), i32> {
    let path = resolve_profile_arg(args)?;
    let profile = load_profile(&path)?;
    let plan = build_plan(&profile, &path);
    let out = json!({
        "schema": "dao-testing/stack-execute@deferred",
        "status": "deferred_g4",
        "message": "Full cw-orch stack execute is G4. Use shell hooks from the plan.",
        "plan": plan,
        "shell_now": {
            "spawn": profile.hooks.as_ref().and_then(|h| h.spawn_shell.clone()),
            "fe_setup": profile.hooks.as_ref().and_then(|h| h.fe_setup.clone()),
            "seed": profile.hooks.as_ref().and_then(|h| h.seed.clone()),
        },
        "docs": "packages/dao-testing/docs/DEMO-WORKFLOW.md",
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    Ok(())
}

fn cmd_export_state(args: &[String]) -> Result<(), i32> {
    let scenario = flag_value(args, "--scenario").unwrap_or("open-lab");
    let profile_path = if let Some(p) = flag_value(args, "--profile") {
        PathBuf::from(p)
    } else {
        profile_for_scenario(scenario)?
    };
    let profile = load_profile(&profile_path)?;

    let appstate_path = flag_value(args, "--appstate")
        .map(PathBuf::from)
        .or_else(|| {
            profile
                .export
                .as_ref()
                .and_then(|e| e.appstate.as_ref())
                .map(|rel| terp_core_root().join(rel))
        })
        .unwrap_or_else(|| {
            terp_core_root().join("artifacts/community-core-local/APPSTATE.json")
        });

    let cw_state_path = flag_value(args, "--cw-state")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs_state_home().join("state.json")
        });

    let appstate: Value = if appstate_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&appstate_path).map_err(|e| {
            eprintln!("read appstate: {e}");
            2
        })?)
        .unwrap_or(Value::Null)
    } else {
        eprintln!("warn: APPSTATE missing at {}", appstate_path.display());
        Value::Null
    };

    let mut code_ids = profile.code_ids.clone().unwrap_or_default();
    let mut state_addrs = BTreeMap::<String, String>::new();
    if cw_state_path.is_file() {
        if let Ok(raw) = fs::read_to_string(&cw_state_path) {
            if let Ok(v) = serde_json::from_str::<Value>(&raw) {
                if let Some(chain) = v.get(&profile.chain.chain_id) {
                    if let Some(obj) = chain.get("code_ids").and_then(|c| c.as_object()) {
                        for (k, val) in obj {
                            if let Some(n) = val.as_u64() {
                                code_ids.insert(k.clone(), n);
                            } else if let Some(s) = val.as_str() {
                                if let Ok(n) = s.parse::<u64>() {
                                    code_ids.insert(k.clone(), n);
                                }
                            }
                        }
                    }
                    if let Some(def) = chain.get("default").and_then(|d| d.as_object()) {
                        for (k, val) in def {
                            if let Some(s) = val.as_str() {
                                state_addrs.insert(k.clone(), s.to_string());
                            }
                        }
                    }
                }
            }
        }
    } else {
        eprintln!("warn: cw-orch state missing at {}", cw_state_path.display());
    }

    let snapshot = json!({
        "schema": "dao-testing/export-state@1",
        "exported_at_hint": "local-matrix",
        "scenario": scenario,
        "profile": profile.name,
        "profile_path": profile_path.display().to_string(),
        "chain": profile.chain,
        "dao": profile.dao,
        "governance": profile.governance,
        "apps": profile.apps,
        "code_ids": code_ids,
        "state_default_addrs": state_addrs,
        "appstate_path": appstate_path.display().to_string(),
        "appstate": appstate,
        "hooks": profile.hooks,
    });

    let pretty = serde_json::to_string_pretty(&snapshot).unwrap();
    if let Some(out) = flag_value(args, "-o").or_else(|| flag_value(args, "--out")) {
        fs::write(out, &pretty).map_err(|e| {
            eprintln!("write {out}: {e}");
            2
        })?;
        eprintln!("wrote {out}");
    } else {
        println!("{pretty}");
    }
    Ok(())
}

fn dirs_state_home() -> PathBuf {
    if let Ok(h) = env::var("HOME") {
        PathBuf::from(h).join(".cw-orchestrator")
    } else {
        PathBuf::from(".cw-orchestrator")
    }
}

fn cmd_list_profiles() -> Result<(), i32> {
    let dir = package_root().join("profiles");
    let mut list = vec![];
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("toml") {
                list.push(p.display().to_string());
            }
        }
    }
    list.sort();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "profiles_dir": dir.display().to_string(),
            "profiles": list,
        }))
        .unwrap()
    );
    Ok(())
}
