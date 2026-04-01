use crate::msg::AssetUnchecked;
use crate::msg::CheckOptionResponse;
use crate::msg::{
    AdapterBankMsg, AdapterCw20Msgs, AdapterQueryMsg, AdapterWasmMsg, AllOptionsResponse,
    AllSubmissionsResponse, ExecuteMsg, PossibleMsg, ReceiveMsg, StargateWire, SubmissionMsg,
    SubmissionResponse,
};
use anyhow::Result as AnyResult;
use cosmwasm_std::{coins, to_json_binary, Addr, BankMsg, Binary, Coin, Uint128, Uint256};
use cw20::{BalanceResponse, Cw20QueryMsg};
use cw20::{Cw20Coin, MinterResponse};
use cw20_base::msg::ExecuteMsg as Cw20BaseExecuteMsg;
use cw20_base::msg::InstantiateMsg as Cw20BaseInstantiateMsg;
use cw_denom::UncheckedDenom;
use cosmwasm_std::testing::MockApi;
use cw_multi_test::{App, AppResponse, ContractWrapper, Executor};

pub const NATIVE: &str = "juno";
pub const CW20: &str = "wynd";
pub const OWNER: &str = "owner";
pub const TREASURY: &str = "treasury";

// Store the marketing gauge adapter contract and returns the code id.
fn store_gauge_adapter(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    ));
    app.store_code(contract)
}

// Store the cw20 contract and returns the code id.
fn store_cw20(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ));
    app.store_code(contract)
}

// ---------------------------------------------------------------------------
// Helpers used by anybuf tests (no cw-orch)
// ---------------------------------------------------------------------------

/// Wraps an App + gauge adapter address for use in anybuf tests.
pub struct GaugeAdapterApp {
    pub app: App,
    pub addr: Addr,
}

impl GaugeAdapterApp {
    pub fn available_messages(&self) -> AnyResult<Vec<PossibleMsg>> {
        self.app
            .wrap()
            .query_wasm_smart(self.addr.clone(), &AdapterQueryMsg::AvailableMessages {})
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub fn query_all_options(&self) -> AnyResult<AllOptionsResponse> {
        self.app
            .wrap()
            .query_wasm_smart(self.addr.clone(), &AdapterQueryMsg::AllOptions {})
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub fn query_check_option(&self, option: String) -> AnyResult<CheckOptionResponse> {
        self.app.wrap().query_wasm_smart(
            self.addr.clone(),
            &AdapterQueryMsg::CheckOption { option },
        ).map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub fn give_balance(&mut self, addr: &str, coins: Vec<Coin>) {
        self.app
            .init_modules(|router, _, storage| -> AnyResult<()> {
                router
                    .bank
                    .init_balance(storage, &MockApi::default().addr_make(addr), coins)
                    .map_err(|e| anyhow::anyhow!("{e}"))
            })
            .unwrap();
    }
}

/// Sets up a gauge adapter with cw_multi_test::App.
/// Owner (OWNER) starts with 1_000_000 juno. Treasury is TREASURY.
/// Always includes Bank::MsgSend and Wasm::Cw20::Send as default possible msgs.
pub fn setup_gauge_adapter(
    required_deposit: Option<AssetUnchecked>,
    possible_msgs: Option<Vec<PossibleMsg>>,
) -> GaugeAdapterApp {
    let owner = MockApi::default().addr_make(OWNER);
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner, coins(1_000_000, NATIVE))
            .unwrap();
    });

    let mut msgs = vec![
        PossibleMsg {
            stargate: StargateWire::Bank(AdapterBankMsg::MsgSend()),
            max_amount: Some(Uint256::from(1_000u128)),
        },
        PossibleMsg {
            stargate: StargateWire::Wasm(AdapterWasmMsg::Cw20(AdapterCw20Msgs::Send())),
            max_amount: Some(Uint256::from(1_000u128)),
        },
    ];
    if let Some(extra) = possible_msgs {
        msgs.extend(extra);
    }

