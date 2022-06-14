#[cfg(not(feature = "library"))]
use crate::ContractError;
use crate::state::{Config, CONFIG, PRICE};
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
const CONTRACT_NAME: &str = "marble-collection";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTANTIATE_TOKEN_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, crate::ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;


    if msg.max_tokens == 0 {
        return Err(crate::ContractError::InvalidMaxTokens {});
    }

    let config = Config {
        owner: info.sender,
        cw20_address: msg.cw20_address,
        cw721_address: None,
        max_tokens: msg.max_tokens,
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        unused_token_id: 0,
        royalty: msg.royalty,
        uri: msg.uri
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
            label: msg.name.clone(),
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
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, crate::ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if config.cw721_address != None {
        return Err(crate::ContractError::Cw721AlreadyLinked {});
    }

    if msg.id != INSTANTIATE_TOKEN_REPLY_ID {
        return Err(crate::ContractError::InvalidTokenReplyId {});
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
        QueryMsg::GetPrice {token_id} => to_binary(&query_get_price(deps, token_id)?)
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner,
        cw20_address: config.cw20_address,
        cw721_address: config.cw721_address,
        max_tokens: config.max_tokens,
        name: config.name,
        symbol: config.symbol,
        unused_token_id: config.unused_token_id,
        royalty: config.royalty,
        uri: config.uri
    })
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
) -> Result<Response, crate::ContractError> {
    match msg {
        
        ExecuteMsg::Mint{ uri, price, extension } => {
            execute_mint(deps, env, info, uri, price, extension)
        },
        ExecuteMsg::BatchMint{ uri, price, extension} => {
            execute_batch_mint(deps, env, info, uri, price, extension)
        },

        ExecuteMsg::Receive(msg) => execute_cw20_buy_move(deps, env, info, msg),
        
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
        } => execute_update_price(deps, info, token_id, price),
        ExecuteMsg::UpdateUnusedTokenId {
            token_id
        } => execute_update_unused_token_id(deps, info, token_id)
    }
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    uri: String,
    price: Uint128,
    extension: Extension
) -> Result<Response, crate::ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    // if info.sender != config.owner {
    //     return Err(crate::ContractError::Unauthorized {});
    // }
    if config.cw721_address == None {
        return Err(crate::ContractError::Uninitialized {});
    }

    if config.unused_token_id >= config.max_tokens {
        return Err(crate::ContractError::SoldOut {});
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
) -> Result<Response, crate::ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    // if info.sender != config.owner {
    //     return Err(crate::ContractError::Unauthorized {});
    // }

    if uri.len() != price.len() {
        return Err(crate::ContractError::CountNotMatch {});
    }

    if config.cw721_address == None {
        return Err(crate::ContractError::Uninitialized {});
    }

    if config.unused_token_id >= config.max_tokens {
        return Err(crate::ContractError::SoldOut {});
    }

    let count = uri.len();
    let mut token_id:Vec<String> = vec![];
    for i in 0..count {
        token_id.push(config.unused_token_id.to_string());
        PRICE.save(deps.storage, config.unused_token_id, &price[i])?;
        config.unused_token_id += 1;
    }
    
    let mint_msg = Cw721ExecuteMsg::BatchMint(BatchMintMsg::<Extension> {
        token_id: token_id,
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



pub fn send_nft (
    deps: DepsMut,
    _env: Env,
    recipient: String,
    token_id: u32,
    cw20_amount: Uint128,
    funds: Option<Coin>
) -> Result<Response, crate::ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    let mut action;

    action = "buy_cw20";


    let mut msgs: Vec<CosmosMsg> = vec![];
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.cw721_address.clone().unwrap().to_string(),
        funds: vec![],
        msg: to_binary(&Cw721ExecuteMsg::<Extension>::TransferNft {
            recipient: recipient.clone(),
            token_id: token_id.to_string()
        })?,
    }));
    if cw20_amount > Uint128::zero() {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.cw20_address.clone().into(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: cfg.owner.clone().into(),
                amount: cw20_amount * Uint128::from(cfg.royalty) / Uint128::from(100u128),
            })?,
            funds: vec![],
        }));

        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.cw20_address.clone().into(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient.clone(),
                amount: cw20_amount * Uint128::from(100-cfg.royalty) / Uint128::from(100u128),
            })?,
            funds: vec![],
        }));
    }

    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", action)
        .add_attribute("address", recipient.clone());

    Ok(res)
}

pub fn execute_cw20_buy_move(
    deps: DepsMut, 
    env: Env,
    info: MessageInfo, 
    wrapper: Cw20ReceiveMsg
) -> Result<Response, crate::ContractError> {
    
    let mut cfg = CONFIG.load(deps.storage)?;

    if cfg.cw20_address != info.sender {
        return Err(crate::ContractError::InvalidCw20Token {})
    }

    if cfg.unused_token_id == 0 {
        return Err(crate::ContractError::NotMinted {})
    }

    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let balance = Cw20CoinVerified {
        address: info.sender.clone(),
        amount: wrapper.amount,
    };
    let mut price:Uint128 = Uint128::zero();
    let mut sell_id = 0u32;
    let mut rec_addr:Addr;

    match msg {
        ReceiveMsg::Move {token_id, recipient} => {
            
            sell_id = token_id;
            price = PRICE.load(deps.storage, sell_id)?;
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
                return Err(crate::ContractError::Unauthorized {});
            }
        },
        ReceiveMsg::Buy {token_id, recipient, sale_price} => {
            sell_id = token_id;
            price = sale_price;
            // price = price.checked_mul(Uint128::from(cfg.royalty as u64)).unwrap().checked_div(Uint128::from(100u64)).unwrap();
            rec_addr = recipient;
    
        }
    }
    if balance.amount < price {
        return Err(crate::ContractError::InsufficientFund {});
    }
    
    send_nft(deps, env, rec_addr.to_string(), sell_id, balance.amount, None)
    
}


pub fn execute_change_contract(
    deps: DepsMut,
    info: MessageInfo,
    cw721_address: Addr
) -> Result<Response, crate::ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(crate::ContractError::Unauthorized {});
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
) -> Result<Response, crate::ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(crate::ContractError::Unauthorized {});
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
) -> Result<Response, crate::ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(crate::ContractError::Unauthorized {});
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
) -> Result<Response, crate::ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(crate::ContractError::Unauthorized {});
    }
    
    if token_id.len() != price.len() {
        return Err(crate::ContractError::WrongLength {});
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


pub fn execute_update_unused_token_id(
    deps: DepsMut,
    info: MessageInfo,
    token_id: u32
) -> Result<Response, crate::ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(crate::ContractError::Unauthorized {});
    }
    config.unused_token_id = token_id;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "change_unused_token_id")
        .add_attribute("token_id", token_id.to_string())
    )
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, crate::ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(crate::ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}
