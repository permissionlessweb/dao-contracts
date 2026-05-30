use cw_orch::prelude::*;
use dao_cw_orch::*;

/// Composite deploy data for external modules.
#[derive(Clone, Debug, Default)]
pub struct DaoExternalDeployData {
    pub admin: Option<Addr>,
}

impl DaoExternalDeployData {
    /// Run preflight validation on all present module deploy data.
    pub fn preflight(&self) -> Result<(), String> {
        Ok(())
    }
}

/// External module interfaces.
pub struct DaoExternalSuite<Chain: CwEnv> {
    pub admin_factory: DaoExternalAdminFactory<Chain>,
    pub btsg_ft_factory: DaoExternalFantokenFactory<Chain>,
    pub payroll_factory: DaoExternalPayrollFactory<Chain>,
    pub cw_tokenswap: DaoExternalTokenSwap<Chain>,
    pub cw_tokenfactory_issuer: DaoExternalTokenfactoryIssuer<Chain>,
    pub cw_vesting: DaoExternalCwVesting<Chain>,
    pub cw721_roles: DaoExternalCw721Roles<Chain>,
}

impl<Chain: CwEnv> DaoExternalSuite<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            admin_factory: DaoExternalAdminFactory::new("cw_admin_factory", chain.clone()),
            btsg_ft_factory: DaoExternalFantokenFactory::new("btsg_ft_factory", chain.clone()),
            payroll_factory: DaoExternalPayrollFactory::new("cw_payroll", chain.clone()),
            cw_tokenswap: DaoExternalTokenSwap::new("cw_tokenswap", chain.clone()),
            cw_tokenfactory_issuer: DaoExternalTokenfactoryIssuer::new(
                "cw_tokenfactory",
                chain.clone(),
            ),
            cw_vesting: DaoExternalCwVesting::new("cw_vesting", chain.clone()),
            cw721_roles: DaoExternalCw721Roles::new("cw721_roles", chain),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.admin_factory.upload()?;
        self.btsg_ft_factory.upload()?;
        self.payroll_factory.upload()?;
        self.cw_tokenswap.upload()?;
        self.cw_tokenfactory_issuer.upload()?;
        self.cw_vesting.upload()?;
        self.cw721_roles.upload()?;
        Ok(())
    }

    pub fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![
            Box::new(&mut self.admin_factory),
            Box::new(&mut self.btsg_ft_factory),
            Box::new(&mut self.payroll_factory),
            Box::new(&mut self.cw_tokenswap),
            Box::new(&mut self.cw_tokenfactory_issuer),
            Box::new(&mut self.cw_vesting),
            Box::new(&mut self.cw721_roles),
        ]
    }
}

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoExternalSuite<Chain> {
    type Error = CwOrchError;
    type DeployData = DaoExternalDeployData;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let suite = Self::new(chain);
        suite.upload()?;
        Ok(suite)
    }
    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        self.get_contracts_mut()
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        Ok(Self::new(chain))
    }

    fn deploy_on(chain: Chain, data: Self::DeployData) -> Result<Self, Self::Error> {
        data.preflight().map_err(CwOrchError::StdErr)?;
        Self::store_on(chain)
    }
}