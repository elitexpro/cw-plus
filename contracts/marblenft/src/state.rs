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
    pub pay_native: bool,
    pub native_denom: String,
    pub cw20_address: Addr,
    pub airdrop: bool,
    pub cw721_address: Option<Addr>,
    pub max_tokens: u32,
    pub sold_cnt: u32,
    pub name: String,
    pub symbol: String,
    pub extension: Extension,
    pub unused_token_id: u32,
    pub royalty: u32
}


pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);


pub const PRICE_KEY: &str = "price";
pub const PRICE: Map<u32, Uint128> = Map::new(PRICE_KEY);

pub const STAGE_EXPIRATION_KEY: &str = "stage_exp";
pub const STAGE_EXPIRATION: Map<u8, Expiration> = Map::new(STAGE_EXPIRATION_KEY);

pub const STAGE_START_KEY: &str = "stage_start";
pub const STAGE_START: Map<u8, Scheduled> = Map::new(STAGE_START_KEY);

pub const STAGE_AMOUNT_CLAIMED_KEY: &str = "stage_claimed_amount";
pub const STAGE_AMOUNT_CLAIMED: Map<u8, u32> = Map::new(STAGE_AMOUNT_CLAIMED_KEY);

pub const MERKLE_ROOT_PREFIX: &str = "merkle_root";
pub const MERKLE_ROOT: Map<u8, String> = Map::new(MERKLE_ROOT_PREFIX);

pub const CLAIM_PREFIX: &str = "claim";
pub const CLAIM: Map<(&Addr, u8), bool> = Map::new(CLAIM_PREFIX);

pub const CLAIMED_AMOUNT_PREFIX: &str = "claimed_amount";
pub const CLAIMED_AMOUNT: Map<(&Addr, u8), bool> = Map::new(CLAIMED_AMOUNT_PREFIX);

pub const SOLD_PREFIX: &str = "sold";
pub const SOLD: Map<u32, bool> = Map::new(SOLD_PREFIX);