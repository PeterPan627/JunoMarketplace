use cosmwasm_std::{
    entry_point, to_binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,from_binary,Binary,
    StdResult, Uint128,CosmosMsg,WasmMsg,Decimal,BankMsg,Order
};

use cw2::set_contract_version;
use cw20::{ Cw20ExecuteMsg,Cw20ReceiveMsg};
use cw721::{Cw721ReceiveMsg, Cw721ExecuteMsg};

use crate::error::{ContractError};
use crate::msg::{ ExecuteMsg, InstantiateMsg, QueryMsg,SellNft, BuyNft};
use crate::state::{State,CONFIG,Offering, OFFERINGS,Asset,UserInfo, MEMBERS,SALEHISTORY,PRICEINFO,SaleInfo,PriceInfo, COLLECTIONINFO, CollectionInfo, TOKENADDRESS, TVL, TvlInfo};
use crate::package::{QueryOfferingsResult};


const CONTRACT_NAME: &str = "Hope_Market_Place";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let state = State {
        owner:msg.owner,
        new:true,
    };
    CONFIG.save(deps.storage,&state)?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
    ExecuteMsg::ReceiveNft(msg) =>execute_receive_nft(deps,env,info,msg),
    ExecuteMsg::Receive(msg) =>execute_receive(deps,env,info,msg),
    ExecuteMsg::BuyNft { offering_id,nft_address } =>execute_buy_nft(deps,env,info,offering_id,nft_address),
    ExecuteMsg::WithdrawNft { offering_id,nft_address } => execute_withdraw(deps,env,info,offering_id,nft_address),
    ExecuteMsg::AddTokenAddress { symbol, address }  => execute_token_address(deps,env,info,symbol,address),
    ExecuteMsg::ChangeOwner { address } =>execute_change_owner(deps,env,info,address),
    ExecuteMsg::AddCollection { royalty_portion, members,nft_address ,offering_id} =>execute_add_collection(deps,env,info,royalty_portion,members,nft_address,offering_id),
    ExecuteMsg::UpdateCollection { royalty_portion, members,nft_address } =>execute_update_collection(deps,env,info,royalty_portion,members,nft_address),
    ExecuteMsg:: FixNft{address,token_id} =>execute_fix_nft(deps,env,info,address,token_id),
    ExecuteMsg::SetOfferings { address, offering }=>execute_set_offerings(deps,env,info,address,offering),
    ExecuteMsg::SetTvl { address, tvl } =>execute_set_tvl(deps,env,info,address,tvl),
    ExecuteMsg::Migrate { address, dest, token_id }=>execute_migrate(deps,env,info,address,dest,token_id)
}
}


fn execute_receive_nft(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    rcv_msg: Cw721ReceiveMsg,
)-> Result<Response, ContractError> {

    let collection_info = COLLECTIONINFO.may_load(deps.storage, &info.sender.to_string())?;

    if collection_info == None{
        return Err(ContractError::WrongNFTContractError { });
    }

    let mut collection_info = collection_info.unwrap();
    

    let msg:SellNft = from_binary(&rcv_msg.msg)?;
    let nft_address = info.sender.to_string();
    
    collection_info.offering_id = collection_info.offering_id + 1;
   
    COLLECTIONINFO.save(deps.storage, &nft_address,&collection_info)?;

    let off = Offering {
        token_id: rcv_msg.token_id.clone(),
        seller: deps.api.addr_validate(&rcv_msg.sender)?.to_string(),
        list_price: msg.list_price.clone(),
    };

    
  
    OFFERINGS.save(deps.storage, (&nft_address,&collection_info.offering_id.to_string()), &off)?;
    Ok(Response::default())
}

