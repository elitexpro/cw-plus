use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;

use cw721_base::Extension;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_utils::{Expiration, Scheduled};
use cw20::Denom;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub price: Uint128,
    pub denom: String,
    pub count: u32,
    pub cw721_address: Addr
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateOwner {
        owner: Addr,
    },
    UpdateEnabled {
        enabled: bool
    },
    Buy{},
    Send{
        token_id: String,
        address: Addr
    }
    
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig {},
    GetSoldState {
        token_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: Addr,
    pub price: Uint128,
    pub count: u32,
    pub sold_count: u32,
    pub cw721_address: Addr,
    pub enabled: bool,
    pub denom: String,
    pub unsold_list_str: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}