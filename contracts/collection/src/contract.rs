#[cfg(not(feature = "library"))]
use crate::ContractError;
use crate::state::{Config, CONFIG, SALE};
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, Api,
    StdResult, SubMsg, Uint128, WasmMsg, Coin, from_binary, BankMsg, QueryRequest, WasmQuery, Storage, Order
};
use cw2::set_contract_version;
use cw721::{
    OwnerOfResponse,
    
};
use cw20::Denom;

use crate::constants::{ATOMPOOL, OSMOPOOL, USDCPOOL, SCRTPOOL, BLOCKATOMPOOL, BLOCKJUNOPOOL, BLOCKMARBLEPOOL, ATOMDENOM, OSMODENOM, USDCDENOM, SCRTDENOM, JUNODENOM, BLOCKADDR, MARBLEADDR};
use cw2::{get_contract_version};
use cw_storage_plus::Bound;
use cw721_base::{
    msg::ExecuteMsg as Cw721ExecuteMsg, msg::InstantiateMsg as Cw721InstantiateMsg, Extension, 
    msg::MintMsg, msg::BatchMintMsg, msg::QueryMsg as Cw721QueryMsg,  msg::EditMsg
};
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, MigrateMsg, SaleType, DurationType, SaleInfo, SalesResponse, Request};
use cw_utils::{Expiration, Scheduled};
use cw20::{Cw20ReceiveMsg, Cw20ExecuteMsg, Cw20CoinVerified, Balance};
use cw_utils::parse_reply_instantiate_data;
use sha2::Digest;
use std::convert::TryInto;

use crate::util;

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
        owner: msg.owner.clone(),
        cw20_address: msg.cw20_address,
        cw721_address: None,
        max_tokens: msg.max_tokens,
        name: msg.name.clone(),
        symbol: msg.symbol.clone(),
        unused_token_id: 0,
        royalty: msg.royalty,
        enabled: true,
        uri: msg.uri
    };

    CONFIG.save(deps.storage, &config)?;

    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: msg.token_code_id,
            msg: to_binary(&Cw721InstantiateMsg {
                name: msg.name.clone() + " cw721_base",
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
        QueryMsg::GetSale {token_id} => to_binary(&query_get_sale(deps, token_id)?),
        QueryMsg::GetSales {start_after, limit} => to_binary(&query_get_sales(deps, start_after, limit)?),
        QueryMsg::GetBaseAmount {denom, amount} => to_binary(&query_get_base_amount(deps, denom, amount)?)
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
        uri: config.uri,
        enabled: config.enabled
    })
}


fn query_get_sale(
    deps: Deps,
    token_id: u32,
) -> StdResult<SaleInfo> {

    let sale_info = SALE.load(deps.storage, token_id)?;
    Ok(sale_info)
}
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 20;


fn map_sales(
    item: StdResult<(u32, SaleInfo)>,
) -> StdResult<SaleInfo> {
    item.map(|(id, record)| {
        record
    })
}

fn query_get_sales(
    deps: Deps,
    start_after: Option<u32>,
    limit: Option<u32>
) -> StdResult<SalesResponse> {

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start = start_after.map(|str| Bound::exclusive(str.to_string()));
    
    let sales:StdResult<Vec<_>> = SALE
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| map_sales(item))
        .collect();

    Ok(SalesResponse {
        list: sales?
    })
    
}


