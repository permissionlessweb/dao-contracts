# DAO DAO Suite — API Reference

> Auto-generated from contract schemas and `DaoDaoSuite` registry.
> Regenerate: `cargo test -p dao-testing generate_suite_api_docs -- --ignored`

## Table of Contents

- [Core](#core)
- [Proposal](#proposal)
- [Pre-Propose](#pre-propose)
- [Voting](#voting)
- [Staking](#staking)
- [Distribution](#distribution)
- [External](#external)
- [Gauges](#gauges)

## Contract Summary

| Key | Name | Category | Description |
|-----|------|----------|-------------|
| `dao_core` | DAO Core | Core | Core DAO contract — modules, admin, pause, sub-DAOs |
| `prop_single` | Proposal Single-Choice | Proposal | Single-choice (yes/no/abstain) proposal module |
| `prop_multiple` | Proposal Multiple-Choice | Proposal | Multiple-choice proposal module with ranked options |
| `prop_condorcet` | Proposal Condorcet | Proposal | Condorcet-method ranked-choice proposal module |
| `prop_sudo` | Proposal Sudo | Proposal | Test-only sudo proposal module |
| `pre_prop_single` | Pre-Propose Single | Pre-Propose | Gatekeeper for single-choice proposals — deposits, whitelists |
| `pre_prop_multiple` | Pre-Propose Multiple | Pre-Propose | Gatekeeper for multiple-choice proposals |
| `pre_prop_approval_single` | Pre-Propose Approval Single | Pre-Propose | Approval-gated pre-propose — proposals need approver sign-off |
| `pre_prop_approver` | Pre-Propose Approver | Pre-Propose | Approver contract for approval-gated pre-propose flow |
| `voting_cw4` | Voting CW4 | Voting | Voting power from CW4 group membership weights |
| `voting_cw20_staked` | Voting CW20 Staked | Voting | Voting power from staked CW20 tokens |
| `voting_cw721_roles` | Voting CW721 Roles | Voting | Voting power from CW721 NFTs with role-based weights |
| `voting_cw721_staked` | Voting CW721 Staked | Voting | Voting power from staked CW721 NFTs (1 NFT = 1 vote) |
| `voting_token_staked` | Voting Token Staked | Voting | Voting power from staked native/tokenfactory tokens |
| `cw20_stake` | CW20 Stake | Staking | CW20 token staking with configurable unbonding |
| `external_rewards` | CW20 External Rewards | Staking | External reward distribution for CW20 stakers |
| `rewards_distributor` | CW20 Reward Distributor | Staking | Scheduled reward distribution for CW20 stakers |
| `fund_distributor` | Fund Distributor | Distribution | Pro-rata native/CW20 fund distribution to voters |
| `reward_distributor` | Rewards Distributor | Distribution | Continuous reward streaming to stakers/voters |
| `admin_factory` | Admin Factory | External | Factory for self-admin contract instantiation |
| `btsg_ft_factory` | BitSong FanToken Factory | External | BitSong fantoken creation and management |
| `payroll_factory` | Payroll Factory | External | Factory for vesting/payroll payment streams |
| `cw_tokenswap` | Token Swap | External | Escrow-based two-party token swap |
| `cw_tokenfactory_issuer` | TokenFactory Issuer | External | Tokenfactory denom management — mint, burn, freeze |
| `cw_vesting` | CW Vesting | External | Token vesting with configurable curves and clawback |
| `cw721_roles` | CW721 Roles | External | CW721 NFT collection with weighted roles for governance |
| `calendar` | DAO Calendar | External | On-chain event calendar with groups, gauges, scheduling |
| `gauge_orchestrator` | Gauge Orchestrator | Gauges | Orchestrates gauge voting rounds and option set management |
| `gauge_adapter` | Gauge Adapter | Gauges | Adapter connecting a gauge to an external reward or allocation target |

---

## Core

### DAO Core (`dao_core`)

> Core DAO contract — modules, admin, pause, sub-DAOs

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `admin` | no | Optional Admin with the ability to execute DAO messages directly. Useful for building SubDAOs controlled by a parent ... |
| `automatically_add_cw20s` | yes | If true the contract will automatically add received cw20 tokens to its treasury. |
| `automatically_add_cw721s` | yes | If true the contract will automatically add received cw721 tokens to its treasury. |
| `dao_uri` | no | Implements the DAO Star standard: <https://daostar.one/EIP> |
| `description` | yes | A description of the core contract. |
| `image_url` | no | An image URL to describe the core module contract. |
| `initial_actions` | no | Actions for the DAO to execute immediately. |
| `initial_items` | no | The items to instantiate this DAO with. Items are arbitrary key-value pairs whose contents are controlled by governan... |
| `name` | yes | The name of the core contract. |
| `proposal_modules_instantiate_info` | yes | Instantiate information for the core contract's proposal modules. NOTE: the pre-propose-base package depends on it be... |
| `voting_module_instantiate_info` | yes | Instantiate information for the core contract's voting power module. |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `ExecuteAdminMsgs` | Callable by the Admin, if one is configured. Executes messages in order. |
| `ExecuteProposalHook` | Callable by proposal modules. The DAO will execute the messages in the hook in order. |
| `Pause` | Pauses the DAO for a set duration. When paused the DAO is unable to execute proposals |
| `Unpause` | Unpauses the DAO |
| `Receive` | Executed when the contract receives a cw20 token. Depending on the contract's configuration the contract will automatically add the token... |
| `ReceiveNft` | Executed when the contract receives a cw721 token. Depending on the contract's configuration the contract will automatically add the toke... |
| `RemoveItem` | Removes an item from the governance contract's item map. |
| `SetItem` | Adds an item to the governance contract's item map. If the item already exists the existing value is overridden. If the item does not exi... |
| `NominateAdmin` | Callable by the admin of the contract. If ADMIN is None the admin is set as the contract itself so that it may be updated later by vote. ... |
| `AcceptAdminNomination` | Callable by a nominated admin. Admins are nominated via the `NominateAdmin` message. Accepting a nomination will make the nominated addre... |
| `WithdrawAdminNomination` | Callable by the current admin. Withdraws the current admin nomination. |
| `UpdateConfig` | Callable by the core contract. Replaces the current governance contract config with the provided config. |
| `UpdateCw20List` | Updates the list of cw20 tokens this contract has registered. |
| `UpdateCw721List` | Updates the list of cw721 tokens this contract has registered. |
| `UpdateProposalModules` | Updates the governance contract's governance modules. Module instantiate info in `to_add` is used to create new modules and install them. |
| `UpdateVotingModule` | Callable by the core contract. Replaces the current voting module with a new one instantiated by the governance contract. |
| `UpdateSubDaos` | Update the core module to add/remove SubDAOs and their charters |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Admin` | Get's the DAO's admin. Returns `Addr`. |
| `AdminNomination` | Get's the currently nominated admin (if any). |
| `Config` | Gets the contract's config. |
| `Cw20Balances` | Gets the token balance for each cw20 registered with the contract. |
| `Cw20TokenList` | Lists the addresses of the cw20 tokens in this contract's treasury. |
| `Cw721TokenList` | Lists the addresses of the cw721 tokens in this contract's treasury. |
| `DumpState` | Dumps all of the core contract's state in a single query. Useful for frontends as performance for queries is more limited by network time... |
| `GetItem` | Gets the address associated with an item key. |
| `ListItems` | Lists all of the items associted with the contract. For example, given the items `{ "group": "foo", "subdao": "bar"}` this query would re... |
| `Info` | Returns contract version info |
| `ProposalModules` | Gets all proposal modules associated with the contract. |
| `ActiveProposalModules` | Gets the active proposal modules associated with the contract. |
| `ProposalModuleCount` | Gets the number of active and total proposal modules registered with this module. |
| `PauseInfo` | Returns information about if the contract is currently paused. |
| `VotingModule` | Gets the contract's voting module. |
| `ListSubDaos` | Returns all SubDAOs with their charters in a vec. start_after is bound exclusive and asks for a string address. |
| `DaoURI` | Implements the DAO Star standard: <https://daostar.one/EIP> |
| `VotingPowerAtHeight` | Returns the voting power for an address at a given height. |
| `TotalPowerAtHeight` | Returns the total voting power at a given block height. |
| `InitialActions` | Returns the actions executed by the DAO on instantiation, if any. |

#### MigrateMsg

| Variant | Description |
|---------|-------------|
| `FromV1` |  |
| `FromCompatible` |  |

---

## Proposal

### Proposal Single-Choice (`prop_single`)

> Single-choice (yes/no/abstain) proposal module

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `allow_revoting` | yes | Allows changing votes before the proposal expires. If this is enabled proposals will not be able to complete early as... |
| `close_proposal_on_execution_failure` | yes | If set to true proposals will be closed if their execution fails. Otherwise, proposals will remain open after executi... |
| `delegation_module` | no | The address of the delegation module to use for this proposal module (if any). |
| `max_voting_period` | yes | The default maximum amount of time a proposal may be voted on before expiring. |
| `min_voting_period` | no | The minimum amount of time a proposal must be open before passing. A proposal may fail before this amount of time has... |
| `only_members_execute` | yes | If set to true only members may execute passed proposals. Otherwise, any address may execute a passed proposal. |
| `pre_propose_info` | yes | Information about what addresses may create proposals. |
| `threshold` | yes | The threshold a proposal must reach to complete. |
| `veto` | no | Optional veto configuration for proposal execution. If set, proposals can only be executed after the timelock delay e... |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Propose` | Creates a proposal in the module. |
| `Vote` | Votes on a proposal. Voting power is determined by the DAO's voting power module. |
| `UpdateRationale` | Updates the sender's rationale for their vote on the specified proposal. Errors if no vote vote has been cast. |
| `Execute` | Causes the messages associated with a passed proposal to be executed by the DAO. |
| `Veto` | Callable only if veto is configured |
| `Close` | Closes a proposal that has failed (either not passed or timed out). If applicable this will cause the proposal deposit associated wth sai... |
| `UpdateConfig` | Updates the governance module's config. |
| `UpdatePreProposeInfo` | Update's the proposal creation policy used for this module. Only the DAO may call this method. |
| `UpdateDelegationModule` | Update's the address of the delegation module associated with this proposal module. Only the DAO may call this method. |
| `AddProposalHook` | Adds an address as a consumer of proposal hooks. Consumers of proposal hooks have hook messages executed on them whenever the status of a... |
| `RemoveProposalHook` | Removes a consumer of proposal hooks. |
| `AddVoteHook` | Adds an address as a consumer of vote hooks. Consumers of vote hooks have hook messages executed on them whenever the a vote is cast. If ... |
| `RemoveVoteHook` | Removed a consumer of vote hooks. |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Config` | Gets the proposal module's config. |
| `Proposal` | Gets information about a proposal. |
| `ListProposals` | Lists all the proposals that have been cast in this module. |
| `ReverseProposals` | Lists all of the proposals that have been cast in this module in decending order of proposal ID. |
| `GetVote` | Returns a voters position on a propsal. |
| `ListVotes` | Lists all of the votes that have been cast on a proposal. |
| `ProposalCount` | Returns the number of proposals that have been created in this module. |
| `ProposalCreationPolicy` | Gets the current proposal creation policy for this module. |
| `DelegationModule` | Gets the address of the delegation module associated with this proposal module (if any). |
| `ProposalHooks` | Lists all of the consumers of proposal hooks for this module. |
| `VoteHooks` | Lists all of the consumers of vote hooks for this module. |
| `Dao` | Returns the address of the DAO this module belongs to |
| `Info` | Returns contract version info |
| `NextProposalId` | Returns the proposal ID that will be assigned to the next proposal created. |

#### MigrateMsg

| Variant | Description |
|---------|-------------|
| `FromV1` |  |
| `FromCompatible` |  |

### Proposal Multiple-Choice (`prop_multiple`)

> Multiple-choice proposal module with ranked options

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `allow_revoting` | yes | Allows changing votes before the proposal expires. If this is enabled proposals will not be able to complete early as... |
| `close_proposal_on_execution_failure` | yes | If set to true proposals will be closed if their execution fails. Otherwise, proposals will remain open after executi... |
| `delegation_module` | no | The address of the delegation module to use for this proposal module (if any). |
| `max_voting_period` | yes | The amount of time a proposal can be voted on before expiring |
| `min_voting_period` | no | The minimum amount of time a proposal must be open before passing. A proposal may fail before this amount of time has... |
| `only_members_execute` | yes | If set to true only members may execute passed proposals. Otherwise, any address may execute a passed proposal. |
| `pre_propose_info` | yes | Information about what addresses may create proposals. |
| `veto` | no | Optional veto configuration for proposal execution. If set, proposals can only be executed after the timelock delay e... |
| `voting_strategy` | yes | Voting params configuration |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Propose` | Creates a proposal in the governance module. |
| `Vote` | Votes on a proposal. Voting power is determined by the DAO's voting power module. |
| `Execute` | Causes the messages associated with a passed proposal to be executed by the DAO. |
| `Veto` | Callable only if veto is configured |
| `Close` | Closes a proposal that has failed (either not passed or timed out). If applicable this will cause the proposal deposit associated wth sai... |
| `UpdateConfig` | Updates the governance module's config. |
| `UpdateRationale` | Updates the sender's rationale for their vote on the specified proposal. Errors if no vote vote has been cast. |
| `UpdatePreProposeInfo` | Update's the proposal creation policy used for this module. Only the DAO may call this method. |
| `UpdateDelegationModule` | Update's the address of the delegation module associated with this proposal module. Only the DAO may call this method. |
| `AddProposalHook` |  |
| `RemoveProposalHook` |  |
| `AddVoteHook` |  |
| `RemoveVoteHook` |  |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Config` | Gets the governance module's config. |
| `Proposal` | Gets information about a proposal. |
| `ListProposals` | Lists all the proposals that have been cast in this module. |
| `ReverseProposals` | Lists all of the proposals that have been cast in this module in decending order of proposal ID. |
| `GetVote` | Returns a voters position on a proposal. |
| `ListVotes` | Lists all of the votes that have been cast on a proposal. |
| `ProposalCount` | Returns the number of proposals that have been created in this module. |
| `ProposalCreationPolicy` | Gets the current proposal creation policy for this module. |
| `DelegationModule` | Gets the address of the delegation module associated with this proposal module (if any). |
| `ProposalHooks` | Lists all of the consumers of proposal hooks for this module. |
| `VoteHooks` | Lists all of the consumers of vote hooks for this module. |
| `Dao` | Returns the address of the DAO this module belongs to |
| `Info` | Returns contract version info |
| `NextProposalId` | Returns the proposal ID that will be assigned to the next proposal created. |

#### MigrateMsg

| Variant | Description |
|---------|-------------|
| `FromV1` |  |
| `FromCompatible` |  |

### Proposal Condorcet (`prop_condorcet`)

> Condorcet-method ranked-choice proposal module

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `close_proposals_on_execution_failure` | yes |  |
| `min_voting_period` | no |  |
| `quorum` | yes |  |
| `voting_period` | yes |  |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Propose` |  |
| `Vote` |  |
| `Execute` |  |
| `Close` |  |
| `SetConfig` |  |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Proposal` |  |
| `Config` |  |
| `Dao` | Returns the address of the DAO this module belongs to |
| `Info` | Returns contract version info |
| `NextProposalId` | Returns the proposal ID that will be assigned to the next proposal created. |

### Proposal Sudo (`prop_sudo`)

> Test-only sudo proposal module

*No schema available.*

---

## Pre-Propose

### Pre-Propose Single (`pre_prop_single`)

> Gatekeeper for single-choice proposals — deposits, whitelists

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `deposit_info` | no | Information about the deposit requirements for this module. None if no deposit. |
| `extension` | yes | Extension for instantiation. The default implementation will do nothing with this data. |
| `submission_policy` | yes | The policy dictating who is allowed to submit proposals. |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Propose` | Creates a new proposal in the pre-propose module. MSG will be serialized and used as the proposal creation message. |
| `UpdateConfig` | Updates the configuration of this module. This will completely override the existing configuration. This new configuration will only appl... |
| `UpdateSubmissionPolicy` | Perform more granular submission policy updates to allow for atomic operations that don't override others. |
| `Withdraw` | Withdraws funds inside of this contract to the message sender. The contracts entire balance for the specifed DENOM is withdrawn to the me... |
| `Extension` | Extension message. Contracts that extend this one should put their custom execute logic here. The default implementation will do nothing ... |
| `AddProposalSubmittedHook` | Adds a proposal submitted hook. Fires when a new proposal is submitted to the pre-propose contract. Only the DAO may call this method. |
| `RemoveProposalSubmittedHook` | Removes a proposal submitted hook. Only the DAO may call this method. |
| `ProposalCompletedHook` | Handles proposal hook fired by the associated proposal module when a proposal is completed (ie executed or rejected). By default, the bas... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `ProposalModule` | Gets the proposal module that this pre propose module is associated with. Returns `Addr`. |
| `Dao` | Gets the DAO (dao-dao-core) module this contract is associated with. Returns `Addr`. |
| `Info` | Returns contract version info. |
| `Config` | Gets the module's configuration. |
| `DepositInfo` | Gets the deposit info for the proposal identified by PROPOSAL_ID. |
| `CanPropose` | Returns whether or not the address can submit proposals. |
| `ProposalSubmittedHooks` | Returns list of proposal submitted hooks. |
| `QueryExtension` | Extension for queries. The default implementation will do nothing if queried for will return `Binary::default()`. |

#### MigrateMsg

| Variant | Description |
|---------|-------------|
| `FromUnderV250` |  |
| `Extension` |  |

### Pre-Propose Multiple (`pre_prop_multiple`)

> Gatekeeper for multiple-choice proposals

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `deposit_info` | no | Information about the deposit requirements for this module. None if no deposit. |
| `extension` | yes | Extension for instantiation. The default implementation will do nothing with this data. |
| `submission_policy` | yes | The policy dictating who is allowed to submit proposals. |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Propose` | Creates a new proposal in the pre-propose module. MSG will be serialized and used as the proposal creation message. |
| `UpdateConfig` | Updates the configuration of this module. This will completely override the existing configuration. This new configuration will only appl... |
| `UpdateSubmissionPolicy` | Perform more granular submission policy updates to allow for atomic operations that don't override others. |
| `Withdraw` | Withdraws funds inside of this contract to the message sender. The contracts entire balance for the specifed DENOM is withdrawn to the me... |
| `Extension` | Extension message. Contracts that extend this one should put their custom execute logic here. The default implementation will do nothing ... |
| `AddProposalSubmittedHook` | Adds a proposal submitted hook. Fires when a new proposal is submitted to the pre-propose contract. Only the DAO may call this method. |
| `RemoveProposalSubmittedHook` | Removes a proposal submitted hook. Only the DAO may call this method. |
| `ProposalCompletedHook` | Handles proposal hook fired by the associated proposal module when a proposal is completed (ie executed or rejected). By default, the bas... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `ProposalModule` | Gets the proposal module that this pre propose module is associated with. Returns `Addr`. |
| `Dao` | Gets the DAO (dao-dao-core) module this contract is associated with. Returns `Addr`. |
| `Info` | Returns contract version info. |
| `Config` | Gets the module's configuration. |
| `DepositInfo` | Gets the deposit info for the proposal identified by PROPOSAL_ID. |
| `CanPropose` | Returns whether or not the address can submit proposals. |
| `ProposalSubmittedHooks` | Returns list of proposal submitted hooks. |
| `QueryExtension` | Extension for queries. The default implementation will do nothing if queried for will return `Binary::default()`. |

#### MigrateMsg

| Variant | Description |
|---------|-------------|
| `FromUnderV250` |  |
| `Extension` |  |

### Pre-Propose Approval Single (`pre_prop_approval_single`)

> Approval-gated pre-propose — proposals need approver sign-off

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `deposit_info` | no | Information about the deposit requirements for this module. None if no deposit. |
| `extension` | yes | Extension for instantiation. The default implementation will do nothing with this data. |
| `submission_policy` | yes | The policy dictating who is allowed to submit proposals. |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Propose` | Creates a new proposal in the pre-propose module. MSG will be serialized and used as the proposal creation message. |
| `UpdateConfig` | Updates the configuration of this module. This will completely override the existing configuration. This new configuration will only appl... |
| `UpdateSubmissionPolicy` | Perform more granular submission policy updates to allow for atomic operations that don't override others. |
| `Withdraw` | Withdraws funds inside of this contract to the message sender. The contracts entire balance for the specifed DENOM is withdrawn to the me... |
| `Extension` | Extension message. Contracts that extend this one should put their custom execute logic here. The default implementation will do nothing ... |
| `AddProposalSubmittedHook` | Adds a proposal submitted hook. Fires when a new proposal is submitted to the pre-propose contract. Only the DAO may call this method. |
| `RemoveProposalSubmittedHook` | Removes a proposal submitted hook. Only the DAO may call this method. |
| `ProposalCompletedHook` | Handles proposal hook fired by the associated proposal module when a proposal is completed (ie executed or rejected). By default, the bas... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `ProposalModule` | Gets the proposal module that this pre propose module is associated with. Returns `Addr`. |
| `Dao` | Gets the DAO (dao-dao-core) module this contract is associated with. Returns `Addr`. |
| `Info` | Returns contract version info. |
| `Config` | Gets the module's configuration. |
| `DepositInfo` | Gets the deposit info for the proposal identified by PROPOSAL_ID. |
| `CanPropose` | Returns whether or not the address can submit proposals. |
| `ProposalSubmittedHooks` | Returns list of proposal submitted hooks. |
| `QueryExtension` | Extension for queries. The default implementation will do nothing if queried for will return `Binary::default()`. |

#### MigrateMsg

| Variant | Description |
|---------|-------------|
| `FromUnderV250` |  |
| `Extension` |  |

### Pre-Propose Approver (`pre_prop_approver`)

> Approver contract for approval-gated pre-propose flow

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `pre_propose_approval_contract` | yes |  |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Propose` | Creates a new proposal in the pre-propose module. MSG will be serialized and used as the proposal creation message. |
| `UpdateConfig` | Updates the configuration of this module. This will completely override the existing configuration. This new configuration will only appl... |
| `UpdateSubmissionPolicy` | Perform more granular submission policy updates to allow for atomic operations that don't override others. |
| `Withdraw` | Withdraws funds inside of this contract to the message sender. The contracts entire balance for the specifed DENOM is withdrawn to the me... |
| `Extension` | Extension message. Contracts that extend this one should put their custom execute logic here. The default implementation will do nothing ... |
| `AddProposalSubmittedHook` | Adds a proposal submitted hook. Fires when a new proposal is submitted to the pre-propose contract. Only the DAO may call this method. |
| `RemoveProposalSubmittedHook` | Removes a proposal submitted hook. Only the DAO may call this method. |
| `ProposalCompletedHook` | Handles proposal hook fired by the associated proposal module when a proposal is completed (ie executed or rejected). By default, the bas... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `ProposalModule` | Gets the proposal module that this pre propose module is associated with. Returns `Addr`. |
| `Dao` | Gets the DAO (dao-dao-core) module this contract is associated with. Returns `Addr`. |
| `Info` | Returns contract version info. |
| `Config` | Gets the module's configuration. |
| `DepositInfo` | Gets the deposit info for the proposal identified by PROPOSAL_ID. |
| `CanPropose` | Returns whether or not the address can submit proposals. |
| `ProposalSubmittedHooks` | Returns list of proposal submitted hooks. |
| `QueryExtension` | Extension for queries. The default implementation will do nothing if queried for will return `Binary::default()`. |

#### MigrateMsg

| Variant | Description |
|---------|-------------|
| `FromUnderV250` |  |
| `Extension` |  |

---

## Voting

### Voting CW4 (`voting_cw4`)

> Voting power from CW4 group membership weights

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `group_contract` | yes |  |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `GroupContract` |  |
| `VotingPowerAtHeight` | Returns the voting power for an address at a given height. |
| `TotalPowerAtHeight` | Returns the total voting power at a given block heigh. |
| `Dao` | Returns the address of the DAO this module belongs to. |
| `Info` | Returns contract version info. |

### Voting CW20 Staked (`voting_cw20_staked`)

> Voting power from staked CW20 tokens

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `active_threshold` | no | The number or percentage of tokens that must be staked for the DAO to be active |
| `token_info` | yes |  |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `UpdateActiveThreshold` | Sets the active threshold to a new value. Only the instantiator this contract (a DAO most likely) may call this method. |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `StakingContract` | Gets the address of the cw20-stake contract this voting module is wrapping. |
| `ActiveThreshold` |  |
| `VotingPowerAtHeight` | Returns the voting power for an address at a given height. |
| `TotalPowerAtHeight` | Returns the total voting power at a given block heigh. |
| `Dao` | Returns the address of the DAO this module belongs to. |
| `Info` | Returns contract version info. |
| `TokenContract` |  |
| `IsActive` |  |

### Voting CW721 Roles (`voting_cw721_roles`)

> Voting power from CW721 NFTs with role-based weights

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `nft_contract` | yes | Info about the associated NFT contract |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Config` |  |
| `VotingPowerAtHeight` | Returns the voting power for an address at a given height. |
| `TotalPowerAtHeight` | Returns the total voting power at a given block heigh. |
| `Dao` | Returns the address of the DAO this module belongs to. |
| `Info` | Returns contract version info. |

### Voting CW721 Staked (`voting_cw721_staked`)

> Voting power from staked CW721 NFTs (1 NFT = 1 vote)

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `active_threshold` | no | The number or percentage of tokens that must be staked for the DAO to be active |
| `nft_contract` | yes | Address of the cw721 NFT contract that may be staked. |
| `unstaking_duration` | no | Amount of time between unstaking and tokens being avaliable. To unstake with no delay, leave as `None`. |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `ReceiveNft` | Used to stake NFTs. To stake a NFT send a cw721 send message to this contract with the NFT you would like to stake. The `msg` field is ig... |
| `Unstake` | Unstakes the specified token_ids on behalf of the sender. token_ids must have unique values and have non-zero length. |
| `ClaimNfts` | Claim NFTs that have been unstaked for the specified duration. |
| `UpdateConfig` | Updates the contract configuration, namely unstaking duration. Only callable by the DAO that initialized this voting contract. |
| `AddHook` | Adds a hook which is called on staking / unstaking events. Only callable by the DAO that initialized this voting contract. |
| `RemoveHook` | Removes a hook which is called on staking / unstaking events. Only callable by the DAO that initialized this voting contract. |
| `UpdateActiveThreshold` | Sets the active threshold to a new value. Only callable by the DAO that initialized this voting contract. |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Config` |  |
| `NftClaims` |  |
| `Hooks` |  |
| `StakedNfts` |  |
| `ActiveThreshold` |  |
| `IsActive` |  |
| `VotingPowerAtHeight` | Returns the voting power for an address at a given height. |
| `TotalPowerAtHeight` | Returns the total voting power at a given block heigh. |
| `Dao` | Returns the address of the DAO this module belongs to. |
| `Info` | Returns contract version info. |

### Voting Token Staked (`voting_token_staked`)

> Voting power from staked native/tokenfactory tokens

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `active_threshold` | no | The number or percentage of tokens that must be staked for the DAO to be active |
| `token_info` | yes | New or existing native token to use for voting power. |
| `unstaking_duration` | no | How long until the tokens become liquid again |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Stake` | Stakes tokens with the contract to get voting power in the DAO |
| `Unstake` | Unstakes tokens so that they begin unbonding |
| `UpdateConfig` | Updates the contract configuration |
| `Claim` | Claims unstaked tokens that have completed the unbonding period |
| `UpdateActiveThreshold` | Sets the active threshold to a new value. Only the instantiator of this contract (a DAO most likely) may call this method. |
| `AddHook` | Adds a hook that fires on staking / unstaking |
| `RemoveHook` | Removes a hook that fires on staking / unstaking |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `GetConfig` |  |
| `Claims` |  |
| `ListStakers` |  |
| `ActiveThreshold` |  |
| `GetHooks` |  |
| `TokenContract` |  |
| `Denom` |  |
| `IsActive` |  |
| `VotingPowerAtHeight` | Returns the voting power for an address at a given height. |
| `TotalPowerAtHeight` | Returns the total voting power at a given block heigh. |
| `Dao` | Returns the address of the DAO this module belongs to. |
| `Info` | Returns contract version info. |

---

## Staking

### CW20 Stake (`cw20_stake`)

> CW20 token staking with configurable unbonding

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `owner` | no |  |
| `token_address` | yes |  |
| `unstaking_duration` | no |  |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Receive` |  |
| `Unstake` |  |
| `Claim` |  |
| `UpdateConfig` |  |
| `AddHook` |  |
| `RemoveHook` |  |
| `UpdateOwnership` | Update the contract's ownership. The `action` to be provided can be either to propose transferring ownership to an account, accept a pend... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `StakedBalanceAtHeight` |  |
| `TotalStakedAtHeight` |  |
| `StakedValue` |  |
| `TotalValue` |  |
| `GetConfig` |  |
| `Claims` |  |
| `GetHooks` |  |
| `ListStakers` |  |
| `Ownership` |  |

#### MigrateMsg

| Variant | Description |
|---------|-------------|
| `FromV1` | Migrates the contract from version one to version two. This will remove the contract's current manager, and require a nomination -> acceptance flow for future ownership transfers. |

### CW20 External Rewards (`external_rewards`)

> External reward distribution for CW20 stakers

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `owner` | no |  |
| `reward_duration` | yes |  |
| `reward_token` | yes |  |
| `staking_contract` | yes |  |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `StakeChangeHook` |  |
| `Claim` |  |
| `Receive` |  |
| `Fund` |  |
| `UpdateRewardDuration` |  |
| `UpdateOwnership` | Update the contract's ownership. The `action` to be provided can be either to propose transferring ownership to an account, accept a pend... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Info` |  |
| `GetPendingRewards` |  |
| `Ownership` |  |

#### MigrateMsg

| Variant | Description |
|---------|-------------|
| `FromV1` | Migrates from version 0.2.6 to 2.0.0. The significant changes being the addition of a two-step ownership transfer using `cw_ownable` and the removal of the manager. Migrating will automatically remove the current manager. |

### CW20 Reward Distributor (`rewards_distributor`)

> Scheduled reward distribution for CW20 stakers

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `owner` | yes |  |
| `reward_rate` | yes |  |
| `reward_token` | yes |  |
| `staking_addr` | yes |  |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `UpdateConfig` |  |
| `Distribute` |  |
| `Withdraw` |  |
| `UpdateOwnership` | Update the contract's ownership. The `action` to be provided can be either to propose transferring ownership to an account, accept a pend... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Info` |  |
| `Ownership` |  |

#### MigrateMsg

| Variant | Description |
|---------|-------------|
| `FromV1` | Updates the contract from v1 -> v2. Version two implements a two step ownership transfer. |

---

## Distribution

### Fund Distributor (`fund_distributor`)

> Pro-rata native/CW20 fund distribution to voters

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `active_threshold` | no | The number or percentage of tokens that must be staked for the DAO to be active |
| `token_info` | yes |  |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `UpdateActiveThreshold` | Sets the active threshold to a new value. Only the instantiator this contract (a DAO most likely) may call this method. |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `StakingContract` | Gets the address of the cw20-stake contract this voting module is wrapping. |
| `ActiveThreshold` |  |
| `VotingPowerAtHeight` | Returns the voting power for an address at a given height. |
| `TotalPowerAtHeight` | Returns the total voting power at a given block heigh. |
| `Dao` | Returns the address of the DAO this module belongs to. |
| `Info` | Returns contract version info. |
| `TokenContract` |  |
| `IsActive` |  |

### Rewards Distributor (`reward_distributor`)

> Continuous reward streaming to stakers/voters

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `owner` | no | The owner of the contract. Is able to fund the contract and update the reward duration. If not provided, the instanti... |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `MemberChangedHook` | Called when a member is added or removed to a cw4-groups or cw721-roles contract. |
| `NftStakeChangeHook` | Called when NFTs are staked or unstaked. |
| `StakeChangeHook` | Called when tokens are staked or unstaked. |
| `Create` | registers a new distribution |
| `Update` | updates the config for a distribution |
| `Receive` | Used to fund this contract with cw20 tokens. |
| `Fund` | Used to fund this contract with native tokens. |
| `FundLatest` | Used to fund the latest distribution with native tokens. |
| `Claim` | Claims rewards for the sender. |
| `Withdraw` | withdraws the undistributed rewards for a distribution. members can claim whatever they earned until this point. this is effectively an i... |
| `UnsafeForceWithdraw` | forcibly withdraw funds from the contract. this is unsafe and should only be used to recover funds that are stuck in the contract. |
| `UpdateOwnership` | Update the contract's ownership. The `action` to be provided can be either to propose transferring ownership to an account, accept a pend... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Info` | Returns contract version info |
| `Ownership` | Returns information about the ownership of this contract. |
| `PendingRewards` | Returns the pending rewards for the given address. |
| `UndistributedRewards` | Returns the undistributed rewards for a distribution. |
| `Distribution` | Returns the state of the given distribution. |
| `Distributions` | Returns the state of all the distributions. |

---

## External

### Admin Factory (`admin_factory`)

> Factory for self-admin contract instantiation

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `admin` | no | The account allowed to execute this contract. If no admin, anyone can execute it. |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `InstantiateContractWithSelfAdmin` | Instantiates the target contract with the provided instantiate message, code ID, and label and updates the contract's admin to be itself. |
| `Instantiate2ContractWithSelfAdmin` | Instantiates the target contract with the provided instantiate message, code ID, label, and salt, via instantiate2 to give a predictable ... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Admin` |  |

### BitSong FanToken Factory (`btsg_ft_factory`)

> BitSong fantoken creation and management

**Version**: `2.8.0-alpha.2`

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Issue` | Issues a new fantoken. |

### Payroll Factory (`payroll_factory`)

> Factory for vesting/payroll payment streams

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `owner` | no |  |
| `vesting_code_id` | yes |  |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Receive` | Instantiates a new vesting contract that is funded by a cw20 token. |
| `InstantiateNativePayrollContract` | Instantiates a new vesting contract that is funded by a native token. |
| `UpdateCodeId` | Callable only by the current owner. Updates the code ID used while instantiating vesting contracts. |
| `UpdateOwnership` | Update the contract's ownership. The `action` to be provided can be either to propose transferring ownership to an account, accept a pend... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `ListVestingContracts` | Returns list of all vesting payment contracts |
| `ListVestingContractsReverse` | Returns list of all vesting payment contracts in reverse |
| `ListVestingContractsByInstantiator` | Returns list of all vesting payment contracts by who instantiated them |
| `ListVestingContractsByInstantiatorReverse` | Returns list of all vesting payment contracts by who instantiated them in reverse |
| `ListVestingContractsByRecipient` | Returns list of all vesting payment contracts by recipient |
| `ListVestingContractsByRecipientReverse` | Returns list of all vesting payment contracts by recipient in reverse |
| `Ownership` | Returns info about the contract ownership, if set |
| `CodeId` | Returns the code ID currently being used to instantiate vesting contracts. |

### Token Swap (`cw_tokenswap`)

> Escrow-based two-party token swap

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `counterparty_one` | yes |  |
| `counterparty_two` | yes |  |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Receive` | Used to provide cw20 tokens to satisfy a funds promise. |
| `Fund` | Provides native tokens to satisfy a funds promise. |
| `Withdraw` | Withdraws provided funds. Only allowed if the other counterparty has yet to provide their promised funds. |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Status` |  |

### TokenFactory Issuer (`cw_tokenfactory_issuer`)

> Tokenfactory denom management — mint, burn, freeze

**Version**: `2.8.0-alpha.2`

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Allow` | Allow adds the target address to the allowlist to be able to send or recieve tokens even if the token is frozen. Token Factory's BeforeSe... |
| `Burn` | Burn token to address. Burn allowance is required and wiil be deducted after successful burn. |
| `Mint` | Mint token to address. Mint allowance is required and wiil be deducted after successful mint. |
| `Deny` | Deny adds the target address to the denylist, whis prevents them from sending/receiving the token attached to this contract tokenfactory'... |
| `Freeze` | Block every token transfers of the token attached to this contract. Token Factory's BeforeSendHook listener must be set to this contract ... |
| `ForceTransfer` | Force transfer token from one address to another. |
| `SetBeforeSendHook` | Attempt to SetBeforeSendHook on the token attached to this contract. This will fail if the chain does not support bank module hooks (many... |
| `SetBurnerAllowance` | Grant/revoke burn allowance. |
| `SetDenomMetadata` | Set denom metadata. see: https://docs.cosmos.network/main/modules/bank#denom-metadata. |
| `SetMinterAllowance` | Grant/revoke mint allowance. |
| `UpdateTokenFactoryAdmin` | Updates the admin of the Token Factory token. Normally this is the cw-tokenfactory-issuer contract itself. This is intended to be used on... |
| `UpdateOwnership` | Updates the owner of this contract who is allowed to call privileged methods. NOTE: this is separate from the Token Factory token admin, ... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `IsFrozen` | Returns if token transfer is disabled. Response: IsFrozenResponse |
| `Denom` | Returns the token denom that this contract is the admin for. Response: DenomResponse |
| `Ownership` |  |
| `BurnAllowance` | Returns the burn allowance of the specified address. Response: AllowanceResponse |
| `BurnAllowances` | Enumerates over all burn allownances. Response: AllowancesResponse |
| `MintAllowance` | Returns the mint allowance of the specified user. Response: AllowanceResponse |
| `MintAllowances` | Enumerates over all mint allownances. Response: AllowancesResponse |
| `IsDenied` | Returns wether the user is on denylist or not. Response: StatusResponse |
| `Denylist` | Enumerates over all addresses on the denylist. Response: DenylistResponse |
| `IsAllowed` | Returns wether the user is on the allowlist or not. Response: StatusResponse |
| `Allowlist` | Enumerates over all addresses on the allowlist. Response: AllowlistResponse |
| `BeforeSendHookInfo` | Returns information about the BeforeSendHook for the token. Note: many Token Factory chains do not yet support this feature.

The informa... |

### CW Vesting (`cw_vesting`)

> Token vesting with configurable curves and clawback

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `denom` | yes | The type and denom of token being vested. |
| `description` | no | A description for the payment to provide more context. |
| `owner` | no | The optional owner address of the contract. If an owner is specified, the owner may cancel the vesting contract at an... |
| `recipient` | yes | The receiver address of the vesting tokens. |
| `schedule` | yes | The vesting schedule, can be either `SaturatingLinear` vesting (which vests evenly over time), or `PiecewiseLinear` w... |
| `start_time` | no | The time to start vesting, or None to start vesting when the contract is instantiated. `start_time` may be in the pas... |
| `title` | yes | The a name or title for this payment. |
| `total` | yes | The total amount of tokens to be vested. |
| `unbonding_duration_seconds` | yes | The unbonding duration for the chain this contract is deployed on. Smart contracts do not have access to this data as... |
| `vesting_duration_seconds` | yes | The length of the vesting schedule in seconds. Must be non-zero, though one second vesting durations are allowed. Thi... |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `Receive` | Fund the contract with a cw20 token. The `msg` field must have the shape `{"fund":{}}`, and the amount sent must be the same as the amoun... |
| `Distribute` | Distribute vested tokens to the vest receiver. Anyone may call this method. |
| `Cancel` | Cancels the vesting payment. The current amount vested becomes the total amount that will ever vest, and all pending and future staking r... |
| `Delegate` | This is translated to a [MsgDelegate](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto#L81-L90). `... |
| `Redelegate` | This is translated to a [MsgBeginRedelegate](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto#L96)... |
| `Undelegate` | This is translated to a [MsgUndelegate](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto#L112-L121... |
| `SetWithdrawAddress` | This is translated to a [MsgSetWithdrawAddress](https://github.com/cosmos/cosmos-sdk/blob/v0.42.4/proto/cosmos/distribution/v1beta1/tx.pr... |
| `WithdrawDelegatorReward` | This is translated to a [MsgWithdrawDelegatorReward](https://github.com/cosmos/cosmos-sdk/blob/v0.42.4/proto/cosmos/distribution/v1beta1/... |
| `WithdrawCanceledPayment` | If the owner cancels a payment and there are not enough liquid tokens to settle the owner may become entitled to some number of staked to... |
| `RegisterSlash` | Registers a slash event bonded or unbonding tokens with the contract. Only callable by the owner as the contract is unable to verify that... |
| `UpdateOwnership` | Update the contract's ownership. The `action` to be provided can be either to propose transferring ownership to an account, accept a pend... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `Ownership` | Get the current ownership. |
| `Info` | Returns information about the vesting contract and the status of the payment. |
| `Distributable` | Returns the number of tokens currently claimable by the vestee. This is the minimum of the number of unstaked tokens in the contract, and... |
| `Vested` | Gets the current value of `vested(t)`. If `t` is `None`, the current time is used. |
| `TotalToVest` | Gets the total amount that will ever vest, `max(vested(t))`.

Note that if the contract is canceled at time c, this value will change to ... |
| `VestDuration` | Gets the amount of time between the vest starting, and it completing. Returns `None` if the vest has been cancelled. |
| `Stake` | Queries information about the contract's understanding of it's bonded and unbonding token balances. See the `StakeTrackerQuery` in `packa... |

### CW721 Roles (`cw721_roles`)

> CW721 NFT collection with weighted roles for governance

**Version**: `2.8.0-alpha.2`

#### InstantiateMsg

| Field | Required | Description |
|-------|----------|-------------|
| `minter` | yes | The minter is the only one who can create new NFTs. This is designed for a base NFT that is controlled by an external... |
| `name` | yes | Name of the NFT contract |
| `symbol` | yes | Symbol of the NFT contract |

#### ExecuteMsg

| Variant | Description |
|---------|-------------|
| `TransferNft` | Transfer is a base message to move a token to another account without triggering actions |
| `SendNft` | Send is a base message to transfer a token to a contract and trigger an action on the receiving contract. |
| `Approve` | Allows operator to transfer / send the token from the owner's account. If expiration is set, then this allowance has a time/height limit |
| `Revoke` | Remove previously granted Approval |
| `ApproveAll` | Allows operator to transfer / send any token from the owner's account. If expiration is set, then this allowance has a time/height limit |
| `RevokeAll` | Remove previously granted ApproveAll permission |
| `Mint` | Mint a new NFT, can only be called by the contract minter |
| `Burn` | Burn an NFT the sender has access to |
| `Extension` | Extension msg |
| `UpdateOwnership` | Update the contract's ownership. The `action` to be provided can be either to propose transferring ownership to an account, accept a pend... |

#### QueryMsg

| Variant | Description |
|---------|-------------|
| `OwnerOf` | Return the owner of the given token, error if token does not exist |
| `Approval` | Return operator that can access all of the owner's tokens. |
| `Approvals` | Return approvals that a token has |
| `Operator` | Return approval of a given operator for all tokens of an owner, error if not set |
| `AllOperators` | List all operators that can access all of the owner's tokens |
| `NumTokens` | Total number of tokens issued |
| `ContractInfo` | With MetaData Extension. Returns top-level metadata about the contract |
| `NftInfo` | With MetaData Extension. Returns metadata about one particular token, based on *ERC721 Metadata JSON Schema* but directly from the contract |
| `AllNftInfo` | With MetaData Extension. Returns the result of both `NftInfo` and `OwnerOf` as one query as an optimization for clients |
| `Tokens` | With Enumerable extension. Returns all tokens owned by the given address, [] if unset. |
| `AllTokens` | With Enumerable extension. Requires pagination. Lists all token_ids controlled by the contract. |
| `Minter` | Return the minter |
| `Extension` | Extension query |
| `Ownership` | Query the contract's ownership information |

### DAO Calendar (`calendar`)

> On-chain event calendar with groups, gauges, scheduling

*Schema not found: `schema/dao-calendar.json`*

---

## Gauges

### Gauge Orchestrator (`gauge_orchestrator`)

> Orchestrates gauge voting rounds and option set management

*Schema not found: `contracts/gauges/gauge-orchestrator/schema/gauge-orchestrator.json`*

### Gauge Adapter (`gauge_adapter`)

> Adapter connecting a gauge to an external reward or allocation target

*Schema not found: `contracts/gauges/gauge-adapter/schema/gauge-adapter.json`*

---

*29 contracts across 8 categories.*
