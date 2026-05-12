use super::super::super::deploy_data::DaoDeployData;

/// Deploy data for the calendar module.
#[derive(Clone, Debug, Default)]
pub struct CalendarDeployData {
    pub initial_groups: Option<Vec<dao_calendar::msg::GroupInit>>,
}

impl DaoDeployData for CalendarDeployData {
    type Init = dao_calendar::msg::InstantiateMsg;

    fn into_init(self) -> Self::Init {
        dao_calendar::msg::InstantiateMsg {
            initial_groups: self.initial_groups,
        }
    }
}
