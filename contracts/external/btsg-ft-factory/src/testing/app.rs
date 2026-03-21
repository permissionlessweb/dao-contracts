use std::ops::{Deref, DerefMut};

use crate::testing::bitsong_stargate::StargateKeeper;
use cosmwasm_std::{testing::MockApi, Empty};
use cw_multi_test::{
    no_init, App, AppBuilder, BankKeeper, DistributionKeeper, FailingModule, GovFailingModule,
    IbcFailingModule, StakeKeeper, WasmKeeper,
};
use cosmwasm_std::testing::MockStorage;

#[allow(clippy::type_complexity)]
pub struct BitsongApp(
    App<
        BankKeeper,
        MockApi,
        MockStorage,
        FailingModule<Empty, Empty, Empty>,
        WasmKeeper<Empty, Empty>,
        StakeKeeper,
        DistributionKeeper,
        IbcFailingModule,
        GovFailingModule,
        StargateKeeper,
    >,
);
impl Deref for BitsongApp {
    type Target = App<
        BankKeeper,
        MockApi,
        MockStorage,
        FailingModule<Empty, Empty, Empty>,
        WasmKeeper<Empty, Empty>,
        StakeKeeper,
        DistributionKeeper,
        IbcFailingModule,
        GovFailingModule,
        StargateKeeper,
    >;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BitsongApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Default for BitsongApp {
    fn default() -> Self {
        Self::new()
    }
}

impl BitsongApp {
    pub fn new() -> Self {
        let app_builder = AppBuilder::default();
        let stargate = StargateKeeper {};
        let app = app_builder.with_stargate(stargate).build(no_init);
        BitsongApp(app)
    }
}