fn execute_receive(
    deps: DepsMut,
    env:Env,
    info: MessageInfo,
    rcv_msg: Cw20ReceiveMsg,
)-> Result<Response, ContractError> {
    let _state = CONFIG.load(deps.storage)?;

    let token_symbol = TOKENADDRESS.may_load(deps.storage, &info.sender.to_string())?;

    if token_symbol == None{
        return Err(ContractError::WrongTokenContractError {  })
    }
    let token_symbol = token_symbol.unwrap();

    let msg:BuyNft = from_binary(&rcv_msg.msg)?;
    deps.api.addr_validate(&msg.nft_address)?;

    let collection_info = COLLECTIONINFO.may_load(deps.storage, &msg.nft_address)?;
    if collection_info == None{
        return Err(ContractError::WrongNFTContractError {  })
    }

    let off = OFFERINGS.load(deps.storage, (&msg.nft_address,&msg.offering_id))?;

    
    if off.list_price.denom != token_symbol{
        return Err(ContractError::NotEnoughFunds  { })
    }

    if off.list_price.amount != rcv_msg.amount{
        return Err(ContractError::NotEnoughFunds  { })
    }

    let tvl = TVL.may_load(deps.storage, (&msg.nft_address,&off.list_price.denom))?;
    let crr_tvl:Uint128;
    if tvl == None {
        crr_tvl = off.list_price.amount;
    }
    else {
        crr_tvl = tvl.unwrap()+off.list_price.amount;
    }

    TVL.save(deps.storage,( &msg.nft_address,&off.list_price.denom), &crr_tvl)?;
  
    let members = MEMBERS.load(deps.storage,&msg.nft_address)?;
    let collection_info = COLLECTIONINFO.may_load(deps.storage, &msg.nft_address)?;

    if collection_info == None{
        return Err(ContractError::WrongNFTContractError {  })
    }

    let collection_info = collection_info.unwrap();

    if collection_info.offering_id == 1{    
          OFFERINGS.remove( deps.storage, (&msg.nft_address,&msg.offering_id));
          COLLECTIONINFO.update(deps.storage, &msg.nft_address,
             |collection_info|->StdResult<_>{
                    let mut collection_info = collection_info.unwrap();
                    collection_info.offering_id = 0;
                    Ok(collection_info)
             })?;
    }

    else{
        let crr_offering_id = collection_info.offering_id;
        let offering = OFFERINGS.may_load(deps.storage, (&msg.nft_address,&crr_offering_id.to_string()))?;
        
        if offering!=None{
            OFFERINGS.save(deps.storage, (&msg.nft_address,&msg.offering_id.to_string()), &offering.unwrap())?;
           
            COLLECTIONINFO.update(deps.storage, &msg.nft_address,
                |collection_info|->StdResult<_>{
                        let mut collection_info = collection_info.unwrap();
                        collection_info.offering_id =collection_info.offering_id-1;
                        Ok(collection_info)
                })?;
          OFFERINGS.remove( deps.storage, (&msg.nft_address,&crr_offering_id.to_string()));
         }

         
    }

    let mut messages:Vec<CosmosMsg> = vec![];
    for user in members{
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: info.sender.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                    recipient: user.address.clone(), 
                    amount: rcv_msg.amount*collection_info.royalty_portion*user.portion })?,
        }))
    }

    let price_info = PRICEINFO.may_load(deps.storage,&msg.nft_address)?;
    if price_info == None{
        PRICEINFO.save(deps.storage,&msg.nft_address,&PriceInfo {
            total_juno:Uint128::new(0) ,
            total_hope: rcv_msg.amount })?;
       }
    else{
        PRICEINFO.update(deps.storage,&msg.nft_address,
        |price_info|->StdResult<_>{
            let mut price_info = price_info.unwrap();
            price_info.total_hope = price_info.total_hope + rcv_msg.amount;
            Ok(price_info)
        })?;}
    let sale_id = collection_info.sale_id+1;

    SALEHISTORY.save(deps.storage, (&msg.nft_address,&sale_id.to_string()),&SaleInfo { 
        from:off.seller.clone(),
        to: rcv_msg.sender.to_string(), 
        denom: off.list_price.denom,
        amount: rcv_msg.amount, 
        time: env.block.time.seconds(),
        nft_address:msg.nft_address.clone(),
        token_id:off.token_id.clone()
    })?;

    COLLECTIONINFO.update(deps.storage, &msg.nft_address, 
        |collection_info|->StdResult<_>{
            let mut collection_info = collection_info.unwrap();
            collection_info.sale_id = sale_id;
            Ok(collection_info)
        }
    )?;
    

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: msg.nft_address.to_string(),
                funds: vec![],
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: deps.api.addr_validate(&rcv_msg.sender)?.to_string(),
                    token_id: off.token_id.clone(),
            })?,
        }))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: info.sender.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                    recipient: off.seller, 
                    amount: rcv_msg.amount*(Decimal::one()-collection_info.royalty_portion) })?,
        }))
        .add_messages(messages)
)
}

