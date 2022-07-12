use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;
use cw721_base::Extension;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_utils::{Expiration, Scheduled};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub max_tokens: u32,
    pub name: String,
    pub symbol: String,
    pub token_code_id: u64,
    pub cw20_address: Addr,
    pub royalty: u32,
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
    Edit {token_id: String, uri: String, extension: Extension},
    BatchMint {
        uri: Vec<String>, 
        extension:Vec<Extension>,
        owner: Vec<String>
    },
    StartSale {
        token_id: u32,    
        sale_type: SaleType,
        duration_type: DurationType,
        initial_price: Uint128,
        royalty: u32
    },
    Propose {
        token_id: u32,
        price: Uint128
    },
    Receive(Cw20ReceiveMsg),
    ChangeContract {
        cw721_address: Addr
    },
    ChangeCw721Owner {
        owner: Addr
    },
    UpdatePrice {
        token_id: Vec<u32>,
        price: Vec<Uint128>
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
pub enum QueryMsg {
    GetConfig {},
    GetPrice {
        token_id: Vec<u32>,
    },
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
    pub royalty: u32,
    pub uri: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MerkleRootResponse {
    pub stage: u8,
    /// MerkleRoot is hex-encoded merkle root.
    pub merkle_root: String,
    pub expiration: Expiration,
    pub start: Option<Scheduled>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceInfo {
    pub token_id: u32,
    pub price: Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IsClaimedResponse {
    pub is_claimed: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PriceListResponse {
    pub prices: Vec<PriceInfo>,
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
pub enum DurationType {
    Fixed,
    Time(u64),
    Bid(u32)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SaleInfo {
    pub token_id: u32,
    pub provider: Addr,
    pub sale_type: SaleType,
    pub duration_type: DurationType,
    pub initial_price: Uint128,
    pub royalty: u32,
    pub requests: Vec<Request>,
    pub sell_index: u32
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}