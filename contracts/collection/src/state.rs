use cw721_base::Extension;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use cw_utils::{Expiration, Scheduled};
use cw_storage_plus::{Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub cw20_address: Addr,
    pub cw721_address: Option<Addr>,
    pub max_tokens: u32,
    pub name: String,
    pub symbol: String,
    pub unused_token_id: u32,
    pub royalty: u32,
    pub uri: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SaleInfo {
    pub provider: Addr,
    pub sale_type: u64,
    pub steps: u64,
    pub price: Uint128,
    pub royalty: u32,
    pub uri: String
}



pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const PRICE_KEY: &str = "price";
pub const PRICE: Map<u32, Uint128> = Map::new(PRICE_KEY);

// pub const PRICE_KEY: &str = "price";
// pub const PRICE: Map<u32, Uint128> = Map::new(PRICE_KEY);

