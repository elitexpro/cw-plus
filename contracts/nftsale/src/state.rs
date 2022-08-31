use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use cw_storage_plus::{Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub price: Uint128,
    pub count: u32,
    pub sold_count: u32,
    pub cw721_address: Addr,
    pub enabled: bool,
    pub denom: String,
    pub unsold_list_str: String
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const SALE_KEY: &str = "sale";
pub const SALE: Map<String, Addr> = Map::new(SALE_KEY);