    let code_id = store_gauge_adapter(&mut app);
    let addr = app
        .instantiate_contract(
            code_id,
            owner.clone(),
            &crate::msg::InstantiateMsg {
                owner: owner.to_string(),
                required_deposit,
                treasury: MockApi::default().addr_make(TREASURY).to_string(),
                reward: AssetUnchecked {
                    denom: UncheckedDenom::Native(NATIVE.into()),
                    amount: 1_000_000u128.into(),
                },
                possible_msgs: msgs,
            },
            &[],
            "gauge_adapter",
            None,
        )
        .unwrap();

    GaugeAdapterApp { app, addr }
}

/// Submits a native-token-backed submission to the gauge adapter.
pub fn native_submission_helper(
    gauge: &mut GaugeAdapterApp,
    sender: &str,
    recipient: &str,
    native_tokens: Option<Coin>,
    msg: SubmissionMsg,
) -> AnyResult<AppResponse> {
    let funds = native_tokens.map(|c| vec![c]).unwrap_or_default();
    gauge.app.execute_contract(
        MockApi::default().addr_make(sender),
        gauge.addr.clone(),
        &ExecuteMsg::CreateSubmission {
            name: "DAOers".to_string(),
            url: "https://daodao.zone".to_string(),
            address: MockApi::default().addr_make(recipient).to_string(),
            message: msg,
        },
        &funds,
    ).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Returns a valid default SubmissionMsg (Bank::MsgSend) for tests that don't care about message content.
pub fn default_submission_msg() -> SubmissionMsg {
    SubmissionMsg {
        stargate: StargateWire::Bank(AdapterBankMsg::MsgSend()),
        msg: to_json_binary(&BankMsg::Send {
            to_address: "recipient".to_string(),
            amount: vec![],
        })
        .unwrap(),
    }
}

// ---------------------------------------------------------------------------
// cw_multi_test suite (used by submission and options tests)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct SuiteBuilder {
    treasury: String,
    required_deposit: Option<AssetUnchecked>,
    cw20_deposit_amount: Option<u128>,
    reward: AssetUnchecked,
    funds: Vec<(Addr, Vec<Coin>)>,
    cw20_funds: Vec<Cw20Coin>,
}

impl SuiteBuilder {
    pub fn new() -> Self {
        Self {
            treasury: MockApi::default().addr_make(TREASURY).to_string(),
            required_deposit: None,
            cw20_deposit_amount: None,
            reward: AssetUnchecked {
                denom: UncheckedDenom::Native(NATIVE.into()),
                amount: Uint256::from(1_000_000u128),
            },
            funds: vec![],
            cw20_funds: vec![],
        }
    }

    pub fn with_treasury(mut self, treasury: &str) -> Self {
        self.treasury = MockApi::default().addr_make(treasury).to_string();
        self
    }

    // Allows to initialize the suite with native coins associated to an address.
    pub fn with_funds(mut self, addr: &str, funds: &[Coin]) -> Self {
        self.funds.push((MockApi::default().addr_make(addr), funds.into()));
        self
    }

    // Allows to initialize the suite with default cw20 tokens associated to an address.
    pub fn with_cw20_funds(mut self, addr: &str, amount: u128) -> Self {
        self.cw20_funds.push(Cw20Coin {
            address: MockApi::default().addr_make(addr).to_string(),
            amount: Uint256::from(amount),
        });
        self
    }

    // Allows to initialize the marketing gauge adapter with required native coins in the config.
    pub fn with_native_deposit(mut self, amount: u128) -> Self {
        self.required_deposit = Some(AssetUnchecked {
            denom: UncheckedDenom::Native(NATIVE.into()),
            amount: Uint256::from(amount),
        });
        self
    }

