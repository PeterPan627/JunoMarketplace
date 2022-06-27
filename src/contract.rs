use cosmwasm_std::{
    entry_point, to_binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,from_binary,Binary,
    StdResult, Uint128,CosmosMsg,WasmMsg,Decimal,BankMsg,Order,Pair
};

use cw2::set_contract_version;
use cw20::{ Cw20ExecuteMsg,Cw20ReceiveMsg};
use cw721::{Cw721ReceiveMsg, Cw721ExecuteMsg};

use crate::error::{ContractError};
use crate::msg::{ ExecuteMsg, InstantiateMsg, QueryMsg,SellNft, BuyNft};
use crate::state::{State,CONFIG,Offering, OFFERINGS,Asset,UserInfo, MEMBERS,SALEHISTORY,PRICEINFO,SaleInfo,PriceInfo, COLLECTIONINFO, CollectionInfo};
use crate::package::{OfferingsResponse,QueryOfferingsResult};


const CONTRACT_NAME: &str = "Hope_Market_Place";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let state = State {
        owner:msg.owner,
        new:true,
        token_address:msg.token_address
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
    ExecuteMsg::SetTokenAddress {address} => execute_token_address(deps,env,info,address),
    ExecuteMsg::ChangeOwner { address } =>execute_change_owner(deps,env,info,address),
    ExecuteMsg::AddCollection { royalty_portion, members,nft_address } =>execute_add_collection(deps,env,info,royalty_portion,members,nft_address),
    ExecuteMsg::UpdateCollection { royalty_portion, members,nft_address } =>execute_update_collection(deps,env,info,royalty_portion,members,nft_address),
    ExecuteMsg:: FixNft{address,token_id} =>execute_fix_nft(deps,env,info,address,token_id)
}
}


fn execute_receive_nft(
    deps: DepsMut,
    env:Env,
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

    let token_id = rcv_msg.token_id.clone();
    let token_history = SALEHISTORY.may_load(deps.storage,(&nft_address,&token_id))?;
 
    if token_history == None{
        SALEHISTORY.save(deps.storage,(&nft_address,&token_id) , &vec![SaleInfo{
            address:rcv_msg.sender,
            denom : "Hope".to_string(),
            amount:Uint128::new(0),
            time:env.block.time.seconds()
        }] )?;
    }

    OFFERINGS.save(deps.storage, (&nft_address,&collection_info.offering_id.to_string()), &off)?;
    Ok(Response::default())
}

