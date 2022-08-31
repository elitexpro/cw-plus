use std::ops::Index;

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

use cw2::{get_contract_version};
use cw721_base::{
    msg::ExecuteMsg as Cw721ExecuteMsg, Extension
};
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, MigrateMsg, };

use cw20::{ Balance};

use crate::util;

// version info for migration info
const CONTRACT_NAME: &str = "nftsale";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, crate::ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let mut unsold_list:Vec<String> = vec![];
    for i in 0..msg.count {
        unsold_list.push((i + 1).to_string());
    }

    let config = Config {
        owner: info.sender.clone(),
        price: msg.price,
        count: msg.count,
        denom: msg.denom,
        sold_count: 0u32,
        cw721_address: msg.cw721_address,
        enabled: true,
        unsold_list_str: unsold_list.concat()
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::GetSoldState {token_id} => to_binary(&query_get_sold_state(deps, token_id)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner,
        price: config.price,
        count: config.count,
        sold_count: config.sold_count,
        cw721_address: config.cw721_address,
        enabled: config.enabled,
        denom: config.denom,
        unsold_list_str: config.unsold_list_str
    })
}

fn query_get_sold_state(
    deps: Deps,
    token_id: String,
) -> StdResult<bool> {
    let sale_info = SALE.has(deps.storage, token_id);
    Ok(sale_info)
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
        ExecuteMsg::Buy { } => execute_buy(deps, env, info),
        ExecuteMsg::Send { token_id, address } => execute_send(deps, env, info, token_id, address),
    }
}

pub fn execute_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, crate::ContractError> {
    util::check_enabled(deps.storage)?;
    let mut config = CONFIG.load(deps.storage)?;

    
    if config.sold_count == config.count {
        return Err(ContractError::AlreadyFinished {  })
    }

    let mut unsold_list:Vec<&str> = config.unsold_list_str.split(",").collect();
    config.sold_count -= 1;

    let amount = util::get_amount_of_denom(Balance::from(info.funds), Denom::Native(config.denom.clone()))?;
    if amount < config.price {
        return Err(ContractError::InsufficientFund {  })
    }

    let index = env.block.time.seconds() % unsold_list.len() as u64;
    let token_id = String::from(unsold_list[index as usize]);
    
    let mut messages:Vec<CosmosMsg> = vec![];
    messages.push(util::transfer_token_message(Denom::Native(config.denom.clone()), amount, config.owner.clone())?);

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.cw721_address.clone().to_string(),
        msg: to_binary(&Cw721ExecuteMsg::<Extension>::TransferNft {
            token_id: token_id.clone(),
            recipient: info.sender.clone().into()
        })?,
        funds: vec![],
    }));

    unsold_list.remove(index as usize);
    config.unsold_list_str = unsold_list.concat();
    
    CONFIG.save(deps.storage, &config)?;
    SALE.save(deps.storage, token_id.clone(), &info.sender.clone())?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "buy")
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("buyer", info.sender.clone())
    )
}


pub fn execute_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    address: Addr
) -> Result<Response, crate::ContractError> {

    util::check_owner(deps.storage, info.sender.clone())?;
    let mut config = CONFIG.load(deps.storage)?;
    if config.sold_count == config.count {
        return Err(ContractError::AlreadyFinished {  })
    }

    let mut unsold_list:Vec<&str> = config.unsold_list_str.split(",").collect();
    config.sold_count -= 1;

    // let payment = unsold_list
    //     .iter()
    //     .find(|x| x == token_id.as_str());
    let index = unsold_list.iter().find(|x| x == &&token_id.as_str()).ok_or_else(|| ContractError::AlreadySold {  })?;
    unsold_list.remove(index as usize);
    config.unsold_list_str = unsold_list.concat();

    let mut messages:Vec<CosmosMsg> = vec![];
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.cw721_address.clone().to_string(),
        msg: to_binary(&Cw721ExecuteMsg::<Extension>::TransferNft {
            token_id: token_id.clone(),
            recipient: address.clone().into()
        })?,
        funds: vec![],
    }));

    CONFIG.save(deps.storage, &config)?;
    SALE.save(deps.storage, token_id.clone(), &address.clone())?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "send")
        .add_attribute("token_id", token_id)
        .add_attribute("address", address.clone())
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
