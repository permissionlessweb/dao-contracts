use cw_orch::prelude::*;
use cw_orch_polytone::Polytone;

/// Upload-only deploy data. Instantiation is optional and defaults to no admin
/// (immutable). The cw-orch Polytone suite owns note / voice / proxy.
#[derive(Clone, Debug, Default)]
pub struct DaoPolytoneDeployData;

/// Thin wrapper around `cw_orch_polytone::Polytone` so DaoDaoSuite can upload
/// polytone the same way it uploads every other module family.
pub struct DaoPolytoneSuite<Chain: CwEnv> {
    pub inner: Polytone<Chain>,
}

impl<Chain: CwEnv> DaoPolytoneSuite<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            inner: Polytone::new(chain),
        }
    }

    /// Store note / voice / proxy if this chain has no code IDs yet.
    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.inner.note.upload_if_needed()?;
        self.inner.voice.upload_if_needed()?;
        self.inner.proxy.upload_if_needed()?;
        Ok(())
    }

    /// Instantiate note + voice with no contract admin (immutable).
    pub fn instantiate_immutable(&self) -> Result<(), CwOrchError> {
        self.inner.instantiate_note(None)?;
        self.inner.instantiate_voice(None)?;
        Ok(())
    }
}

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoPolytoneSuite<Chain> {
    type Error = CwOrchError;
    type DeployData = DaoPolytoneDeployData;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        // Reuse the existing Polytone cw-orch suite.
        let inner = Polytone::store_if_needed(chain)?;
        Ok(Self { inner })
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        self.inner.get_contracts_mut()
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        Ok(Self {
            inner: Polytone::load_from(chain)?,
        })
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        // Upload only. Instantiation stays opt-in (immutable admin = None).
        Self::store_on(chain)
    }
}