fn execute_buy_nft(
    deps: DepsMut,
    env:Env,
    info: MessageInfo,
    offering_id: String,
    nft_address:String
) -> Result<Response, ContractError> {
  
    let collection_info = COLLECTIONINFO.may_load(deps.storage, &nft_address)?;
    if collection_info == None{
        return Err(ContractError::WrongNFTContractError {  })
    }
    let collection_info = collection_info.unwrap();
    let off = OFFERINGS.load(deps.storage, (&nft_address, &offering_id))?;

    let amount= info
        .funds
        .iter()
        .find(|c| c.denom == off.list_price.denom)
        .map(|c| Uint128::from(c.amount))
        .unwrap_or_else(Uint128::zero);

    if off.list_price.amount!=amount{
        return Err(ContractError::NotEnoughFunds {  })
    }
    if collection_info.offering_id == 1{    
          OFFERINGS.remove( deps.storage, (&nft_address,&offering_id));
          COLLECTIONINFO.update(deps.storage, &nft_address,
             |collection_info|->StdResult<_>{
                    let mut collection_info = collection_info.unwrap();
                    collection_info.offering_id = 0;
                    Ok(collection_info)
             })?;
    }

    else{
        let crr_offering_id = collection_info.offering_id;
        let offering = OFFERINGS.may_load(deps.storage, (&nft_address,&crr_offering_id.to_string()))?;
        if offering!=None{
            OFFERINGS.save(deps.storage, (&nft_address,&offering_id.to_string()), &offering.unwrap())?;
           
            COLLECTIONINFO.update(deps.storage, &nft_address,
                |collection_info|->StdResult<_>{
                        let mut collection_info = collection_info.unwrap();
                        collection_info.offering_id =collection_info.offering_id-1;
                        Ok(collection_info)
                })?;
        OFFERINGS.remove( deps.storage, (&nft_address,&crr_offering_id.to_string()));
  
         }
    }
    
    let members = MEMBERS.load(deps.storage,&nft_address)?;
    
    let mut messages:Vec<CosmosMsg> = vec![];
    for user in members{
        messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: user.address,
                amount:vec![Coin{
                    denom:off.clone().list_price.denom,
                    amount:amount*collection_info.royalty_portion*user.portion
                }]
        }))
    }
   
    let price_info = PRICEINFO.may_load(deps.storage,&nft_address)?;
   if price_info == None{
        PRICEINFO.save(deps.storage,&nft_address,&PriceInfo {
            total_hope:Uint128::new(0) ,
            total_juno: amount })?;
       }
    else{
        PRICEINFO.update(deps.storage,&nft_address,
        |price_info|->StdResult<_>{
            let mut price_info = price_info.unwrap();
            price_info.total_juno = price_info.total_juno + amount;
            Ok(price_info)
        })?;}
    
     let sale_id = collection_info.sale_id+1;

    SALEHISTORY.save(deps.storage, (&nft_address,&sale_id.to_string()),&SaleInfo {
         from:off.seller.clone(), 
         to: info.sender.to_string(), 
         denom: off.list_price.denom.clone(),
         amount: amount, 
         time: env.block.time.seconds(),
         nft_address:nft_address.clone(),
         token_id:off.token_id.clone()         
        })?;

    COLLECTIONINFO.update(deps.storage, &nft_address, 
        |collection_info|->StdResult<_>{
            let mut collection_info = collection_info.unwrap();
            collection_info.sale_id = sale_id;
            Ok(collection_info)
        }
    )?;

    
    let tvl = TVL.may_load(deps.storage, (&nft_address,&off.list_price.denom))?;
    let crr_tvl:Uint128;
    if tvl == None {
        crr_tvl = off.list_price.amount;
    }
    else {
        crr_tvl = tvl.unwrap()+off.list_price.amount;
    }

    TVL.save(deps.storage,( &nft_address,&off.list_price.denom), &crr_tvl)?;

  
    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: nft_address.to_string(),
                funds: vec![],
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: info.sender.to_string(),
                    token_id: off.token_id.clone(),
            })?,
        }))
        .add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: off.seller,
                amount:vec![Coin{
                    denom:off.list_price.denom,
                    amount:amount*(Decimal::one()-collection_info.royalty_portion)
                }]
        }))
        .add_messages(messages)
)
}

