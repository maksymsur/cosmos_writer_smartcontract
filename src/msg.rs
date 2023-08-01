use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub data: Vec<u8>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Write { data: Vec<u8> },
    AddAdmin { admin: String },
    RemoveAdmin { admin: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetData returns the written data.
    // This is needed for tests mainly as we parse the data from the blocks
    #[returns(GetWriteResponse)]
    GetData {},
    // GetAdmins returns accounts with admin privilleges
    #[returns(GetAdminResponse)]
    GetAdmins {},
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetWriteResponse {
    pub data: Vec<u8>,
}

#[cw_serde]
pub struct GetAdminResponse {
    pub admins: Vec<Addr>,
}