    // Allows to initialize the marketing gauge adapter with required cw20 tokens in the config.
    // The actual cw20 address is resolved at build() time from the deployed default_cw20.
    pub fn with_cw20_deposit(mut self, amount: u128) -> Self {
        self.cw20_deposit_amount = Some(amount);
        self
    }

    // Instantiate a marketing gauge adapter and returns its address.
    fn instantiate_marketing_gauge_adapter(
        &self,
        app: &mut App,
        gauge_id: u64,
        owner: &str,
    ) -> Addr {
        app.instantiate_contract(
            gauge_id,
            Addr::unchecked(owner), // already valid bech32 from addr_make
            &crate::msg::InstantiateMsg {
                owner: owner.to_owned(),
                required_deposit: self.required_deposit.clone(),
                treasury: self.treasury.clone(),
                reward: self.reward.clone(),
                possible_msgs: vec![PossibleMsg {
                    stargate: StargateWire::Bank(AdapterBankMsg::MsgSend()),
                    max_amount: None,
                }],
            },
            &[],
            "Marketing Gauge Adapter",
            None,
        )
        .unwrap()
    }

    // Instantiate a cw20 and returns its address.
    fn instantiate_default_cw20(&self, app: &mut App, cw20_code_id: u64, owner: &str) -> Addr {
        let res = app
            .instantiate_contract(
                cw20_code_id,
                Addr::unchecked(owner), // already valid bech32 from addr_make
                &Cw20BaseInstantiateMsg {
                    name: CW20.to_owned(),
                    symbol: CW20.to_owned(),
                    decimals: 6,
                    initial_balances: self.cw20_funds.clone(),
                    mint: Some(MinterResponse {
                        minter: owner.to_string(),
                        cap: None,
                    }),
                    marketing: None,
                },
                &[],
                CW20.to_owned(),
                None,
            )
            .unwrap();
        println!("{:#?}", res);
        res
    }

    #[track_caller]
    pub fn build(mut self) -> Suite {
        let mut app = App::default();
        let owner = MockApi::default().addr_make(OWNER);

        // Store required contracts.
        let cw20_code_id = store_cw20(&mut app);
        let gauge_adapter_code_id = store_gauge_adapter(&mut app);

        let owner_addr = MockApi::default().addr_make(OWNER).to_string();
        let cw20_addr = self.instantiate_default_cw20(&mut app, cw20_code_id, &owner_addr);

        // Resolve deferred cw20 deposit with actual deployed cw20 address.
        if let Some(amount) = self.cw20_deposit_amount {
            self.required_deposit = Some(AssetUnchecked {
                denom: UncheckedDenom::Cw20(cw20_addr.to_string()),
                amount: Uint256::from(amount),
            });
        }

        // Instantiate default contracts.
        let gauge_adapter_addr =
            self.instantiate_marketing_gauge_adapter(&mut app, gauge_adapter_code_id, &owner_addr);

        // Mint initial native token if any.
        app.init_modules(|router, _, storage| -> AnyResult<()> {
            for (addr, coin) in self.funds {
                router.bank.init_balance(storage, &addr, coin).map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            Ok(())
        })
        .unwrap();

        Suite {
            owner,
            app,
            gauge_adapter: gauge_adapter_addr,
            default_cw20: cw20_addr,
            cw20_code_id,
        }
    }
}

pub struct Suite {
    pub owner: Addr,
    pub app: App,
    pub gauge_adapter: Addr,
    pub default_cw20: Addr,
    // This is stored to instantiate other cw20 tokens in tests.
    cw20_code_id: u64,
}

impl Suite {
    // ---------------------------------------------------------------------------------------------
    // Execute
    // ---------------------------------------------------------------------------------------------
    pub fn execute_create_submission(
        &mut self,
        sender: Addr,
        name: String,
        url: String,
        address: String,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender,
            self.gauge_adapter.clone(),
            &ExecuteMsg::CreateSubmission {
                name,
                url,
                address,
                message: default_submission_msg(),
            },
            funds,
        ).map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub fn execute_receive_through_cw20(
        &mut self,
        sender: Addr,
        name: String,
        url: String,
        address: String,
        // amount refers to CW20 amount.
        amount: u128,
        cw20_addr: Addr,
    ) -> AnyResult<AppResponse> {
        let msg: Binary = to_json_binary(&ReceiveMsg::CreateSubmission {
            name,
            url,
            address,
            message: default_submission_msg(),
        }).map_err(|e| anyhow::anyhow!("{e}"))?;

        self.app.execute_contract(
            sender,
            cw20_addr,
            &Cw20BaseExecuteMsg::Send {
                contract: self.gauge_adapter.to_string(),
                amount: Uint256::from(amount),
                msg,
            },
            &[],
        ).map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub fn execute_return_deposit(&mut self, sender: &str) -> AnyResult<AppResponse> {
        // sender is expected to already be a valid bech32 addr (e.g. from suite.owner)
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.gauge_adapter.clone(),
            &ExecuteMsg::ReturnDeposits {},
            &[],
        ).map_err(|e| anyhow::anyhow!("{e}"))
    }