fn execute_withdraw(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    offering_id: String,
    nft_address:String
) -> Result<Response, ContractError> {
    let off = OFFERINGS.load(deps.storage,(&nft_address,&offering_id))?;
   // let state = CONFIG.load(deps.storage)?;

    let collection_info = COLLECTIONINFO.may_load(deps.storage, &nft_address)?;
    if collection_info == None{
        return Err(ContractError::WrongNFTContractError {  })
    }
   let collection_info = collection_info.unwrap();

   if collection_info.offering_id == 1{    
          OFFERINGS.remove( deps.storage, (&nft_address,&offering_id));
          COLLECTIONINFO.update(deps.storage, &nft_address,
             |collection_info|->StdResult<_>{
                    let mut collection_info = collection_info.unwrap();
                    collection_info.offering_id = 0;
                    Ok(collection_info)
             })?;
    }

    else{
        let crr_offering_id = collection_info.offering_id;
        let offering = OFFERINGS.may_load(deps.storage, (&nft_address,&crr_offering_id.to_string()))?;
        if offering!=None{
            OFFERINGS.save(deps.storage, (&nft_address,&offering_id.to_string()), &offering.unwrap())?;
           
            COLLECTIONINFO.update(deps.storage, &nft_address,
                |collection_info|->StdResult<_>{
                        let mut collection_info = collection_info.unwrap();
                        collection_info.offering_id =collection_info.offering_id-1;
                        Ok(collection_info)
                })?;
        OFFERINGS.remove( deps.storage, (&nft_address,&crr_offering_id.to_string()));
  
         }
    }

    if off.seller == info.sender.to_string(){

        Ok(Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: nft_address.to_string(),
                funds: vec![],
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: deps.api.addr_validate(&off.seller)?.to_string(),
                    token_id: off.token_id.clone(),
            })?,
        }))
    )
    }
    else {
        return Err(ContractError::Unauthorized {});
    }
    
}


fn execute_add_collection(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    royalty_potion: Decimal,
    members: Vec<UserInfo>,
    nft_address:String,
    offering_id:u64
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    deps.api.addr_validate(&nft_address)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    
    let mut sum_portion = Decimal::zero();

    for item in members.clone() {
        sum_portion = sum_portion + item.portion;
        deps.api.addr_validate(&item.address)?;
    }

    if sum_portion != Decimal::one(){
        return Err(ContractError::WrongPortionError { })
    }

    MEMBERS.save(deps.storage,&nft_address, &members)?;
    COLLECTIONINFO.save(deps.storage,&nft_address,&CollectionInfo{
        nft_address:nft_address.clone(),
        offering_id:offering_id,
        sale_id:0,
        royalty_portion:royalty_potion
    })?;
    Ok(Response::default())
}


fn execute_update_collection(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    royalty_potion: Decimal,
    members: Vec<UserInfo>,
    nft_address:String
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    deps.api.addr_validate(&nft_address)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }

    let collection_info = COLLECTIONINFO.may_load(deps.storage,&nft_address)?;
    if collection_info == None{
        return Err(ContractError::WrongCollection {  })
    }
    let collection_info = collection_info.unwrap();

    let mut sum_portion = Decimal::zero();

    for item in members.clone() {
        sum_portion = sum_portion + item.portion;
        deps.api.addr_validate(&item.address)?;
    }

    if sum_portion != Decimal::one(){
        return Err(ContractError::WrongPortionError { })
    }

    MEMBERS.save(deps.storage,&nft_address, &members)?;
    COLLECTIONINFO.save(deps.storage,&nft_address,&CollectionInfo{
        nft_address:nft_address.clone(),
        offering_id:collection_info.offering_id,
        royalty_portion:royalty_potion,
        sale_id:collection_info.sale_id
    })?;
    Ok(Response::default())
}


fn execute_token_address(
    deps: DepsMut,
    _env:Env,
    _info: MessageInfo,
    symbol:String,
    address: String,
) -> Result<Response, ContractError> {
    let  state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
    
    TOKENADDRESS.save(deps.storage,&address,&symbol)?;

    CONFIG.save(deps.storage, &state)?;
    Ok(Response::default())
}


fn execute_fix_nft(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
    token_id:String
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: address,
                funds: vec![],
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: info.sender.to_string(),
                    token_id: token_id.clone(),
            })?,
        })))
}


fn execute_migrate(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
    dest:String,
    token_ids:Vec<String>
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
     deps.api.addr_validate(&dest)?;

    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    let mut messages:Vec<CosmosMsg> = vec![];

    for token_id in token_ids{
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: address.clone(),
                funds: vec![],
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: dest.clone(),
                    token_id: token_id.clone(),
            })?,
        }))
    }

    Ok(Response::new()
        .add_messages(messages))
}


fn execute_change_owner(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut state = CONFIG.load(deps.storage)?;

    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }
    deps.api.addr_validate(&address)?;
    state.owner = address;
    CONFIG.save(deps.storage,&state)?;
    Ok(Response::default())
}

