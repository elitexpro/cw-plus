#[cfg(not(feature = "library"))]
use crate::error::ContractError;
use crate::state::{Config, CLAIM, CONFIG, MERKLE_ROOT, STAGE_AMOUNT_CLAIMED,
    STAGE_EXPIRATION, STAGE_START, PRICE, SOLD};
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response,
    StdResult, SubMsg, Uint128, WasmMsg, Coin, from_binary, BankMsg, QueryRequest, WasmQuery
};
use cw2::set_contract_version;
use cw721::{
    OwnerOfResponse,
    
};
use cw2::{get_contract_version};

use cw_storage_plus::Bound;
use cw721_base::{
    msg::ExecuteMsg as Cw721ExecuteMsg, msg::InstantiateMsg as Cw721InstantiateMsg, Extension,
    msg::MintMsg, msg::BatchMintMsg, msg::QueryMsg as Cw721QueryMsg, 
};
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, MerkleRootResponse, IsClaimedResponse, PriceListResponse, PriceInfo, MigrateMsg};
use cw_utils::{Expiration, Scheduled};
use cw20::{Cw20ReceiveMsg, Cw20ExecuteMsg, Cw20CoinVerified, Balance};
use cw_utils::parse_reply_instantiate_data;
use sha2::Digest;
use std::convert::TryInto;


// version info for migration info
const CONTRACT_NAME: &str = "marble-presaleairdrop";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MERKLE_STAGE:u8 = 1;
const INSTANTIATE_TOKEN_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;


    if msg.max_tokens == 0 {
        return Err(ContractError::InvalidMaxTokens {});
    }

    let config = Config {
        owner: info.sender,
        pay_native: msg.pay_native,
        native_denom: msg.native_denom,
        cw20_address: msg.cw20_address,
        airdrop: msg.airdrop,
        cw721_address: None,
        max_tokens: msg.max_tokens,
        sold_cnt: 0,
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        unused_token_id: 0,
        royalty: msg.royalty
    };

    CONFIG.save(deps.storage, &config)?;

    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: msg.token_code_id,
            msg: to_binary(&Cw721InstantiateMsg {
                name: msg.name.clone(),
                symbol: msg.symbol,
                minter: env.contract.address.to_string(),
            })?,
            funds: vec![],
            admin: None,
            label: String::from("cw721-base for Marblenauts"),
        }
        .into(),
        id: INSTANTIATE_TOKEN_REPLY_ID,
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }];

    Ok(Response::new().add_submessages(sub_msg))
}

// Reply callback triggered from cw721 contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if config.cw721_address != None {
        return Err(ContractError::Cw721AlreadyLinked {});
    }

    if msg.id != INSTANTIATE_TOKEN_REPLY_ID {
        return Err(ContractError::InvalidTokenReplyId {});
    }

    let reply = parse_reply_instantiate_data(msg).unwrap();
    config.cw721_address = Addr::unchecked(reply.contract_address).into();
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::MerkleRoot {} => to_binary(&query_merkle_root(deps)?),
        QueryMsg::IsClaimed { address } => {
            to_binary(&query_is_claimed(deps, address)?)
        },
        QueryMsg::GetPrice {token_id} => to_binary(&query_get_price(deps, token_id)?)
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner,
        pay_native: config.pay_native,
        airdrop: config.airdrop,
        native_denom: config.native_denom,
        cw20_address: config.cw20_address,
        cw721_address: config.cw721_address,
        max_tokens: config.max_tokens,
        sold_cnt: config.sold_cnt,
        name: config.name,
        symbol: config.symbol,
        unused_token_id: config.unused_token_id,
        royalty: config.royalty
    })
}


pub fn query_merkle_root(deps: Deps) -> StdResult<MerkleRootResponse> {
    let merkle_root = MERKLE_ROOT.load(deps.storage, MERKLE_STAGE)?;
    let expiration = STAGE_EXPIRATION.load(deps.storage, MERKLE_STAGE)?;
    let start = STAGE_START.may_load(deps.storage, MERKLE_STAGE)?;
    let resp = MerkleRootResponse {
        stage:MERKLE_STAGE,
        merkle_root,
        expiration,
        start,
    };

    Ok(resp)
}