fn execute_receive(
    deps: DepsMut,
    env:Env,
    info: MessageInfo,
    rcv_msg: Cw20ReceiveMsg,
)-> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.token_address{
        return Err(ContractError::WrongTokenContractError  { })
    }

    let msg:BuyNft = from_binary(&rcv_msg.msg)?;
    deps.api.addr_validate(&msg.nft_address)?;

    let collection_info = COLLECTIONINFO.may_load(deps.storage, &msg.nft_address)?;
    if collection_info == None{
        return Err(ContractError::WrongNFTContractError {  })
    }

    let off = OFFERINGS.load(deps.storage, (&msg.nft_address,&msg.offering_id))?;

    
    if off.list_price.denom != "hope".to_string(){
        return Err(ContractError::NotEnoughFunds  { })
    }

    if off.list_price.amount != rcv_msg.amount{
        return Err(ContractError::NotEnoughFunds  { })
    }

    OFFERINGS.remove( deps.storage, (&msg.nft_address,&msg.offering_id));
    let members = MEMBERS.load(deps.storage,&msg.nft_address)?;
    let collection_info = COLLECTIONINFO.may_load(deps.storage, &msg.nft_address)?;

    if collection_info == None{
        return Err(ContractError::WrongNFTContractError {  })
    }

    let collection_info = collection_info.unwrap();
    
    let mut messages:Vec<CosmosMsg> = vec![];
    for user in members{
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.token_address.to_string(),
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
   
    SALEHISTORY.update(deps.storage, (&msg.nft_address,&off.token_id.clone()),
     | token_history|->StdResult<_>{
        let mut token_history = token_history.unwrap();
        token_history.push(SaleInfo { address: rcv_msg.sender.to_string(), 
        denom: "Hope".to_string(),
         amount: rcv_msg.amount/Uint128::new(1000000), 
         time: env.block.time.seconds() });
        Ok(token_history)
     })?;
    
    OFFERINGS.remove(deps.storage,(&msg.nft_address,&msg.offering_id) );

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
                contract_addr: state.token_address.to_string(),
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

    OFFERINGS.remove( deps.storage,(&nft_address, &offering_id));
    
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
    
    SALEHISTORY.update(deps.storage, (&nft_address, &off.token_id.clone()),
     | token_history|->StdResult<_>{
        let mut token_history = token_history.unwrap();
        token_history.push(SaleInfo {
         address: info.sender.to_string(), 
         denom: "Juno".to_string(),
         amount: amount/Uint128::new(1000000), 
         time: env.block.time.seconds() });
        Ok(token_history)
     })?;

    OFFERINGS.remove(deps.storage,(&nft_address,&offering_id) );

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
    let state = CONFIG.load(deps.storage)?;

    let collection_info = COLLECTIONINFO.may_load(deps.storage, &nft_address)?;
    if collection_info == None{
        return Err(ContractError::WrongNFTContractError {  })
    }
    let collection_info = collection_info.unwrap();


    OFFERINGS.remove(deps.storage,(&nft_address,&offering_id) );

    if off.seller == info.sender.to_string(){
        OFFERINGS.remove(deps.storage,(&nft_address,&offering_id) );
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
    nft_address:String
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
        offering_id:0,
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
        royalty_portion:royalty_potion
    })?;
    Ok(Response::default())
}


fn execute_token_address(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
    
    state.token_address = address;

    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

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



#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetStateInfo {} => to_binary(&query_state_info(deps)?),
        QueryMsg::GetMembers {address} => to_binary(&query_get_members(deps,address)?),
        QueryMsg::GetTradingInfo { address} => to_binary(&query_get_trading(deps,address)?),
        QueryMsg::GetSaleHistory {address, token_id } => to_binary(&query_get_history(deps,address,token_id)?),
        QueryMsg::GetCollectionInfo { address } =>to_binary(&query_collection_info(deps,address)?),
        QueryMsg::GetOfferingId { }=> to_binary(&query_get_ids(deps)?),
        QueryMsg::GetOfferingPage { id,address }  => to_binary(&query_get_offering(deps,id,address)?),
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


pub fn query_get_ids(deps:Deps) -> StdResult<Vec<String>>{
     let token_id:StdResult<Vec<String>>  = OFFERINGS
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|keys|parse_keys(deps, keys))
        .collect();
    Ok(token_id?)
}

fn parse_keys(
    deps:Deps,
    item: StdResult<(String,String)>,
) -> StdResult<String> {
    item.and_then(|(address,token_id)| {
        Ok(token_id)
    })
}


pub fn query_get_offering(deps:Deps,ids:Vec<String>,address: String) -> StdResult<Vec<QueryOfferingsResult>>{
    let mut offering_group:Vec<QueryOfferingsResult> = vec![];
    for id in ids{
        let offering = OFFERINGS.load(deps.storage,(&address,&id))?;
        offering_group.push(QueryOfferingsResult{
            id:id,
            token_id:offering.token_id,
            list_price:offering.list_price,
            seller:offering.seller
        });
    }
    Ok(offering_group)
}