fn execute_set_tvl(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
    tvls: Vec<TvlInfo>,
) -> Result<Response, ContractError> {
    let  state = CONFIG.load(deps.storage)?;

    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }
   
    for tvl in tvls{
        TVL.save(deps.storage, (&address,&tvl.denom), &tvl.amount)?;
    }

    Ok(Response::default())
}

fn execute_set_offerings(
    deps: DepsMut,
    _env:Env,
    info:MessageInfo,
    address: String,
    offerings:Vec<QueryOfferingsResult>
) -> Result<Response, ContractError> {
    let  state = CONFIG.load(deps.storage)?;

    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }
    
    for offering in offerings{
        let crr_offering = Offering{
            token_id:offering.token_id,
            seller:offering.seller,
            list_price:offering.list_price
        };
        OFFERINGS.save(deps.storage, (&address,&offering.id), &crr_offering)?;
    }
   
    Ok(Response::default())
}



#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetStateInfo {} => to_binary(&query_state_info(deps)?),
        QueryMsg::GetMembers {address} => to_binary(&query_get_members(deps,address)?),
        QueryMsg::GetTradingInfo { address} => to_binary(&query_get_trading(deps,address)?),
        QueryMsg::GetSaleHistory {address, id } => to_binary(&query_get_history(deps,address,id)?),
        QueryMsg::GetCollectionInfo { address } =>to_binary(&query_collection_info(deps,address)?),
        QueryMsg::GetOfferingId {address }=> to_binary(&query_get_ids(deps,address)?),
        QueryMsg::GetOfferingPage { id,address }  => to_binary(&query_get_offering(deps,id,address)?),
        QueryMsg::GetTvl { address, symbol }=> to_binary(&query_get_tvl(deps,address,symbol)?),
        QueryMsg::GetTvlAll { address, symbols }=> to_binary(&query_all_tvl(deps,address,symbols)?)
    }
}

pub fn query_state_info(deps:Deps) -> StdResult<State>{
    let state =  CONFIG.load(deps.storage)?;
    Ok(state)
}

pub fn query_collection_info(deps:Deps,address:String) -> StdResult<CollectionInfo>{
    let collection_info =  COLLECTIONINFO.load(deps.storage,&address)?;
    Ok(collection_info)
}


pub fn query_get_tvl(deps:Deps,address:String,symbol:String) -> StdResult<Uint128>{
    let tvl = TVL.may_load(deps.storage, (&address,&symbol))?;
    if tvl == None{
        Ok(Uint128::new(0))
    }
    else{
        Ok(tvl.unwrap())
    }
}

pub fn query_all_tvl(deps:Deps,address:String,symbols:Vec<String>) -> StdResult<Vec<TvlInfo>>{
    let mut empty:Vec<TvlInfo> = vec![];
    for symbol in symbols
    {
        let tvl = TVL.may_load(deps.storage, (&address,&symbol))?;
        if tvl == None{
            empty.push(TvlInfo { denom: symbol, amount: Uint128::new(0) })
        }
        else{
              empty.push(TvlInfo { denom: symbol, amount: tvl.unwrap() })
         
        }
    }
    Ok(empty)
}


pub fn query_get_members(deps:Deps,address:String) -> StdResult<Vec<UserInfo>>{
    let members = MEMBERS.load(deps.storage,&address)?;
    Ok(members)
}

pub fn query_get_trading(deps:Deps,address:String) -> StdResult<PriceInfo>{
    let price_info = PRICEINFO.may_load(deps.storage,&address)?;
    if price_info ==None{
        Ok(PriceInfo{
            total_hope:Uint128::new(0),
            total_juno:Uint128::new(0)
        })
    }
    else{
    Ok(price_info.unwrap())}
}

// pub fn query_get_offerings(deps:Deps) -> StdResult<OfferingsResponse>{
//     let res: StdResult<Vec<QueryOfferingsResult>> = OFFERINGS
//         .range(deps.storage, None, None, Order::Ascending)
//         .map(|kv_item| parse_offering(deps, kv_item  ))
//         .collect();
//     Ok(OfferingsResponse {
//         offerings: res?, // Placeholder
//     })
// }

// fn parse_offering(
//     deps:Deps,
//     item: StdResult<((String,String),Offering)>,
// ) -> StdResult<QueryOfferingsResult> {
//     item.and_then(|((address,k), offering)| {
//         Ok(QueryOfferingsResult {
//             id: k,
//             token_id: offering.token_id,
//             list_price: offering.list_price,
//             seller: deps.api.addr_validate(&offering.seller)?.to_string(),
//         })
//     })
// }