fn query_get_base_amount(
    deps: Deps,
    denom: Denom,
    amount: Uint128
) -> StdResult<Uint128> {

    let cfg = CONFIG.load(deps.storage)?;
    match denom {
        Denom::Native(str) => {
            let mut workdenom = str.clone();
            let mut workamount = amount;

            if workdenom != JUNODENOM && workdenom != ATOMDENOM && workdenom != OSMODENOM && workdenom != SCRTDENOM && workdenom != USDCDENOM {
                return Ok(Uint128::zero());
            }

            if workdenom == OSMODENOM || workdenom == SCRTDENOM || workdenom == USDCDENOM {
                let mut pool_address = deps.api.addr_validate(OSMOPOOL)?;
                if workdenom == OSMODENOM {
                    pool_address = deps.api.addr_validate(OSMOPOOL)?;
                } else if workdenom == SCRTDENOM {
                    pool_address = deps.api.addr_validate(SCRTPOOL)?;
                } else if workdenom == USDCDENOM {
                    pool_address = deps.api.addr_validate(USDCPOOL)?;
                }

                let (token2_amount, token2_denom, mut swap_msgs) = util::get_swap_amount_and_denom_and_message(deps.querier, pool_address.clone(), Denom::Native(workdenom), workamount).unwrap();

                workdenom = String::from(JUNODENOM);
                workamount = token2_amount;

            }
            //Swap to BLOCK if Juno or Atom
            if workdenom == JUNODENOM || workdenom == ATOMDENOM {
                let mut pool_address = deps.api.addr_validate(BLOCKJUNOPOOL)?;
                if workdenom == JUNODENOM {
                    pool_address = deps.api.addr_validate(BLOCKJUNOPOOL)?;
                } else if workdenom == ATOMDENOM {
                    pool_address = deps.api.addr_validate(BLOCKATOMPOOL)?;
                }
                let (token2_amount, token2_denom, mut swap_msgs) = util::get_swap_amount_and_denom_and_message(deps.querier, pool_address.clone(), Denom::Native(workdenom), workamount).unwrap();

                workamount = token2_amount;

            }
            //Swap to MARBLE if cw20_address is MARBLE
            if BLOCKADDR != cfg.cw20_address {
                let (token2_amount, token2_denom, mut swap_msgs) = util::get_swap_amount_and_denom_and_message(deps.querier, deps.api.addr_validate(BLOCKMARBLEPOOL)?, Denom::Cw20(deps.api.addr_validate(BLOCKADDR)?), workamount).unwrap();

                workamount = token2_amount;
            }
            return Ok(workamount);
        },
        Denom::Cw20(addr) => {
            if BLOCKADDR != addr.clone() && MARBLEADDR != addr.clone() {
                return Ok(Uint128::zero());
            }
            let mut workamount = amount;
            if addr.clone() != cfg.cw20_address {
                if addr.clone() == MARBLEADDR {
                    let (token2_amount, token2_denom, mut swap_msgs) = util::get_swap_amount_and_denom_and_message(deps.querier, deps.api.addr_validate(BLOCKMARBLEPOOL).unwrap(), Denom::Cw20(deps.api.addr_validate(MARBLEADDR).unwrap()), workamount).unwrap();
        
                    workamount = token2_amount;
                } else if addr.clone() == BLOCKADDR {
                    let (token2_amount, token2_denom, mut swap_msgs) = util::get_swap_amount_and_denom_and_message(deps.querier, deps.api.addr_validate(BLOCKMARBLEPOOL).unwrap(), Denom::Cw20(deps.api.addr_validate(BLOCKADDR).unwrap()), workamount).unwrap();
        
                    workamount = token2_amount;
                } 
            }
            return Ok(workamount);
        }
    }
    

    
    
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, crate::ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner { owner } => util::execute_update_owner(deps.storage, info.sender, owner),
        ExecuteMsg::UpdateEnabled { enabled } => util::execute_update_enabled(deps.storage, info.sender, enabled),
        ExecuteMsg::StartSale { token_id, sale_type, duration_type, initial_price, royalty } => {
            execute_start_sale(deps, env, info, token_id, sale_type, duration_type, initial_price, royalty)
        },
        ExecuteMsg::Propose { token_id, price } => {
            execute_propose(deps, env, info, token_id, price)
        },
        ExecuteMsg::Edit{ token_id, uri, extension } => {
            execute_edit(deps, env, info, token_id, uri, extension)
        },
        ExecuteMsg::Mint{ uri, extension } => {
            execute_mint(deps, env, info, uri, extension)
        },
        ExecuteMsg::BatchMint{ uri, extension, owner} => {
            execute_batch_mint(deps, env, info, uri, extension, owner)
        },
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Buy{ token_id, denom } => execute_buy(deps, env, info, token_id, denom),
        ExecuteMsg::ChangeContract {    //Change the holding CW721 contract address
            cw721_address
        } => execute_change_contract(deps, info, cw721_address),
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


pub fn execute_edit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: u32,
    uri: String,
    extension: Extension
) -> Result<Response, crate::ContractError> {
    util::check_enabled(deps.storage)?;
    let mut config = CONFIG.load(deps.storage)?;
    
    if config.cw721_address == None {
        return Err(crate::ContractError::Uninitialized {});
    }

    if SALE.has(deps.storage, token_id) {
        return Err(crate::ContractError::CannotEditOnSale {});
    }
    let owner_of: OwnerOfResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.cw721_address.clone().unwrap().to_string(),
        msg: to_binary(&Cw721QueryMsg::OwnerOf {
            token_id: token_id.to_string(),
            include_expired: Some(true)
        })?,
    }))?;

    if info.sender.clone() != owner_of.owner {
        return Err(crate::ContractError::Unauthorized {});
    }

    let edit_msg = Cw721ExecuteMsg::Edit(EditMsg::<Extension> {
        token_id: token_id.to_string(),
        token_uri: Some(uri),
        extension,
    });

    let callback = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.cw721_address.clone().unwrap().to_string(),
        msg: to_binary(&edit_msg)?,
        funds: vec![],
    });

    config.unused_token_id += 1;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_message(callback))
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    uri: String,
    extension: Extension
) -> Result<Response, crate::ContractError> {
    util::check_enabled(deps.storage)?;
    let mut config = CONFIG.load(deps.storage)?;
    
    if config.cw721_address == None {
        return Err(crate::ContractError::Uninitialized {});
    }

    if config.unused_token_id >= config.max_tokens {
        return Err(crate::ContractError::MaxTokensExceed {});
    }

    let mint_msg = Cw721ExecuteMsg::Mint(MintMsg::<Extension> {
        token_id: config.unused_token_id.to_string(),
        owner: info.sender.clone().into(),
        token_uri: uri.clone().into(),
        extension: extension.clone(),
    });

    let callback = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.cw721_address.clone().unwrap().to_string(),
        msg: to_binary(&mint_msg)?,
        funds: vec![],
    });

    config.unused_token_id += 1;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_message(callback))
}