pub fn query_is_claimed(deps: Deps, address: String) -> StdResult<IsClaimedResponse> {
    let key: (&Addr, u8) = (&deps.api.addr_validate(&address)?, MERKLE_STAGE);
    let is_claimed = CLAIM.may_load(deps.storage, key)?.unwrap_or(false);
    let resp = IsClaimedResponse { is_claimed };

    Ok(resp)
}


fn query_get_price(
    deps: Deps,
    token_id: Vec<u32>,
) -> StdResult<PriceListResponse> {

    let count = token_id.len();
    let mut ret = vec![];
    for i in 0..count {
        ret.push(PriceInfo {
            token_id: token_id[i],
            price: PRICE.load(deps.storage, token_id[i])?
        });
    }

    Ok(PriceListResponse { prices: ret })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        
        ExecuteMsg::Mint{ uri, price, extension } => {
            execute_mint(deps, env, info, uri, price, extension)
        },
        ExecuteMsg::BatchMint{ uri, price, extension} => {
            execute_batch_mint(deps, env, info, uri, price, extension)
        },
        ExecuteMsg::Receive(msg) => execute_cw20_buy_move(deps, env, info, msg),
        
        ExecuteMsg::BuyNative {} => execute_native_buy_move(deps, env, info, None, true, None),
        ExecuteMsg::MoveNative {token_id, recipient} => execute_native_buy_move(deps, env, info, Some(token_id), false, Some(recipient)),

        ExecuteMsg::RegisterMerkleRoot {
            merkle_root,
            expiration,
            start,
        } => execute_register_merkle_root(
            deps,
            env,
            info,
            merkle_root,
            expiration,
            start,
        ),
        ExecuteMsg::Claim {
            proof,
        } => execute_claim(deps, env, info,  proof),
        ExecuteMsg::ChangeContract {    //Change the holding CW721 contract address
            cw721_address
        } => execute_change_contract(deps, info, cw721_address),
        ExecuteMsg::ChangeOwner {       //Change the owner of marblenft contract
            owner
        } => execute_change_owner(deps, info, owner),
        ExecuteMsg::ChangeCw721Owner {       //Change the owner of Cw721 contract
            owner
        } => execute_change_cw721_owner(deps, info, owner),
        ExecuteMsg::UpdatePrice {
            token_id,
            price
        } => execute_update_price(deps, info, token_id, price)
    }
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    uri: String,
    price: Uint128,
    extension: Extension
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    if config.cw721_address == None {
        return Err(ContractError::Uninitialized {});
    }

    if config.unused_token_id >= config.max_tokens {
        return Err(ContractError::SoldOut {});
    }

    let mint_msg = Cw721ExecuteMsg::Mint(MintMsg::<Extension> {
        token_id: config.unused_token_id.to_string(),
        owner: env.contract.address.into(),
        token_uri: uri.clone().into(),
        extension: extension.clone(),
    });

    let callback = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.cw721_address.clone().unwrap().to_string(),
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    });

    PRICE.save(deps.storage, config.unused_token_id, &price)?;
    config.unused_token_id += 1;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_message(callback))
}

pub fn execute_batch_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    uri: Vec<String>,
    price: Vec<Uint128>,
    extension: Vec<Extension>
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if uri.len() != price.len() {
        return Err(ContractError::CountNotMatch {});
    }

    if config.cw721_address == None {
        return Err(ContractError::Uninitialized {});
    }

    if config.unused_token_id >= config.max_tokens {
        return Err(ContractError::SoldOut {});
    }

    let count = uri.len();
    let mut token_id:Vec<String> = vec![];
    for i in 0..count {
        token_id.push(config.unused_token_id.to_string());
        PRICE.save(deps.storage, config.unused_token_id, &price[i])?;
        config.unused_token_id += 1;
    }
    
    let mint_msg = Cw721ExecuteMsg::BatchMint(BatchMintMsg::<Extension> {
        token_id,
        owner: env.contract.address.into(),
        token_uri: uri,
        extension: extension.clone(),
    });

    let callback = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.cw721_address.clone().unwrap().to_string(),
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    });

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_message(callback))
}