pub fn query_get_history(deps:Deps,address:String, token_id:String) -> StdResult<Vec<SaleInfo>>{
    let history = SALEHISTORY.load(deps.storage,(&address,&token_id))?;
    Ok(history)
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
           owner:"creator".to_string(),
           token_address:"token".to_string()
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
        let msg = ExecuteMsg::SetTokenAddress  { address:"token_address".to_string()};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.token_address,"token_address".to_string());

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
            nft_address: "hope1_address".to_string() 
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

     let sale_history = query_get_history(
        deps.as_ref(),
        "hope1_address".to_string(),
        "Hope.1".to_string()
     ).unwrap();

     assert_eq!(sale_history,vec![SaleInfo{
        address:"owner1".to_string(),
        denom:"Hope".to_string(),
        amount:Uint128::new(0),
        time:mock_env().block.time.seconds()
     }]);

     let collection_info = query_collection_info(deps.as_ref(),
      "hope1_address".to_string()).unwrap();
     assert_eq!(collection_info,CollectionInfo{
        nft_address:"hope1_address".to_string(),
        offering_id:1,
        royalty_portion:Decimal::from_ratio(5 as u128, 100 as u128)
     });

     let ids =  query_get_ids(deps.as_ref()).unwrap();
     assert_eq!(ids,vec!["1".to_string()]);

     let offerings = query_get_offering(deps.as_ref(),vec!["1".to_string()],"hope1_address".to_string()).unwrap();
     assert_eq!(offerings,vec![QueryOfferingsResult{
        id:"1".to_string(),
        token_id:"Hope.1".to_string(),
        list_price:Asset { denom: "ujuno".to_string(), amount: Uint128::new(1000000) },
        seller:"owner1".to_string()
     }]);

     //Buy nft

      let info = mock_info("buyer1", &[Coin{
        denom:"ujuno".to_string(),
        amount:Uint128::new(1000000)
      }]);
      let msg = ExecuteMsg::BuyNft { offering_id: "1".to_string(), nft_address: "hope1_address".to_string() };
      let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
      assert_eq!(res.messages.len(),4);
      assert_eq!(res.messages[0].msg,CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "hope1_address".to_string(),
                funds: vec![],
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: "buyer1".to_string(),
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

        let ids =  query_get_ids(deps.as_ref()).unwrap();
        let test_id:Vec<String> = vec![];
        assert_eq!(ids,test_id);
        

         // Sale History and TVL check
        let cw721_msg = SellNft{
            list_price:Asset{
                denom:"ujuno".to_string(),
                amount:Uint128::new(2000000)
            }
        };

        let info = mock_info("hope1_address", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"buyer2".to_string(),
            token_id:"Hope.1".to_string(),
            msg:to_binary(&cw721_msg).unwrap()
        });
      execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

      let info = mock_info("buyer2", &[Coin{
        denom:"ujuno".to_string(),
        amount:Uint128::new(2000000)
      }]);
      let msg = ExecuteMsg::BuyNft { offering_id: "2".to_string(), nft_address: "hope1_address".to_string() };
      execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

      let sale_history = query_get_history(deps.as_ref(),"hope1_address".to_string(), "Hope.1".to_string()).unwrap();
      let trade_info = query_get_trading(deps.as_ref(),"hope1_address".to_string()).unwrap();

      assert_eq!(sale_history,vec![SaleInfo{
        address:"owner1".to_string(),
        denom:"Hope".to_string(),
        amount:Uint128::new(0),
        time:mock_env().block.time.seconds()
     },SaleInfo{
        address:"buyer1".to_string(),
        denom:"Juno".to_string(),
        amount:Uint128::new(1),
        time:mock_env().block.time.seconds()
     },SaleInfo{
        address:"buyer2".to_string(),
        denom:"Juno".to_string(),
        amount:Uint128::new(2),
        time:mock_env().block.time.seconds()
     },
     ]);
     assert_eq!(trade_info,PriceInfo{
        total_hope:Uint128::new(0),
        total_juno:Uint128::new(3000000)
     });
     
     //Bye Nft with hope token

    let cw721_msg = SellNft{
        list_price:Asset{
            denom:"hope".to_string(),
            amount:Uint128::new(1000000)
        }
    };

    let info = mock_info("hope1_address", &[]);
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
        sender:"owner".to_string(),
        token_id:"Hope.2".to_string(),
        msg:to_binary(&cw721_msg).unwrap()
    });
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let cw20_msg = BuyNft{
            offering_id:"3".to_string(),
            nft_address:"hope1_address".to_string()
    };

    let info = mock_info("token_address", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg{
        sender:"buyer1".to_string(),
        amount:Uint128::new(1000000),
        msg:to_binary(&cw20_msg).unwrap()
    });
     execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

     
     let cw721_msg = SellNft{
        list_price:Asset{
            denom:"hope".to_string(),
            amount:Uint128::new(2000000)
        }
    };

    let info = mock_info("hope1_address", &[]);
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
        sender:"buyer1".to_string(),
        token_id:"Hope.2".to_string(),
        msg:to_binary(&cw721_msg).unwrap()
    });
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let cw20_msg = BuyNft{
            offering_id:"4".to_string(),
            nft_address:"hope1_address".to_string()
    };

    let info = mock_info("token_address", &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg{
        sender:"buyer2".to_string(),
        amount:Uint128::new(2000000),
        msg:to_binary(&cw20_msg).unwrap()
    });
    let res =  execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(res.messages.len(),4);

    let sale_history = query_get_history(deps.as_ref(),"hope1_address".to_string(), "Hope.2".to_string()).unwrap();
      let trade_info = query_get_trading(deps.as_ref(),"hope1_address".to_string()).unwrap();

      assert_eq!(sale_history,vec![SaleInfo{
        address:"owner".to_string(),
        denom:"Hope".to_string(),
        amount:Uint128::new(0),
        time:mock_env().block.time.seconds()
     },SaleInfo{
        address:"buyer1".to_string(),
        denom:"Hope".to_string(),
        amount:Uint128::new(1),
        time:mock_env().block.time.seconds()
     },SaleInfo{
        address:"buyer2".to_string(),
        denom:"Hope".to_string(),
        amount:Uint128::new(2),
        time:mock_env().block.time.seconds()
     },
     ]);
     assert_eq!(trade_info,PriceInfo{
        total_hope:Uint128::new(3000000),
        total_juno:Uint128::new(3000000)
     });

     assert_eq!(res.messages[0].msg, CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "hope1_address".to_string(),
            funds: vec![],
            msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: "buyer2".to_string(),
                    token_id: "Hope.2".to_string(),
            }).unwrap(),
        }));
        assert_eq!(res.messages[1].msg, CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token_address".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "buyer1".to_string(),
                    amount:Uint128::new(1900000)
            }).unwrap(),
        }));

        assert_eq!(res.messages[2].msg.clone(), CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token_address".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "admin1".to_string(),
                    amount:Uint128::new(30000)
            }).unwrap(),
        }));


        assert_eq!(res.messages[3].msg, CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token_address".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "admin2".to_string(),
                    amount:Uint128::new(70000)
            }).unwrap(),
        }));

    //testing withdraw
    let cw721_msg = SellNft{
        list_price:Asset{
            denom:"hope".to_string(),
            amount:Uint128::new(2000000)
        }
    };

    let info = mock_info("hope1_address", &[]);
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
        sender:"owner".to_string(),
        token_id:"Hope.3".to_string(),
        msg:to_binary(&cw721_msg).unwrap()
    });
    execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::WithdrawNft { offering_id: "5".to_string(),nft_address:"hope1_address".to_string() };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(1,res.messages.len());
    assert_eq!(res.messages[0].msg, CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "hope1_address".to_string(),
        funds: vec![],
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: "owner".to_string(),
                token_id: "Hope.3".to_string(),
        }).unwrap(),
    }));      

    let ids =  query_get_ids(deps.as_ref()).unwrap();
    let test_id:Vec<String> = vec![];
    assert_eq!(ids,test_id);

    //Second Collection Add
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
            nft_address: "hope2_address".to_string() 
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let trade_info = query_get_trading(deps.as_ref(), "hope2_address".to_string()).unwrap();
     let collection_info = query_collection_info(deps.as_ref(),
      "hope2_address".to_string()).unwrap();
     assert_eq!(collection_info,CollectionInfo{
        nft_address:"hope2_address".to_string(),
        offering_id:0,
        royalty_portion:Decimal::from_ratio(5 as u128, 100 as u128)
     });
     assert_eq!(trade_info,PriceInfo{
        total_juno:Uint128::new(0),
        total_hope:Uint128::new(0)
     })

    }
}
    