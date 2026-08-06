# CosmWasm DAO Testing

Common testing helpers and interfaces for DAO modules (multi-test, suite deploy data, test-tube).

## Lab matrix (cw-orch stack positioning)

Operator surface for spawning / parameterizing lab DAOs on `120u-1`:

| Doc | Purpose |
|-----|---------|
| [`docs/DEMO-WORKFLOW.md`](docs/DEMO-WORKFLOW.md) | One-page ports + plan/stack/export + 2 scenarios |
| [`docs/DESIGN-dao-orch-ecosystem-stack.md`](docs/DESIGN-dao-orch-ecosystem-stack.md) | G0–G7 design |
| [`profiles/*.stack.toml`](profiles/) | Named scenarios (`open-lab`, `community-core`) |

```bash
# Offline plan
cargo run -p dao-testing --bin deploy -- stack-plan \
  --profile packages/dao-testing/profiles/open-lab.stack.toml

# Merge APPSTATE + state.json
cargo run -p dao-testing --bin deploy -- export-state --scenario community-core

cargo run -p dao-testing --bin deploy -- list-profiles
```

Green-path chain spawn remains shell recipes under `artifacts/community-core-local/` and `scripts/scenarios/` (see DEMO-WORKFLOW). Full `deploy stack` execute is design G4.