pub fn query_get_ids(deps:Deps,address: String) -> StdResult<Vec<String>>{
     let token_id:StdResult<Vec<String>>  = OFFERINGS
        .keys(deps.storage, None, None, Order::Ascending)
        .filter(|keys|keys.as_ref().unwrap().0 == address.clone())
        .map(|keys|parse_keys(deps, keys))
        .collect();
    Ok(token_id?)
}

fn parse_keys(
    _deps:Deps,
    item: StdResult<(String,String)>,
) -> StdResult<String> {
    item.and_then(|(_address,token_id)| {
        Ok(token_id)
    })
}


pub fn query_get_offering(deps:Deps,ids:Vec<String>,address: String) -> StdResult<Vec<QueryOfferingsResult>>{
    let mut offering_group:Vec<QueryOfferingsResult> = vec![];
    for id in ids{
        let offering = OFFERINGS.may_load(deps.storage,(&address,&id))?;
        if offering!=None{
            let offering = offering.unwrap();
            offering_group.push(QueryOfferingsResult{
                id:id,
                token_id:offering.token_id,
                list_price:offering.list_price,
                seller:offering.seller
            });
        }
    }
    Ok(offering_group)
}

pub fn query_get_history(deps:Deps,address:String, ids:Vec<String>) -> StdResult<Vec<SaleInfo>>{
    let mut sale_history : Vec<SaleInfo> = vec![];
    for id in ids{
       let history =  SALEHISTORY.may_load(deps.storage, (&address,&id))?;
       if history != None{
        sale_history.push(history.unwrap());
       }
    }
    Ok(sale_history)
}

#[cfg(test)]
mod tests {
  
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{ CosmosMsg, Coin};

    #[test]
    fn testing() {
        //Instantiate
        let mut deps = mock_dependencies();
        let instantiate_msg = InstantiateMsg {
           owner:"creator".to_string()
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());
        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.owner,"creator".to_string());
       