pub fn get_rand_sell_id (
    deps: &DepsMut,
    env: &Env,
) -> Result<u32, ContractError> {
    let mut sell_id = 0u32;
    let cfg = CONFIG.load(deps.storage)?;
    let cycle = (env.block.time.seconds() % (cfg.unused_token_id as u64)) as u32;
    let mut cnt = 0u32;
    let mut pos = 0u32;
    while cnt < cycle {
        let solded = SOLD.may_load(deps.storage, pos)?;
        if solded.is_none() {
            cnt += 1;
        } 
        if cnt == cycle {
            sell_id = pos;
            break;
        }
        pos = (pos + 1) % cfg.unused_token_id;
    }
    return Ok(sell_id);
}

pub fn send_nft (
    deps: DepsMut,
    _env: Env,
    recipient: String,
    token_id: u32,
    cw20_amount: Uint128,
    funds: Option<Coin>
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    let mut action;

    let solded = SOLD.may_load(deps.storage, token_id)?;
    if solded.is_some() {
        return Err(ContractError::AlreadySold {})
    }
    SOLD.save(deps.storage, token_id, &true)?;
    cfg.sold_cnt += 1u32;
    CONFIG.save(deps.storage, &cfg);
    
    if cfg.airdrop {
        action = "airdrop";
    } else if cfg.pay_native {
        action = "buy_native";
    } else {
        action = "buy_cw20";
    }
    let mut submsgs: Vec<SubMsg> = if !cfg.airdrop && cfg.pay_native {
        vec![SubMsg::new(BankMsg::Send {
            to_address: cfg.owner.clone().into(),
            amount: vec![funds.unwrap()],
        })]
        
    } else {
        vec![]
    };

    let mut msgs: Vec<CosmosMsg> = vec![];
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.cw721_address.clone().unwrap().to_string(),
        funds: vec![],
        msg: to_binary(&Cw721ExecuteMsg::<Extension>::TransferNft {
            recipient: recipient.clone(),
            token_id: token_id.to_string()
        })?,
    }));
    if !cfg.airdrop && !cfg.pay_native {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.cw20_address.into(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: cfg.owner.clone().into(),
                amount: cw20_amount,
            })?,
            funds: vec![],
        }));
    }

    let mut res = Response::new()
        .add_messages(msgs)
        .add_submessages(submsgs)
        .add_attribute("action", action)
        .add_attribute("address", recipient.clone());

    Ok(res)
}

pub fn execute_cw20_buy_move(
    deps: DepsMut, 
    env: Env,
    info: MessageInfo, 
    wrapper: Cw20ReceiveMsg
) -> Result<Response, ContractError> {
    
    let mut cfg = CONFIG.load(deps.storage)?;

    if cfg.airdrop || cfg.pay_native {
        return Err(ContractError::NotSupported {})
    }
    
    if cfg.cw20_address != info.sender {
        return Err(ContractError::InvalidCw20Token {})
    }

    if cfg.unused_token_id == 0 {
        return Err(ContractError::NotMinted {})
    }

    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let balance = Cw20CoinVerified {
        address: info.sender.clone(),
        amount: wrapper.amount,
    };
    let mut price:Uint128;
    let mut sell_id = 0u32;
    let mut rec_addr:Addr;

    match msg {
        ReceiveMsg::Buy {token_id, recipient} => {
            
            let is_rand = token_id.map_or(true, |v| (false));
            if is_rand {
                sell_id = get_rand_sell_id(&deps, &env).unwrap();
            } else {
                sell_id = token_id.unwrap();
            }
            price = PRICE.load(deps.storage, sell_id)?;
            rec_addr = recipient;
        },
        ReceiveMsg::Move {token_id, recipient} => {
            sell_id = token_id;
            price = PRICE.load(deps.storage, sell_id)?;
            price = price.checked_mul(Uint128::from(cfg.royalty as u64)).unwrap().checked_div(Uint128::from(100u64)).unwrap();
            rec_addr = recipient;

            //check whether info.sender is the owner of token_id
            let owner_of: OwnerOfResponse =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: cfg.cw721_address.clone().unwrap().to_string(),
                msg: to_binary(&Cw721QueryMsg::OwnerOf {
                    token_id: token_id.to_string(),
                    include_expired: Some(true)

                })?,
            }))?;

            if owner_of.owner != info.sender.clone().to_string() {
                return Err(ContractError::Unauthorized {});
            }
    
        }
    }
    if balance.amount < price {
        return Err(ContractError::InsufficientFund {});
    }
    
    cfg.sold_cnt += 1;
    CONFIG.save(deps.storage, &cfg)?;

    send_nft(deps, env, rec_addr.to_string(), sell_id, balance.amount, None)
    
}