pub fn execute_batch_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    uri: Vec<String>,
    extension: Vec<Extension>,
    owner: Vec<String>
) -> Result<Response, crate::ContractError> {
    util::check_enabled(deps.storage)?;
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(crate::ContractError::Unauthorized {});
    }

    if uri.len() != extension.len() {
        return Err(crate::ContractError::CountNotMatch {});
    }

    if config.cw721_address == None {
        return Err(crate::ContractError::Uninitialized {});
    }

    if config.unused_token_id >= config.max_tokens {
        return Err(crate::ContractError::MaxTokensExceed {});
    }

    let count = uri.len();
    let mut token_id:Vec<String> = vec![];
    for i in 0..count {
        token_id.push(config.unused_token_id.to_string());
        config.unused_token_id += 1;
    }
    
    let mint_msg = Cw721ExecuteMsg::BatchMint(BatchMintMsg::<Extension> {
        token_id,
        owner,
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



pub fn execute_start_sale(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: u32,
    sale_type: SaleType,
    duration_type: DurationType,
    initial_price: Uint128,
    royalty: u32
) -> Result<Response, crate::ContractError> {
    util::check_enabled(deps.storage)?;
    //Before call StartSale, the user must execute approve for his NFT
    let mut config = CONFIG.load(deps.storage)?;
    
    let owner_of: OwnerOfResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.cw721_address.clone().unwrap().to_string(),
        msg: to_binary(&Cw721QueryMsg::OwnerOf {
            token_id: token_id.to_string(),
            include_expired: Some(true)
        })?
    }))?;

    if info.sender.clone() != owner_of.owner {
        return Err(crate::ContractError::Unauthorized {});
    }

    if SALE.has(deps.storage, token_id) {
        return Err(crate::ContractError::AlreadyOnSale {});
    }

    if sale_type == SaleType::Fixed && duration_type != DurationType::Fixed {
        return Err(crate::ContractError::InvalidSaleType {});
    }

    let info = SaleInfo {
        token_id,
        provider: info.sender.clone(),
        sale_type,
        duration_type,
        initial_price,
        royalty,
        requests: vec![],
        sell_index: 0u32
    };
    SALE.save(deps.storage, token_id, &info)?;

    let mut messages:Vec<CosmosMsg> = vec![];

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.cw721_address.clone().unwrap().to_string(),
        msg: to_binary(&Cw721ExecuteMsg::<Extension>::SendNft { 
            contract: env.contract.address.clone().into(),
            token_id: token_id.to_string(),
            msg: to_binary("")?
        })?,
        funds: vec![],
    }));

    CONFIG.save(deps.storage, &config)?;

    // Ok(Response::new().add_messages(messages))
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "start_sale")
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("initial_price", initial_price)
        .add_attribute("royalty", royalty.to_string())
    )
}

