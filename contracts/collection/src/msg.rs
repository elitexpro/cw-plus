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
    pub owner: Addr,
    pub max_tokens: u32,
    pub name: String,
    pub symbol: String,
    pub token_code_id: u64,
    pub cw20_address: Addr,
    pub collection_owner_royalty: u32,
    pub royalties: Vec<Royalty>,
    pub uri: String
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
    Mint {uri: String, extension: Extension},
    Edit {token_id: u32, uri: String, extension: Extension},
    BatchMint {
        uri: Vec<String>, 
        extension:Vec<Extension>,
        owner: Vec<String>
    },
    Propose {
        token_id: u32,
        price: Uint128
    },
    Receive(Cw20ReceiveMsg),
    ReceiveNft(Cw721ReceiveMsg),
    RemoveSale {
        token_id: u32,
    },
    Buy {
        token_id: u32,
        denom: String
    },
    ChangeContract {
        cw721_address: Addr
    },
    ChangeCw721Owner {
        owner: Addr
    },
    UpdateUnusedTokenId {
        token_id: u32
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    Buy {
        token_id: u32,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NftReceiveMsg {
    StartSale {
        sale_type: SaleType,
        duration_type: DurationType,
        initial_price: Uint128,
        reserve_price: Uint128
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig {},
    GetSale {
        token_id: u32,
    },
    GetSales {
        start_after: Option<u32>,
        limit: Option<u32>
    },
    GetBaseAmount {
        denom: Denom,
        amount: Uint128
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: Addr,
    pub cw20_address: Addr,
    pub cw721_address: Option<Addr>,
    pub max_tokens: u32,
    pub name: String,
    pub symbol: String,
    pub unused_token_id: u32,
    pub collection_owner_royalty: u32,
    pub royalties: Vec<Royalty>,
    pub uri: String,
    pub enabled: bool
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Request {
    pub address: Addr,
    pub price: Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum SaleType {
    Fixed,
    Auction,
    Offer
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TimeDuration {
    pub start: u64,
    pub end: u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum DurationType {
    Fixed,
    // Time(TimeDuration),
    Time(u64, u64),
    Bid(u32)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Royalty {
    pub address: Addr,
    pub rate: u32
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SaleInfo {
    pub token_id: u32,
    pub provider: Addr,
    pub sale_type: SaleType,
    pub duration_type: DurationType,
    pub initial_price: Uint128,
    pub reserve_price: Uint128,
    pub requests: Vec<Request>,
    pub sell_index: u32
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SalesResponse {
    pub list: Vec<SaleInfo>
}



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}