pub fn execute_native_buy_move(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: Option<u32>,
    is_buy: bool,
    recipient:Option<Addr>
    
) -> Result<Response, ContractError> {

    let mut cfg = CONFIG.load(deps.storage)?;

    if cfg.airdrop || !cfg.pay_native {
        return Err(ContractError::NotSupported {});
    }
    
    if cfg.unused_token_id == 0 {
        return Err(ContractError::NotMinted {})
    }
    
    let mut funds = Coin {
        amount: Uint128::new(0),
        denom: cfg.native_denom.clone()
    };

    for coin in &info.funds {
        if coin.denom == cfg.native_denom.clone() {
            funds = Coin {
                amount: coin.amount,
                denom: funds.denom,
            }
        }
    }
    
    let mut sell_id = 0u32;
    let mut price:Uint128;
    let rec_addr:Addr;
    if is_buy {
        let is_rand = token_id.map_or(true, |v| (false));
        if is_rand {
            sell_id = get_rand_sell_id(&deps, &env).unwrap();
        } else {
            sell_id = token_id.unwrap();
        }
        price = PRICE.load(deps.storage, sell_id)?;
        rec_addr = info.sender.clone();
    } else {
        sell_id = token_id.unwrap();
        price = PRICE.load(deps.storage, sell_id)?;
        price = price.checked_mul(Uint128::from(cfg.royalty as u64)).unwrap().checked_div(Uint128::from(100u64)).unwrap();
        rec_addr = recipient.unwrap().clone();

        //check whether info.sender is the owner of token_id
        let owner_of: OwnerOfResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: cfg.cw721_address.clone().unwrap().to_string(),
            msg: to_binary(&Cw721QueryMsg::OwnerOf {
                token_id: sell_id.to_string(),
                include_expired: Some(true)

            })?,
        }))?;

        if owner_of.owner != info.sender.clone().to_string() {
            return Err(ContractError::Unauthorized {});
        }
    }

    if funds.amount < price {
        return Err(ContractError::InsufficientFund {});
    }

    cfg.sold_cnt += 1;
    CONFIG.save(deps.storage, &cfg)?;

    send_nft(deps, env, rec_addr.into(), sell_id, Uint128::zero(), Some(funds))
}