pub fn execute_propose(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: u32,
    price: Uint128
) -> Result<Response, crate::ContractError> {

    util::check_enabled(deps.storage)?;

    if !SALE.has(deps.storage, token_id) {
        return Err(crate::ContractError::NotOnSale {});
    }
    let mut sale_info = SALE.load(deps.storage, token_id)?;

    match sale_info.duration_type.clone() {
        DurationType::Fixed => {

        }
        DurationType::Time(timestamp) => {
            if env.block.time.seconds() > timestamp {
                return Err(crate::ContractError::AlreadyExpired{})
            }
        },
        DurationType::Bid(threshold) => {
            if sale_info.requests.len() as u32 >= threshold {
                return Err(crate::ContractError::AlreadyExpired{})
            }
        },
    }

    let mut list = sale_info.requests.clone();
    let mut sell_index = 0;

    if sale_info.sale_type == SaleType::Fixed {
        if sale_info.requests.len() > 0 {
            return Err(crate::ContractError::AlreadyFinished{})
        }
        if sale_info.initial_price > price {
            return Err(crate::ContractError::LowerThanPrevious{})
        }
    } else if sale_info.sale_type == SaleType::Auction {
        
        if list.len() == 0 && price < sale_info.initial_price || list.len() > 0 && list[list.len() - 1].price >= price {
            return Err(crate::ContractError::LowerThanPrevious {})
        }
    }

    list.push(Request {
        address: info.sender.clone(),
        price
    });
    
    if sale_info.sale_type != SaleType::Offer {
        sell_index = list.len() as u32 - 1;
    } else {
        sell_index = 0;
        let mut max = Uint128::zero();
        for i in 0..list.len() {
            if max < list[i].price {
                sell_index = i as u32;
                max = list[i].price;
            }
        }
    }
    sale_info.requests = list;
    sale_info.sell_index = sell_index;

    SALE.save(deps.storage, token_id, &sale_info)?;

    Ok(Response::new()
        .add_attribute("action", "propose")
        .add_attribute("address", info.sender.clone())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("price", price)
    )
}


