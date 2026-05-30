use cw_orch::prelude::*;
use dao_calendar::contract::CalendarModuleCollectionExtension;
use dao_cw_orch::*;

/// Deploy data for the calendar module.
#[derive(Clone, Debug, Default)]
pub struct CalendarDeployData {
    pub name: String,
    pub symbol: String,
    pub ext: CalendarModuleCollectionExtension,
    pub minter: Option<String>,
    pub creator: Option<String>,
    pub withdrawer: Option<String>,
}

impl crate::DaoDeployData for CalendarDeployData {
    type Init = dao_calendar::contract::InstantiateMsg;

    fn into_init(self) -> Self::Init {
        Self::Init {
            name: self.name,
            symbol: self.symbol,
            collection_info_extension: self.ext,
            minter: self.minter,
            creator: self.creator,
            withdraw_address: self.withdrawer,
        }
    }
}

/// Deploy data for proposal modules (placeholder for future config).
#[derive(Clone, Debug, Default)]
pub struct DaoProposalDeployData;

/// Pre-propose module interfaces.
pub struct DaoPreProposeSuite<Chain: CwEnv> {
    pub pre_prop_approval_single: DaoPreProposeApprovalSingle<Chain>,
    pub pre_prop_approver: DaoPreProposeApprover<Chain>,
    pub pre_prop_multiple: DaoPreProposeMultiple<Chain>,
    pub pre_prop_single: DaoPreProposeSingle<Chain>,
}

pub use dao_calendar::contract::msg::ExecuteExtFns as _;

impl<Chain: CwEnv> DaoPreProposeSuite<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            pre_prop_approval_single: DaoPreProposeApprovalSingle::new(
                "dao_pre_propose_approval_single",
                chain.clone(),
            ),
            pre_prop_approver: DaoPreProposeApprover::new(
                "dao_pre_propose_approver",
                chain.clone(),
            ),
            pre_prop_multiple: DaoPreProposeMultiple::new(
                "dao_pre_propose_multiple",
                chain.clone(),
            ),
            pre_prop_single: DaoPreProposeSingle::new("dao_pre_propose_single", chain),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.pre_prop_approval_single.upload()?;
        self.pre_prop_approver.upload()?;
        self.pre_prop_multiple.upload()?;
        self.pre_prop_single.upload()?;
        Ok(())
    }

    pub fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![
            Box::new(&mut self.pre_prop_approval_single),
            Box::new(&mut self.pre_prop_approver),
            Box::new(&mut self.pre_prop_multiple),
            Box::new(&mut self.pre_prop_single),
        ]
    }
}

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoPreProposeSuite<Chain> {
    type Error = CwOrchError;
    type DeployData = DaoProposalDeployData;

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

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        Self::store_on(chain)
    }
}

/// Proposal module interfaces (includes pre-propose as nested suite).
pub struct DaoProposalSuite<Chain: CwEnv> {
    pub prop_single: DaoProposalSingle<Chain>,
    pub prop_multiple: DaoProposalMultiple<Chain>,
    pub prop_condorcet: DaoProposalCondorcet<Chain>,
    pub prop_sudo: DaoProposalSudo<Chain>,
    pub pre_prop_suite: DaoPreProposeSuite<Chain>,
    pub calendar: DaoCalendar<Chain>,
}

impl<Chain: CwEnv> DaoProposalSuite<Chain> {
    pub fn new(chain: Chain) -> Self {
        Self {
            prop_single: DaoProposalSingle::new("dao_proposal_single", chain.clone()),
            prop_multiple: DaoProposalMultiple::new("dao_proposal_multiple", chain.clone()),
            prop_condorcet: DaoProposalCondorcet::new("dao_proposal_condorcet", chain.clone()),
            prop_sudo: DaoProposalSudo::new("dao_proposal_sudo", chain.clone()),
            pre_prop_suite: DaoPreProposeSuite::new(chain.clone()),
            calendar: DaoCalendar::new(chain),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.prop_single.upload()?;
        self.prop_multiple.upload()?;
        self.prop_condorcet.upload()?;
        self.prop_sudo.upload()?;
        self.pre_prop_suite.upload()?;
        self.calendar.upload()?;
        Ok(())
    }

    pub fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        let mut cs: Vec<Box<&mut dyn ContractInstance<Chain>>> = vec![
            Box::new(&mut self.prop_single),
            Box::new(&mut self.prop_multiple),
            Box::new(&mut self.prop_condorcet),
            Box::new(&mut self.prop_sudo),
            Box::new(&mut self.calendar),
        ];
        cs.extend(self.pre_prop_suite.get_contracts_mut());
        cs
    }
}

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoProposalSuite<Chain> {
    type Error = CwOrchError;
    type DeployData = DaoProposalDeployData;

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

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        Self::store_on(chain)
    }
}