        //Change Owner

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::ChangeOwner { address:"owner".to_string()};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.owner,"owner".to_string());

         //Change Token Contract Address

        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::AddTokenAddress  { address:"token_address".to_string(),symbol:"hope".to_string()};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        
        let info = mock_info("owner", &[]);
         let msg = ExecuteMsg::AddTokenAddress  { address:"raw_address".to_string(),symbol:"raw".to_string()};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
       
        //Hope1 Collection Add
       let info = mock_info("owner", &[]);
       let msg = ExecuteMsg::AddCollection {
            royalty_portion: Decimal::from_ratio(5 as u128, 100 as u128), 
            members: vec![UserInfo{
                address:"admin1".to_string(),
                portion:Decimal::from_ratio(3 as u128, 10 as u128)
                },UserInfo{
                address:"admin2".to_string(),
                portion:Decimal::from_ratio(7 as u128, 10 as u128)
                }] ,
            nft_address: "hope1_address".to_string() ,
            offering_id:0
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
       

       // Sell nft
        let cw721_msg = SellNft{
            list_price:Asset{
                denom:"ujuno".to_string(),
                amount:Uint128::new(1000000)
            }
        };

        let info = mock_info("hope1_address", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"owner1".to_string(),
            token_id:"Hope.1".to_string(),
            msg:to_binary(&cw721_msg).unwrap()
        });
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();


        let collection_info = query_collection_info(deps.as_ref(),
                "hope1_address".to_string()).unwrap();
        assert_eq!(collection_info,CollectionInfo{
            nft_address:"hope1_address".to_string(),
            offering_id:1,
            royalty_portion:Decimal::from_ratio(5 as u128, 100 as u128),
            sale_id:0
            });

      
        let offerings = query_get_offering(deps.as_ref(),vec!["1".to_string(),"2".to_string()],"hope1_address".to_string()).unwrap();
        assert_eq!(offerings,vec![QueryOfferingsResult{
            id:"1".to_string(),
            token_id:"Hope.1".to_string(),
            list_price:Asset { denom: "ujuno".to_string(), amount: Uint128::new(1000000) },
            seller:"owner1".to_string()
        }]);

            //Buy nft

      let info = mock_info("test_buyer1", &[Coin{
        denom:"ujuno".to_string(),
        amount:Uint128::new(1000000)
      }]);
      let msg = ExecuteMsg::BuyNft { offering_id: "1".to_string(), nft_address: "hope1_address".to_string() };
      let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
      assert_eq!(res.messages.len(),4);

       let collection_info = query_collection_info(deps.as_ref(),"hope1_address".to_string()).unwrap();
       assert_eq!(collection_info.offering_id,0); 

      assert_eq!(res.messages[0].msg,CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "hope1_address".to_string(),
                funds: vec![],
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: "test_buyer1".to_string(),
                    token_id:"Hope.1".to_string(),
            }).unwrap(),
        }));

      assert_eq!(res.messages[1].msg,CosmosMsg::Bank(BankMsg::Send {
                to_address: "owner1".to_string(),
                amount:vec![Coin{
                    denom:"ujuno".to_string(),
                    amount:Uint128::new(950000)
                }]
        }));

        assert_eq!(res.messages[2].msg,CosmosMsg::Bank(BankMsg::Send {
                to_address: "admin1".to_string(),
                amount:vec![Coin{
                    denom:"ujuno".to_string(),
                    amount:Uint128::new(15000)
                }]
        }));
        assert_eq!(res.messages[3].msg,CosmosMsg::Bank(BankMsg::Send {
                to_address: "admin2".to_string(),
                amount:vec![Coin{
                    denom:"ujuno".to_string(),
                    amount:Uint128::new(35000)
                }]
        }));
                                                                    
        let ids =  query_get_ids(deps.as_ref(),"hope1_address".to_string()).unwrap();
        let test_id:Vec<String> = vec![];
        assert_eq!(ids,test_id);
        

         // Rearragnge the offering

         //sell
        let cw721_msg = SellNft{
            list_price:Asset{
                denom:"osmos".to_string(),
                amount:Uint128::new(2000000)
            }
        };

        let info = mock_info("hope1_address", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"buyer1".to_string(),
            token_id:"Hope.1".to_string(),
            msg:to_binary(&cw721_msg).unwrap()
        });
         execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
     
         //sell
         let cw721_msg = SellNft{
            list_price:Asset{
                denom:"ujuno".to_string(),
                amount:Uint128::new(2000000)
            }
        };

          let info = mock_info("hope1_address", &[]);
          let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"buyer2".to_string(),
            token_id:"Hope.2".to_string(),
            msg:to_binary(&cw721_msg).unwrap()
            });
         execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

         //buy

        let info = mock_info("test_buyer2", &[Coin{
            denom:"osmos".to_string(),
            amount:Uint128::new(2000000)
        }]);
        let msg = ExecuteMsg::BuyNft { offering_id: "1".to_string(), nft_address: "hope1_address".to_string() };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let id = query_get_ids(deps.as_ref(), "hope1_address".to_string()).unwrap();
        let collection_info = query_collection_info(deps.as_ref(),"hope1_address".to_string()).unwrap();
        assert_eq!(collection_info.offering_id,1);  
        assert_eq!(id,vec!["1"]);

         let cw721_msg = SellNft{
            list_price:Asset{
                denom:"hope".to_string(),
                amount:Uint128::new(2000000)
            }
        };

        let info = mock_info("hope1_address", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"buyer3".to_string(),
            token_id:"Hope.3".to_string(),
            msg:to_binary(&cw721_msg).unwrap()
        });

        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let ids = query_get_ids(deps.as_ref(),"hope1_address".to_string()).unwrap();
        assert_eq!(ids,vec!["1".to_string(),"2".to_string()]);
        let offerings = query_get_offering(deps.as_ref(),vec!["1".to_string(),"2".to_string()],"hope1_address".to_string()).unwrap();
        assert_eq!(offerings,vec![QueryOfferingsResult{
            id:"1".to_string(),
            token_id:"Hope.2".to_string(),
            list_price:Asset { denom: "ujuno".to_string(),  amount:Uint128::new(2000000) },
            seller:"buyer2".to_string()
        },QueryOfferingsResult{
            id:"2".to_string(),
            token_id:"Hope.3".to_string(),
            list_price:Asset { denom: "hope".to_string(),  amount:Uint128::new(2000000) },
            seller:"buyer3".to_string()
        }]);

        let cw20_msg= BuyNft{
            offering_id:"2".to_string(),
            nft_address:"hope1_address".to_string()
        };

        let info = mock_info("token_address", &[]);
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg{
            sender:"test_buyer3".to_string(),
            amount:Uint128::new(2000000),
            msg:to_binary(&cw20_msg).unwrap()
        });
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();    
        assert_eq!(res.messages[1].msg,CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "token_address".to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                    recipient: "buyer3".to_string(), 
                    amount:Uint128::new(1900000),
                 }).unwrap()
         }));

         assert_eq!(res.messages[2].msg,CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "token_address".to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                    recipient: "admin1".to_string(), 
                    amount:Uint128::new(30000),
                 }).unwrap()
         }));

          assert_eq!(res.messages[3].msg,CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "token_address".to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                    recipient: "admin2".to_string(), 
                    amount:Uint128::new(70000),
                 }).unwrap()
         }));


        let offerings = query_get_offering(deps.as_ref(),vec!["1".to_string(),"2".to_string()],"hope1_address".to_string()).unwrap();
        assert_eq!(offerings,vec![QueryOfferingsResult{
            id:"1".to_string(),
            token_id:"Hope.2".to_string(),
            list_price:Asset { denom: "ujuno".to_string(),  amount:Uint128::new(2000000) },
            seller:"buyer2".to_string()
        }]);

        let cw721_msg = SellNft{
            list_price:Asset{
                denom:"raw".to_string(),
                amount:Uint128::new(2000000)
            }
        };

        let info = mock_info("hope1_address", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"owner1".to_string(),
            token_id:"Hope.1".to_string(),
            msg:to_binary(&cw721_msg).unwrap()
        });
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let cw20_msg= BuyNft{
            offering_id:"2".to_string(),
            nft_address:"hope1_address".to_string()
        };

        let info = mock_info("raw_address", &[]);
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg{
            sender:"test_buyer4".to_string(),
            amount:Uint128::new(2000000),
            msg:to_binary(&cw20_msg).unwrap()
        });
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        assert_eq!(offerings,vec![QueryOfferingsResult{
            id:"1".to_string(),
            token_id:"Hope.2".to_string(),
            list_price:Asset { denom: "ujuno".to_string(),  amount:Uint128::new(2000000) },
            seller:"buyer2".to_string()
        }]);

        let juno_tvl = query_get_tvl(deps.as_ref(),"hope1_address".to_string(),"ujuno".to_string()).unwrap();
        let hope_tvl = query_get_tvl(deps.as_ref(),"hope1_address".to_string(),"hope".to_string()).unwrap();
        let osmos_tvl = query_get_tvl(deps.as_ref(),"hope1_address".to_string(),"osmos".to_string()).unwrap();
        let raw_tvl  = query_get_tvl(deps.as_ref(),"hope1_address".to_string(),"raw".to_string()).unwrap();
        println!("{}","juno".to_string());
        assert_eq!(juno_tvl,Uint128::new(1000000));
        
        println!("{}","hope".to_string());
        assert_eq!(hope_tvl,Uint128::new(2000000));

        println!("{}","osmos".to_string());
        assert_eq!(osmos_tvl,Uint128::new(2000000));

         println!("{}","raw".to_string());
        assert_eq!(raw_tvl,Uint128::new(2000000));

        let collection_info = query_collection_info(deps.as_ref(), "hope1_address".to_string()).unwrap();
        assert_eq!(collection_info.sale_id,4);
        let _sale_history = query_get_history(deps.as_ref(), "hope1_address".to_string(), vec!["1".to_string(),"2".to_string(),"3".to_string(),"4".to_string()]).unwrap();
        
        let tvl_all = query_all_tvl(deps.as_ref(), "hope1_address".to_string(), vec!["ujuno".to_string(),"hope".to_string(),"osmos".to_string(),"xyz".to_string()]).unwrap();
        assert_eq!(tvl_all,vec![TvlInfo{
            denom:"ujuno".to_string(),
            amount:Uint128::new(1000000)
        },TvlInfo{
            denom:"hope".to_string(),
            amount:Uint128::new(2000000)
        },TvlInfo{
            denom:"osmos".to_string(),
            amount:Uint128::new(2000000)
        },TvlInfo{
            denom:"xyz".to_string(),
            amount:Uint128::new(0)
        }]);

         let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetTvl { address: "hope1_address".to_string(), tvl: vec![TvlInfo{
            denom:"ujuno".to_string(),
            amount:Uint128::new(0)
        }] };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let tvl_all = query_all_tvl(deps.as_ref(), "hope1_address".to_string(), vec!["ujuno".to_string()]).unwrap();
        assert_eq!(tvl_all,vec![TvlInfo{
            denom:"ujuno".to_string(),
            amount:Uint128::new(0)
        }]);
       
    }
}
    