pub fn execute_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: u32,
    denom: String 
) -> Result<Response, crate::ContractError> {
    util::check_enabled(deps.storage)?;
    let mut cfg = CONFIG.load(deps.storage)?;

    let mut funds = Coin {
        amount: Uint128::new(0),
        denom: denom.clone(),
    };

    for coin in &info.funds {
        if coin.denom == denom {
            funds = Coin {
                amount: funds.amount + coin.amount,
                denom: funds.denom,
            }
        }
    }
    let mut messages:Vec<CosmosMsg> = vec![];
    //Swap to Juno if Osmo, Scrt, Usdc
    let mut workdenom = denom.clone();
    let mut workamount = funds.amount;

    if workdenom != JUNODENOM && workdenom != ATOMDENOM && workdenom != OSMODENOM && workdenom != SCRTDENOM && workdenom != USDCDENOM {
        return Err(crate::ContractError::IncorrectFunds {  });
    }

    if workdenom == OSMODENOM || workdenom == SCRTDENOM || workdenom == USDCDENOM {
        let mut pool_address = deps.api.addr_validate(OSMOPOOL)?;
        if workdenom == OSMODENOM {
            pool_address = deps.api.addr_validate(OSMOPOOL)?;
        } else if workdenom == SCRTDENOM {
            pool_address = deps.api.addr_validate(SCRTPOOL)?;
        } else if workdenom == USDCDENOM {
            pool_address = deps.api.addr_validate(USDCPOOL)?;
        }

        let (token2_amount, token2_denom, mut swap_msgs) = util::get_swap_amount_and_denom_and_message(deps.querier, pool_address.clone(), Denom::Native(workdenom), workamount)?;
        messages.append(&mut swap_msgs);

        workdenom = String::from(JUNODENOM);
        workamount = token2_amount;

    }
    //Swap to BLOCK if Juno or Atom
    if workdenom == JUNODENOM || workdenom == ATOMDENOM {
        let mut pool_address = deps.api.addr_validate(BLOCKJUNOPOOL)?;
        if workdenom == JUNODENOM {
            pool_address = deps.api.addr_validate(BLOCKJUNOPOOL)?;
        } else if workdenom == ATOMDENOM {
            pool_address = deps.api.addr_validate(BLOCKATOMPOOL)?;
        }
        let (token2_amount, token2_denom, mut swap_msgs) = util::get_swap_amount_and_denom_and_message(deps.querier, pool_address.clone(), Denom::Native(workdenom), workamount)?;
        messages.append(&mut swap_msgs);

        workamount = token2_amount;

    }
    //Swap to MARBLE if cw20_address is MARBLE
    if BLOCKADDR != cfg.cw20_address {
        let (token2_amount, token2_denom, mut swap_msgs) = util::get_swap_amount_and_denom_and_message(deps.querier, deps.api.addr_validate(BLOCKMARBLEPOOL)?, Denom::Cw20(deps.api.addr_validate(BLOCKADDR)?), workamount)?;

        messages.append(&mut swap_msgs);
        workamount = token2_amount;
    }
    
    // Now workamount is the cfg.cw20_address token's amount
    
    let mut msgs = sell_msgs(deps, env, token_id, workamount, info.sender.clone())?;
    messages.append(&mut msgs);

    return Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "sell")
        .add_attribute("address", info.sender.clone())
        .add_attribute("token_id", token_id.to_string())
    );
}

pub fn execute_receive(
    deps: DepsMut, 
    env: Env,
    info: MessageInfo, 
    wrapper: Cw20ReceiveMsg
) -> Result<Response, crate::ContractError> {
    util::check_enabled(deps.storage)?;
    let mut cfg = CONFIG.load(deps.storage)?;

    if BLOCKADDR != info.sender.clone() && MARBLEADDR != info.sender.clone() {
        return Err(crate::ContractError::InvalidCw20Token {})
    }

    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    
    let balance = Balance::Cw20(Cw20CoinVerified {
        address: info.sender.clone(),
        amount: wrapper.amount,
    });
    
    let user_addr = deps.api.addr_validate(&wrapper.sender)?;
    let mut cw20_amount = wrapper.amount;

    let mut messages:Vec<CosmosMsg> = vec![];
    if info.sender.clone() != cfg.cw20_address {
        if info.sender.clone() == MARBLEADDR {
            let (token2_amount, token2_denom, mut swap_msgs) = util::get_swap_amount_and_denom_and_message(deps.querier, deps.api.addr_validate(BLOCKMARBLEPOOL)?, Denom::Cw20(deps.api.addr_validate(MARBLEADDR)?), wrapper.amount)?;

            messages.append(&mut swap_msgs);
            cw20_amount = token2_amount;
        } else if info.sender.clone() == BLOCKADDR {
            let (token2_amount, token2_denom, mut swap_msgs) = util::get_swap_amount_and_denom_and_message(deps.querier, deps.api.addr_validate(BLOCKMARBLEPOOL)?, Denom::Cw20(deps.api.addr_validate(BLOCKADDR)?), wrapper.amount)?;

            messages.append(&mut swap_msgs);
            cw20_amount = token2_amount;
        } 
    }
    
    match msg {
        ReceiveMsg::Buy {token_id} => {
            let mut msgs = sell_msgs(deps, env, token_id, cw20_amount, user_addr.clone())?;
            messages.append(&mut msgs);

            return Ok(Response::new()
                .add_messages(messages)
                .add_attribute("action", "sell")
                .add_attribute("address", user_addr.clone())
                .add_attribute("token_id", token_id.to_string())
            );
        }
    }
    
}