    // ---------------------------------------------------------------------------------------------
    // Queries
    // ---------------------------------------------------------------------------------------------
    pub fn query_submission(&self, address: String) -> AnyResult<SubmissionResponse> {
        self.app.wrap().query_wasm_smart(
            self.gauge_adapter.clone(),
            &AdapterQueryMsg::Submission { address },
        ).map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub fn query_submissions(&self) -> AnyResult<Vec<SubmissionResponse>> {
        let res: AllSubmissionsResponse = self.app.wrap().query_wasm_smart(
            self.gauge_adapter.clone(),
            &AdapterQueryMsg::AllSubmissions {},
        ).map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(res.submissions)
    }

    pub fn query_all_options(&self) -> AnyResult<Vec<String>> {
        let res: AllOptionsResponse = self
            .app
            .wrap()
            .query_wasm_smart(self.gauge_adapter.clone(), &AdapterQueryMsg::AllOptions {})
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(res.options)
    }

    pub fn query_check_option(&self, option: String) -> AnyResult<bool> {
        let res: CheckOptionResponse = self.app.wrap().query_wasm_smart(
            self.gauge_adapter.clone(),
            &AdapterQueryMsg::CheckOption { option },
        ).map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(res.valid)
    }

    // ---------------------------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------------------------

    // Instantiate a cw20 token and assign to address a specific amount.
    pub fn instantiate_token(&mut self, address: &str, token: &str, amount: u128) -> Addr {
        // address is expected to already be a valid bech32 addr (e.g. from suite.owner)
        self.app
            .instantiate_contract(
                self.cw20_code_id,
                Addr::unchecked(address),
                &Cw20BaseInstantiateMsg {
                    name: token.to_owned(),
                    symbol: token.to_owned(),
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: address.to_owned(),
                        amount: Uint256::from(amount),
                    }],
                    mint: None,
                    marketing: None,
                },
                &[],
                token,
                None,
            )
            .unwrap()
    }

    pub fn query_cw20_balance(&self, user: &str, contract: &Addr) -> AnyResult<u128> {
        let balance: BalanceResponse = self.app.wrap().query_wasm_smart(
            contract,
            &Cw20QueryMsg::Balance {
                address: user.to_owned(),
            },
        ).map_err(|e| anyhow::anyhow!("{e}"))?;

        balance.balance.to_string().parse::<u128>().map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub fn query_native_balance(&self, user: &str) -> AnyResult<u128> {
        let balance = self.app.wrap().query_balance(user, NATIVE).map_err(|e| anyhow::anyhow!("{e}"))?;

        balance.amount.to_string().parse::<u128>().map_err(|e| anyhow::anyhow!("{e}"))
    }
}
