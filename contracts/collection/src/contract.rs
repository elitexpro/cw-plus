#[cfg(not(feature = "library"))]
use crate::ContractError;
use crate::state::{Config, CONFIG, SALE};
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, Api,
    StdResult, SubMsg, Uint128, WasmMsg, Coin, from_binary, BankMsg, QueryRequest, WasmQuery, Storage
};
use cw2::set_contract_version;
use cw721::{
    OwnerOfResponse,
    
};
use cw2::{get_contract_version};
use cw_storage_plus::Bound;
use cw721_base::{
    msg::ExecuteMsg as Cw721ExecuteMsg, msg::InstantiateMsg as Cw721InstantiateMsg, Extension, 
    msg::MintMsg, msg::BatchMintMsg, msg::QueryMsg as Cw721QueryMsg,  msg::EditMsg
};
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, MerkleRootResponse, IsClaimedResponse, PriceListResponse, PriceInfo, MigrateMsg, SaleType, DurationType, SaleInfo, Request};
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
    // for i in 0..count {
    //     ret.push(PriceInfo {
    //         token_id: token_id[i],
    //         price: PRICE.load(deps.storage, token_id[i])?
    //     });
    // }

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
    token_id: String,
    uri: String,
    extension: Extension
) -> Result<Response, crate::ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    
    if config.cw721_address == None {
        return Err(crate::ContractError::Uninitialized {});
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
        token_id,
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



pub fn execute_receive(
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

    let user_addr = deps.api.addr_validate(&wrapper.sender)?;
    let cw20_amount = wrapper.amount;

    match msg {
        ReceiveMsg::Buy {token_id} => {
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
            if sale_info.clone().requests[index].address != user_addr.clone() || price > cw20_amount {
                return Err(crate::ContractError::InvalidUserOrPrice {})
            }

            //send NFT
            let msgs: Vec<CosmosMsg> = sell_nft_messages(deps.storage, deps.api, user_addr.clone(), cw20_amount, sale_info)?;
            return Ok(Response::new()
                .add_messages(msgs)
                .add_attribute("action", "sell")
                .add_attribute("address", info.sender.clone())
                .add_attribute("token_id", token_id.to_string())
                .add_attribute("price", price)
            )
        }
    }
    
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
