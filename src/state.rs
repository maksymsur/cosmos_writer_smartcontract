use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub data: Vec<u8>,
    pub admins: Vec<Addr>,
}

pub const STATE: Item<State> = Item::new("state");
