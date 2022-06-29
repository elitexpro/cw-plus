use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;
use cw721_base::Extension;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_utils::{Expiration, Scheduled};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub pay_native: bool,
    pub airdrop: bool,
    pub native_denom: String,
    pub max_tokens: u32,
    pub name: String,
    pub symbol: String,
    pub token_code_id: u64,
    pub cw20_address: Addr,
    pub extension: Extension,
    pub royalty: u32
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Mint {uri: String, price:Uint128, extension: Extension},
    BatchMint {uri: Vec<String>, price: Vec<Uint128>, extension:Vec<Extension>},
    BuyNative {},
    MoveNative { token_id:u32, recipient: Addr },
    Receive(Cw20ReceiveMsg),

    RegisterMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        merkle_root: String,
        expiration: Option<Expiration>,
        start: Option<Scheduled>,
    },
    /// Claim does not check if contract has enough funds, owner must ensure it.
    Claim {
        /// Proof is hex-encoded merkle proof.
        proof: Vec<String>,
    },
    ChangeContract {
        cw721_address: Addr
    },
    ChangeOwner {
        owner: Addr
    },
    ChangeCw721Minter {
        minter: Addr
    },
    UpdatePrice {
        token_id: Vec<u32>,
        price: Vec<Uint128>
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    Buy {
        token_id: Option<u32>,
        recipient: Addr
    },
    Move {
        token_id: u32,
        recipient: Addr
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig {},
    MerkleRoot {},
    IsClaimed {address:String},
    GetPrice {
        token_id: Vec<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
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
    pub unused_token_id: u32,
    pub royalty: u32
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
pub struct MigrateMsg {}