pub fn execute_register_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    merkle_root: String,
    expiration: Option<Expiration>,
    start: Option<Scheduled>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    // if owner set validate, otherwise unauthorized
    if info.sender != cfg.owner || !cfg.airdrop {
        return Err(ContractError::Unauthorized {});
    }

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root.to_string(), &mut root_buf).unwrap();

    MERKLE_ROOT.save(deps.storage, MERKLE_STAGE, &merkle_root)?;

    // save expiration
    let exp = expiration.unwrap_or(Expiration::Never {});
    STAGE_EXPIRATION.save(deps.storage, MERKLE_STAGE, &exp)?;

    // save start
    if let Some(start) = start {
        STAGE_START.save(deps.storage, MERKLE_STAGE, &start)?;
    }

    // save total airdropped amount
    STAGE_AMOUNT_CLAIMED.save(deps.storage, MERKLE_STAGE, &0u32)?;

    Ok(Response::new()
        .add_attribute("action", "register_merkle_root")
        .add_attribute("merkle_root", merkle_root)
        .add_submessages(vec![]))
            
}

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proof: Vec<String>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    // just enable on airdrop mode
    if !cfg.airdrop {
        return Err(ContractError::NotSupported {});
    }
    
    if cfg.unused_token_id == 0 {
        return Err(ContractError::NotMinted {})
    }
    // airdrop begun
    let start = STAGE_START.may_load(deps.storage, MERKLE_STAGE)?;
    if let Some(start) = start {
        if !start.is_triggered(&env.block) {
            return Err(ContractError::StageNotBegun { stage:MERKLE_STAGE, start });
        }
    }
    // not expired
    let expiration = STAGE_EXPIRATION.load(deps.storage, MERKLE_STAGE)?;
    if expiration.is_expired(&env.block) {
        return Err(ContractError::StageExpired { stage:MERKLE_STAGE, expiration });
    }

    // verify not claimed
    let claimed = CLAIM.may_load(deps.storage, (&info.sender, MERKLE_STAGE))?;
    if claimed.is_some() {
        return Err(ContractError::Claimed {});
    }
    let merkle_root = MERKLE_ROOT.load(deps.storage, MERKLE_STAGE)?;

    
    let user_input = format!("{}{}", info.sender, Uint128::from(1u128));
    let hash = sha2::Sha256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .map_err(|_| ContractError::WrongLength {})?;

    let hash = proof.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        
        hex::decode_to_slice(p, &mut proof_buf).unwrap();
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(&hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::WrongLength {})
    })?;

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf).unwrap();
    if root_buf != hash {
        return Err(ContractError::VerificationFailed {});
    }

    // Update claim index to the current stage
    CLAIM.save(deps.storage, (&info.sender, MERKLE_STAGE), &true)?;

    // Update total claimed to reflect
    let mut claimed_amount = STAGE_AMOUNT_CLAIMED.load(deps.storage, MERKLE_STAGE)?;
    let total_amount = cfg.unused_token_id;
    if claimed_amount >= cfg.unused_token_id {
        return Err(ContractError::AlreadySold {});
    }
    claimed_amount += 1;
    STAGE_AMOUNT_CLAIMED.save(deps.storage, MERKLE_STAGE, &claimed_amount)?;

    let sell_id = get_rand_sell_id(&deps, &env).unwrap();
    send_nft(deps, env, info.sender.into(), sell_id, Uint128::zero(), None)
}

pub fn execute_change_contract(
    deps: DepsMut,
    info: MessageInfo,
    cw721_address: Addr
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    config.cw721_address = Some(cw721_address.clone());
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "change_contract")
        .add_attribute("cw721_address", cw721_address.to_string())
        .add_submessages(vec![]))
}

pub fn execute_change_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: Addr
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    config.owner = owner.clone();
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "change_owner")
        .add_attribute("owner", owner.to_string())
        .add_submessages(vec![]))
}


pub fn execute_change_cw721_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: Addr
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    config.owner = owner.clone();
    CONFIG.save(deps.storage, &config)?;


    let change_msg = Cw721ExecuteMsg::<Extension>::ChangeOwner {
        owner: owner.clone().into()
    };

    let callback = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.cw721_address.clone().unwrap().to_string(),
        msg: to_binary(&change_msg)?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(callback)
        .add_attribute("action", "change_cw721_owner")
        .add_attribute("owner", owner.to_string())
        .add_submessages(vec![]))
}

pub fn execute_update_price(
    deps: DepsMut,
    info: MessageInfo,
    token_id: Vec<u32>,
    price: Vec<Uint128>
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    
    if token_id.len() != price.len() {
        return Err(ContractError::WrongLength {});
    }
    let count = token_id.len();
    for i in 0..count {
        PRICE.save(deps.storage, token_id[i], &price[i])?;
    }

    Ok(Response::new()
        .add_attribute("action", "change_price")
        .add_attribute("count", count.to_string())
        .add_submessages(vec![]))
}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

