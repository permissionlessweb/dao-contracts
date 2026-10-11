# CosmWasm DAO Testing

Common testing helpers and interfaces for DAO modules (multi-test, suite deploy data, test-tube).

## Lab / testnet / mainnet matrix (cw-orch stack profiles)

Operator surface for parameterizing DAOs across environments:

| Doc | Purpose |
|-----|---------|
| [`docs/DEMO-WORKFLOW.md`](docs/DEMO-WORKFLOW.md) | Ports + plan/stack/export commands |
| [`docs/STACK-PROFILES.md`](docs/STACK-PROFILES.md) | Profile table, code-id reuse, mainnet guards |
| [`profiles/*.stack.toml`](profiles/) | `open-lab`, `community-core-local`, `testnet`, `mainnet` |

```bash
cd crates/dao-contracts

# List profiles
cargo run -p dao-testing --bin deploy -- list-profiles

# Offline plan (lab / public testnet)
cargo run -p dao-testing --bin deploy -- stack-plan \
  --profile packages/dao-testing/profiles/open-lab.stack.toml
cargo run -p dao-testing --bin deploy -- stack-plan \
  --profile packages/dao-testing/profiles/testnet.stack.toml

# Merge APPSTATE + ~/.cw-orchestrator/state.json (upload_if_needed reuse)
cargo run -p dao-testing --bin deploy -- export-state --scenario open-lab
cargo run -p dao-testing --bin deploy -- export-state --scenario testnet
cargo run -p dao-testing --bin deploy -- export-state --scenario mainnet

# Mainnet deferred stack is guarded
ALLOW_MAINNET=1 cargo run -p dao-testing --bin deploy -- stack \
  --profile packages/dao-testing/profiles/mainnet.stack.toml --allow-mainnet
```

Green-path lab spawn remains shell recipes under `artifacts/community-core-local/`
(see DEMO-WORKFLOW). Full `deploy stack` execute is design G4. Soft-dep:
**LAB-ORCH-2** `lab-spawn ensure --profile …`.