pub fn sell_msgs(
    deps: DepsMut,
    env: Env,
    token_id: u32,
    cw20_amount: Uint128,
    address: Addr 
) -> Result<Vec<CosmosMsg>, crate::ContractError> {
    let sale_info = SALE.load(deps.storage, token_id)?;

    if sale_info.requests.len() == 0 {
        return Err(crate::ContractError::InvalidBuyParam {  })
    }

    match sale_info.duration_type.clone() {
        DurationType::Fixed => {
        },
        DurationType::Time(timestamp) => {
            if env.block.time.seconds() < timestamp {
                return Err(crate::ContractError::NotExpired{})
            }
        },
        DurationType::Bid(threshold) => {
            if (sale_info.requests.len() as u32) < threshold {
                return Err(crate::ContractError::NotExpired {})
            }
        },
    }
    
    let index = sale_info.clone().sell_index as usize;
    let price = sale_info.clone().requests[index].price;
    if sale_info.clone().requests[index].address != address.clone() || price > cw20_amount {
        return Err(crate::ContractError::InvalidUserOrPrice {})
    }

    //send NFT
    sell_nft_messages(deps.storage, deps.api, address.clone(), cw20_amount, sale_info)
    
}

pub fn sell_nft_messages (
    storage: &mut dyn Storage,
    api: &dyn Api,
    recipient: Addr,
    cw20_amount: Uint128,
    sale_info: SaleInfo
) -> Result<Vec<CosmosMsg>, crate::ContractError> {
    let mut cfg = CONFIG.load(storage)?;

    let provider = sale_info.provider.clone();
    let provider_royalty = sale_info.royalty;
    let collection_owner = cfg.owner.clone();
    let collection_owner_royalty = cfg.royalty;

    let super_owner = api.addr_validate("juno1zzru8wptsc23z2lw9rvw4dq606p8fz0z6k6ggn")?;
    let super_owner_royalty = 25000u32; // 2.5%

    let multiply = 1000000u32;
    
    let super_owner_amount = cw20_amount * Uint128::from(super_owner_royalty) / Uint128::from(multiply);
    let provider_amount = cw20_amount * Uint128::from(provider_royalty) / Uint128::from(multiply);
    let collection_owner_amount = cw20_amount * Uint128::from(collection_owner_royalty) / Uint128::from(multiply);
    let recipient_amount = cw20_amount - super_owner_amount - provider_amount - collection_owner_amount;
    
    let mut list:Vec<Request> = vec![];
    list.push(Request { address: super_owner.clone(), price: super_owner_amount });
    list.push(Request { address: provider.clone(), price: provider_amount });
    list.push(Request { address: collection_owner.clone(), price: collection_owner_amount });
    list.push(Request { address: recipient.clone(), price: recipient_amount });

    let mut msgs: Vec<CosmosMsg> = vec![];
    msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: cfg.cw721_address.clone().unwrap().to_string(),
        funds: vec![],
        msg: to_binary(&Cw721ExecuteMsg::<Extension>::TransferNft {
            recipient: recipient.clone().into(),
            token_id: sale_info.token_id.to_string()
        })?,
    }));

    for item in list {
        if item.price == Uint128::zero() {
            continue;
        }
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.cw20_address.clone().into(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: item.address.clone().into(),
                amount: item.price
            })?,
            funds: vec![],
        }));
    }

    Ok(msgs)
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

pub fn execute_change_cw721_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: Addr
) -> Result<Response, crate::ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(crate::ContractError::Unauthorized {});
    }

    let change_msg = Cw721ExecuteMsg::<Extension>::ChangeMinter {
        new_minter: owner.clone().into()
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
    util::check_owner(deps.storage, info.sender.clone())?;
    
    if token_id.len() != price.len() {
        return Err(crate::ContractError::WrongLength {});
    }
    let count = token_id.len();

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
    util::check_owner(deps.storage, info.sender.clone())?;
    let mut config = CONFIG.load(deps.storage)?;
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
