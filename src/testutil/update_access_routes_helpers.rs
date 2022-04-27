use crate::execute::update_access_routes::UpdateAccessRoutesV1;
use cosmwasm_std::MessageInfo;

pub struct TestUpdateAccessRoutes {
    pub info: MessageInfo,
    pub update_access_definitions: UpdateAccessRoutesV1,
}
impl TestUpdateAccessRoutes {}
