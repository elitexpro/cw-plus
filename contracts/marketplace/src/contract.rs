use std::collections::btree_set::Difference;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, from_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
    WasmMsg, WasmQuery, QueryRequest, CosmosMsg, Order, Addr, Decimal, Storage, Api, SubMsg, ReplyOn, Reply
};
use cw_utils::parse_reply_instantiate_data;
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, Cw20QueryMsg, Cw20CoinVerified};
use cw20::{TokenInfoResponse, Balance};
use cw_utils::{maybe_addr};
use cw_storage_plus::Bound;
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, CollectionInfo, CollectionListResponse
};
use crate::state::{
    Config, CONFIG, COLLECTIONS
};

use marble_collection::msg::{InstantiateMsg as CollectionInstantiateMsg, ExecuteMsg as CollectionExecuteMsg, QueryMsg as CollectionQueryMsg, ConfigResponse as CollectionConfigResponse};

// Version info, for migration info
const CONTRACT_NAME: &str = "marble-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = info.sender;

    let config = Config {
        owner,
        max_collection_id: 0u32,
        collection_code_id: msg.collection_code_id,
        cw721_base_code_id: msg.cw721_base_code_id
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { new_owner } => execute_update_config(deps, info, new_owner),
        ExecuteMsg::UpdateConstants { collection_code_id, cw721_base_code_id } => execute_update_constants(deps, info, collection_code_id, cw721_base_code_id),
        // ExecuteMsg::AddCollection {collection_addr, cw721_addr} => execute_add_collection(deps, info, collection_addr, cw721_addr),
        ExecuteMsg::RemoveCollection {id} => execute_remove_collection(deps, info, id),
        ExecuteMsg::RemoveAllCollection {  } => execute_remove_all_collection(deps, info),
        ExecuteMsg::AddCollection(msg) => execute_add_collection(deps, info, msg)
    }
}

pub fn check_owner(
    deps: &DepsMut,
    info: &MessageInfo
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    
    if info.sender != cfg.owner {
        return Err(ContractError::Unauthorized {})
    }
    Ok(Response::new().add_attribute("action", "check_owner"))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Addr,
) -> Result<Response, ContractError> {
    // authorize owner
    check_owner(&deps, &info)?;
    
    CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
        exists.owner = new_owner;
        Ok(exists)
    })?;

    Ok(Response::new().add_attribute("action", "update_config"))
}


pub fn execute_update_constants(
    deps: DepsMut,
    info: MessageInfo,
    collection_code_id: u64,
    cw721_base_code_id: u64
) -> Result<Response, ContractError> {
    // authorize owner
    check_owner(&deps, &info)?;
    
    CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
        exists.collection_code_id = collection_code_id;
        exists.cw721_base_code_id = cw721_base_code_id;
        Ok(exists)
    })?;

    Ok(Response::new().add_attribute("action", "update_constants"))
}
const INSTANTIATE_TOKEN_REPLY_ID: u64 = 2;

pub fn execute_add_collection(
    deps: DepsMut,
    info: MessageInfo,
    msg: CollectionInstantiateMsg
) -> Result<Response, ContractError> {

    // check_owner(&deps, &info)?;

    let cfg = CONFIG.load(deps.storage)?;
    
    let sub_msg: Vec<SubMsg> = vec![SubMsg {
        msg: WasmMsg::Instantiate {
            code_id: cfg.collection_code_id,
            msg: to_binary(&msg)?,
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
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let mut cfg: Config = CONFIG.load(deps.storage)?;

    if msg.id != INSTANTIATE_TOKEN_REPLY_ID {
        return Err(ContractError::InvalidTokenReplyId {});
    }

    let reply = parse_reply_instantiate_data(msg).unwrap();
    let collection_address = Addr::unchecked(reply.contract_address);

    let collection_response: CollectionConfigResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: collection_address.clone().into(),
        msg: to_binary(&CollectionQueryMsg::GetConfig {})?,
    }))?;
    let cw721_address = collection_response.cw721_address.unwrap();

    cfg.max_collection_id += 1;
    CONFIG.save(deps.storage, &cfg)?;

    COLLECTIONS.save(deps.storage, cfg.max_collection_id, &(collection_address.clone(), cw721_address.clone()))?;

    Ok(Response::new()
        .add_attribute("action", "instantiate_collection")
        .add_attribute("collection_address", collection_address)
        .add_attribute("cw721_address", cw721_address)
    )
}

// pub fn execute_add_collection(
//     deps: DepsMut,
//     info: MessageInfo,
//     collection_addr: Addr,
//     cw721_addr: Addr
// ) -> Result<Response, ContractError> {

//     check_owner(&deps, &info)?;

//     let mut cfg = CONFIG.load(deps.storage)?;
//     cfg.max_collection_id += 1;
//     CONFIG.save(deps.storage, &cfg);

//     COLLECTIONS.save(deps.storage, cfg.max_collection_id, &(collection_addr.clone(), cw721_addr.clone()))?;
//     Ok(Response::new()
//         .add_attribute("action", "add_collection")
//         .add_attribute("collection_addr", collection_addr)
//         .add_attribute("cw721_addr", cw721_addr)
//     )
// }

pub fn execute_remove_collection(
    deps: DepsMut,
    info: MessageInfo,
    id: u32
) -> Result<Response, ContractError>{
    check_owner(&deps, &info)?;
    COLLECTIONS.remove(deps.storage, id);
    Ok(Response::new()
        .add_attribute("action", "remove_collection")
       
    )
}

pub fn execute_remove_all_collection(
    deps: DepsMut,
    info: MessageInfo
) -> Result<Response, ContractError> {
    // authorize owner
    check_owner(&deps, &info)?;

    let collections:StdResult<Vec<_>> = COLLECTIONS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| map_collection(item))
        .collect();

    if collections.is_err() {
        return Err(ContractError::Map2ListFailed {})
    }
    
    for item in collections.unwrap() {
        COLLECTIONS.remove(deps.storage, item.id);
    }
    
    Ok(Response::new().add_attribute("action", "remove_all_collection"))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} 
            => to_binary(&query_config(deps)?),
        QueryMsg::Collection {id} 
            => to_binary(&query_collection(deps, id)?),
        QueryMsg::ListCollections {} 
            => to_binary(&query_list_collections(deps)?)
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: cfg.owner,
        max_collection_id: cfg.max_collection_id,
        collection_code_id: cfg.collection_code_id,
        cw721_base_code_id: cfg.cw721_base_code_id
    })
    
}

pub fn query_collection(deps: Deps, id: u32) -> StdResult<CollectionInfo> {
    let exists = COLLECTIONS.may_load(deps.storage, id)?;
    let cfg = CONFIG.load(deps.storage)?;
    let (mut collection_addr, mut cw721_addr) = (cfg.owner.clone(), cfg.owner.clone());
    if exists.is_some() {
        (collection_addr, cw721_addr) = exists.unwrap();
    } 
    Ok(CollectionInfo {
        id,
        collection_addr,
        cw721_addr
    })
}

pub fn query_list_collections(deps: Deps) 
-> StdResult<CollectionListResponse> {
    let collections:StdResult<Vec<_>> = COLLECTIONS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| map_collection(item))
        .collect();

    Ok(CollectionListResponse {
        list: collections?
    })
}

fn map_collection(
    item: StdResult<(u32, (Addr, Addr))>,
) -> StdResult<CollectionInfo> {
    item.map(|(id, (collection_addr, cw721_addr))| {
        CollectionInfo {
            id,
            collection_addr,
            cw721_addr
        }
    })
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
