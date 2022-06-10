use cosmwasm_std::{
    entry_point, to_binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,from_binary,Binary,
    StdResult, Uint128,CosmosMsg,WasmMsg,Decimal,BankMsg,Order,Pair
};

use cw2::set_contract_version;
use cw20::{ Cw20ExecuteMsg,Cw20ReceiveMsg};
use cw721::{Cw721ReceiveMsg, Cw721ExecuteMsg};

use crate::error::{ContractError};
use crate::msg::{ ExecuteMsg, InstantiateMsg, QueryMsg,SellNft, BuyNft};
use crate::state::{State,CONFIG,Offering, OFFERINGS,Asset,UserInfo, MEMBERS,SALEHISTORY,PRICEINFO,SaleInfo,PriceInfo};
use crate::package::{OfferingsResponse,QueryOfferingsResult};
use std::rc;
use std::str::from_utf8;

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
        owner:info.sender.to_string(),
        token_address:String::from("token_address"),
        nft_address :String::from("nft_address"),
        offering_id:0,
        royalty_portion:msg.royalty_portion
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
    ExecuteMsg::BuyNft { offering_id } =>execute_buy_nft(deps,env,info,offering_id),
    ExecuteMsg::SetAdminsList { members } => execute_set_members(deps,env,info,members),
    ExecuteMsg::ChangeRoyaltyPortion { royalty_portion } => execute_change_royalty(deps,env,info,royalty_portion),
    ExecuteMsg::WithdrawNft { offering_id } => execute_withdraw(deps,env,info,offering_id),
    ExecuteMsg::SetTokenAddress {address} => execute_token_address(deps,env,info,address),
    ExecuteMsg::SetNftAddress { address } =>execute_nft_address(deps,env,info,address),
    ExecuteMsg::ChangeOwner { address } =>execute_change_owner(deps,env,info,address),
    }
}


fn execute_receive_nft(
     deps: DepsMut,
    env:Env,
    info: MessageInfo,
    rcv_msg: Cw721ReceiveMsg,
)-> Result<Response, ContractError> {
    
    let mut state = CONFIG.load(deps.storage)?;
    
    if info.sender.to_string()!=state.nft_address{
        return Err(ContractError::WrongNFTContractError { });
    }

    let msg:SellNft = from_binary(&rcv_msg.msg)?;
    
    state.offering_id = state.offering_id + 1;
    CONFIG.save(deps.storage, &state)?;

    let off = Offering {
        token_id: rcv_msg.token_id.clone(),
        seller: deps.api.addr_validate(&rcv_msg.sender)?.to_string(),
        list_price: msg.list_price.clone(),
    };

    let token_history = SALEHISTORY.may_load(deps.storage,&info.sender.to_string())?;
    let token_id = rcv_msg.token_id.clone();
    if token_history == None{
        SALEHISTORY.save(deps.storage,&token_id , &vec![SaleInfo{
            address:rcv_msg.sender,
            denom : "owner".to_string(),
            amount:Uint128::new(0),
            time:env.block.time.seconds()
        }] )?;
    }

    OFFERINGS.save(deps.storage, &state.offering_id.to_string(), &off)?;
    let price_string = format!("{} ", msg.list_price.amount);

    Ok(Response::new()
        .add_attribute("price_string", price_string)
    )
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
    let off = OFFERINGS.load(deps.storage, &msg.offering_id)?;

    
    if off.list_price.denom != "hope".to_string(){
        return Err(ContractError::NotEnoughFunds  { })
    }

    if off.list_price.amount > rcv_msg.amount{
        return Err(ContractError::NotEnoughFunds  { })
    }

    OFFERINGS.remove( deps.storage, &msg.offering_id);
    let members = MEMBERS.load(deps.storage)?;
    
    let mut messages:Vec<CosmosMsg> = vec![];
    for user in members{
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.token_address.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                    recipient: user.address.clone(), 
                    amount: rcv_msg.amount*state.royalty_portion*user.portion })?,
        }))
    }

    let price_info = PRICEINFO.may_load(deps.storage)?;
    if price_info == None{
        PRICEINFO.save(deps.storage,&PriceInfo {
            min_juno: Uint128::new(100000000000),
            max_juno: Uint128::new(0),
            min_hope:rcv_msg.amount, 
            max_hope: rcv_msg.amount,
            total_juno:Uint128::new(0) ,
            total_hope: rcv_msg.amount })?;
       }
    else{
         let price_info = price_info.unwrap();
         if price_info.max_hope == Uint128::new(0){
              PRICEINFO.update(deps.storage,
                   |mut price|->StdResult<_>{
                       price.max_hope = rcv_msg.amount;
                       price.min_hope = rcv_msg.amount;               
                       price.total_hope = rcv_msg.amount;
                      Ok(price)
             }
        )?;}
        else{
            if price_info.max_hope<rcv_msg.amount{
            PRICEINFO.update(deps.storage,
                |mut price|->StdResult<_>{
                    price.max_hope = rcv_msg.amount;
                    price.total_hope = price.total_hope + rcv_msg.amount;
                    Ok(price)
                }
            )?;}
            else 
            { if rcv_msg.amount<price_info.min_hope{
                PRICEINFO.update(deps.storage,
                    |mut price|->StdResult<_>{
                        price.min_hope = rcv_msg.amount;
                        price.total_hope = price.total_hope + rcv_msg.amount;
                        Ok(price)
                    }
                )?;}
                else{
                    PRICEINFO.update(deps.storage,
                        |mut price|->StdResult<_>{
                            price.total_hope = price.total_hope + rcv_msg.amount;
                            Ok(price)
                        }
                    )?;}
                }}
}
   
    SALEHISTORY.update(deps.storage, &off.token_id.clone(),
     | token_history|->StdResult<_>{
        let mut token_history = token_history.unwrap();
        token_history.push(SaleInfo { address: rcv_msg.sender.to_string(), 
        denom: "Hope".to_string(),
         amount: rcv_msg.amount, 
         time: env.block.time.seconds() });
        Ok(token_history)
     })?;
    
    OFFERINGS.remove(deps.storage,&msg.offering_id );

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.nft_address.to_string(),
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
                    amount: rcv_msg.amount*(Decimal::one()-state.royalty_portion) })?,
        }))
        .add_messages(messages)
)
}

fn execute_buy_nft(
    deps: DepsMut,
    env:Env,
    info: MessageInfo,
    offering_id: String,
) -> Result<Response, ContractError> {
     let state = CONFIG.load(deps.storage)?;

   
    let off = OFFERINGS.load(deps.storage, &offering_id)?;

    let amount= info
        .funds
        .iter()
        .find(|c| c.denom == off.list_price.denom)
        .map(|c| Uint128::from(c.amount))
        .unwrap_or_else(Uint128::zero);

    if off.list_price.amount>amount{
        return Err(ContractError::NotEnoughFunds {  })
    }

    OFFERINGS.remove( deps.storage, &offering_id);
    
    let members = MEMBERS.load(deps.storage)?;
    
    let mut messages:Vec<CosmosMsg> = vec![];
    for user in members{
        messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: user.address,
                amount:vec![Coin{
                    denom:off.clone().list_price.denom,
                    amount:amount*state.royalty_portion*user.portion
                }]
        }))
    }
    let price_info = PRICEINFO.may_load(deps.storage)?;
    if price_info == None{
        PRICEINFO.save(deps.storage,&PriceInfo {
            min_juno:amount,
            max_juno:amount,
            min_hope:Uint128::new(100000000000), 
            max_hope: Uint128::new(0),
            total_juno:Uint128::new(0) ,
            total_hope: amount })?;
       }
    else{
         let price_info = price_info.unwrap();
         if price_info.max_juno == Uint128::new(0){
              PRICEINFO.update(deps.storage,
                   |mut price|->StdResult<_>{
                       price.max_juno = amount;
                       price.min_juno = amount;               
                       price.total_juno = amount;
                      Ok(price)})?;
             }
        else{
            if price_info.max_juno<amount{
                PRICEINFO.update(deps.storage,
                    |mut price|->StdResult<_>{
                        price.max_juno = amount;
                        price.total_juno = price.total_juno + amount;
                        Ok(price)
                    }
                )?;}
            else 
                { if amount<price_info.min_juno{
                    PRICEINFO.update(deps.storage,
                        |mut price|->StdResult<_>{
                            price.min_juno = amount;
                            price.total_juno = price.total_juno + amount;
                            Ok(price)
                        }
                    )?;}
                    else{
                        PRICEINFO.update(deps.storage,
                            |mut price|->StdResult<_>{
                                price.total_juno = price.total_juno + amount;
                                Ok(price)
                            }
                        )?;}
                    }}
            }
        SALEHISTORY.update(deps.storage, &off.token_id.clone(),
     | token_history|->StdResult<_>{
        let mut token_history = token_history.unwrap();
        token_history.push(SaleInfo { address: info.sender.to_string(), 
        denom: "Juno".to_string(),
         amount: amount, 
         time: env.block.time.seconds() });
        Ok(token_history)
     })?;

    OFFERINGS.remove(deps.storage,&offering_id );

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.nft_address.to_string(),
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
                    amount:amount*(Decimal::one()-state.royalty_portion)
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
) -> Result<Response, ContractError> {
    let off = OFFERINGS.load(deps.storage,&offering_id)?;
    let state = CONFIG.load(deps.storage)?;

    OFFERINGS.remove(deps.storage,&offering_id );

    if off.seller == info.sender.to_string(){
        OFFERINGS.remove(deps.storage,&offering_id);
        Ok(Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.nft_address.to_string(),
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

fn execute_set_members(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    members: Vec<UserInfo>,
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

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

    MEMBERS.save(deps.storage, &members)?;
    Ok(Response::default())
}

fn execute_change_royalty(
     deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    royalty_potion: Decimal,
)->Result<Response,ContractError>{
    let mut state = CONFIG.load(deps.storage)?;    
    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }

    state.royalty_portion = royalty_potion;
    CONFIG.save(deps.storage, &state)?;
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

fn execute_nft_address(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
    state.nft_address = address;
    
    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    CONFIG.save(deps.storage, &state)?;
    Ok(Response::default())
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
        QueryMsg::GetOfferings {} => to_binary(&query_get_offerings(deps)?),
        QueryMsg::GetMembers {} => to_binary(&query_get_members(deps)?),
        QueryMsg::GetOfferingId { }=> to_binary(&query_get_ids(deps)?),
        QueryMsg::GetSaleHistory { token_id } => to_binary(&query_get_history(deps,token_id)?),
        QueryMsg::GetOfferingPage { id }  => to_binary(&query_get_offering(deps,id)?),
        QueryMsg::GetTradingInfo { } => to_binary(&query_get_trading(deps)?)
    }
}

pub fn query_state_info(deps:Deps) -> StdResult<State>{
    let state =  CONFIG.load(deps.storage)?;
    Ok(state)
}

pub fn query_get_members(deps:Deps) -> StdResult<Vec<UserInfo>>{
    let members = MEMBERS.load(deps.storage)?;
    Ok(members)
}

pub fn query_get_trading(deps:Deps) -> StdResult<PriceInfo>{
    let price_info = PRICEINFO.load(deps.storage)?;
    Ok(price_info)
}

pub fn query_get_offerings(deps:Deps) -> StdResult<OfferingsResponse>{
    let res: StdResult<Vec<QueryOfferingsResult>> = OFFERINGS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv_item| parse_offering(deps, kv_item  ))
        .collect();
    Ok(OfferingsResponse {
        offerings: res?, // Placeholder
    })
}

pub fn query_get_offering(deps:Deps,ids:Vec<String>) -> StdResult<Vec<QueryOfferingsResult>>{
    let mut offering_group:Vec<QueryOfferingsResult> = vec![];
    for id in ids{
        let offering = OFFERINGS.load(deps.storage,&id)?;
        offering_group.push(QueryOfferingsResult{
            id:id,
            token_id:offering.token_id,
            list_price:offering.list_price,
            seller:offering.seller
        });
    }
    Ok(offering_group)
}

fn parse_offering(
    deps:Deps,
    item: StdResult<(String,Offering)>,
) -> StdResult<QueryOfferingsResult> {
    item.and_then(|(k, offering)| {
        Ok(QueryOfferingsResult {
            id: k,
            token_id: offering.token_id,
            list_price: offering.list_price,
            seller: deps.api.addr_validate(&offering.seller)?.to_string(),
        })
    })
}

pub fn query_get_ids(deps:Deps) -> StdResult<Vec<String>>{
     let token_id:StdResult<Vec<String>>  = OFFERINGS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(token_id?)
}

pub fn query_get_history(deps:Deps,token_id:String) -> StdResult<Vec<SaleInfo>>{
    let history = SALEHISTORY.load(deps.storage,&token_id)?;
    Ok(history)
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{ CosmosMsg, Coin};

    #[test]
    fn testing() {
        //Instantiate
        let mut deps = mock_dependencies();
        let instantiate_msg = InstantiateMsg {
            royalty_portion:Decimal::from_ratio(2 as u128, 100 as u128)
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());
        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.owner,"creator".to_string());
        assert_eq!(state.royalty_portion,Decimal::from_ratio(2 as u128, 100 as u128));

        //Change Owner

        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::ChangeOwner { address:"owner".to_string()};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.owner,"owner".to_string());

        //Set Members

        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetAdminsList { members: vec![UserInfo{
            address:"admin1".to_string(),
            portion:Decimal::from_ratio(3 as u128, 10 as u128)
        },UserInfo{
            address:"admin2".to_string(),
            portion:Decimal::from_ratio(7 as u128, 10 as u128)
        }] }; 
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let members = query_get_members(deps.as_ref()).unwrap();
        assert_eq!(members, vec![UserInfo{
            address:"admin1".to_string(),
            portion:Decimal::from_ratio(3 as u128, 10 as u128)
        },UserInfo{
            address:"admin2".to_string(),
            portion:Decimal::from_ratio(7 as u128, 10 as u128)
        }]);
        let nft_market_datas = query_get_offerings(deps.as_ref()).unwrap();
        assert_eq!(nft_market_datas.offerings,
            vec![]
        );
        //Chage Portion

        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::ChangeRoyaltyPortion { royalty_portion: Decimal::from_ratio(3 as u128, 100 as u128) };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.royalty_portion,Decimal::from_ratio(3 as u128, 100 as u128));
        
        //Change Token Contract Address

        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetTokenAddress  { address:"token_address1".to_string()};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.token_address,"token_address1".to_string());

        //Change NFT contract Address

        let info = mock_info("owner", &[]);
        let msg = ExecuteMsg::SetNftAddress  { address:"nft_address1".to_string()};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let state = query_state_info(deps.as_ref()).unwrap();
        assert_eq!(state.nft_address,"nft_address1".to_string());
        
        //Send NFT to marketplace contract

        let cw721_msg = SellNft{
            list_price:Asset{
                denom:"ujuno".to_string(),
                amount:Uint128::new(1000000)
            }
        };

        let info = mock_info("nft_address1", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"owner1".to_string(),
            token_id:"Hope.1".to_string(),
            msg:to_binary(&cw721_msg).unwrap()
        });
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0,res.messages.len());


         let cw721_msg = SellNft{
            list_price:Asset{
                denom:"ujuno".to_string(),
                amount:Uint128::new(1000000)
            }
        };

       
        let sale_history = query_get_history(deps.as_ref(),"Hope.1".to_string()).unwrap();
        assert_eq!(sale_history,vec![SaleInfo{
            address:"owner1".to_string(),
            denom:"owner".to_string(),
            amount:Uint128::new(0),
            time:mock_env().block.time.seconds()
        }]);

        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"owner2".to_string(),
            token_id:"Hope.2".to_string(),
            msg:to_binary(&cw721_msg).unwrap()
        });
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0,res.messages.len());

        let nft_market_datas = query_get_offerings(deps.as_ref()).unwrap();
        assert_eq!(nft_market_datas.offerings,
            vec![
                QueryOfferingsResult{
                    id :"1".to_string(),
                    token_id:"Hope.1".to_string(),
                    seller : "owner1".to_string(),
                    list_price:Asset { 
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(1000000) 
                    }
                },
                QueryOfferingsResult{
                    id :"2".to_string(),
                    token_id:"Hope.2".to_string(),
                    seller : "owner2".to_string(),
                    list_price:Asset { 
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(1000000) 
                    }
                }
            ]
        );

        let ids = query_get_ids(deps.as_ref()).unwrap();
        assert_eq!(ids,vec!["1".to_string(),"2".to_string()]);

        let offering_result = query_get_offering(deps.as_ref(), ids).unwrap();
        assert_eq!(offering_result,
            vec![
                QueryOfferingsResult{
                    id :"1".to_string(),
                    token_id:"Hope.1".to_string(),
                    seller : "owner1".to_string(),
                    list_price:Asset { 
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(1000000) 
                    }
                },
                QueryOfferingsResult{
                    id :"2".to_string(),
                    token_id:"Hope.2".to_string(),
                    seller : "owner2".to_string(),
                    list_price:Asset { 
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(1000000) 
                    }
                }
            ]
        );

        //Withdraw nft from market place

        let info = mock_info("owner1", &[]);
        let msg = ExecuteMsg::WithdrawNft { offering_id: "1".to_string() };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(1,res.messages.len());
        assert_eq!(res.messages[0].msg, CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "nft_address1".to_string(),
            funds: vec![],
            msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: "owner1".to_string(),
                    token_id: "Hope.1".to_string(),
            }).unwrap(),
        }));      

         let ids = query_get_ids(deps.as_ref()).unwrap();
        assert_eq!(ids,vec!["2".to_string()]);
        
        let nft_market_datas = query_get_offerings(deps.as_ref()).unwrap();
        assert_eq!(nft_market_datas.offerings,
            vec![
                QueryOfferingsResult{
                    id :"2".to_string(),
                    token_id:"Hope.2".to_string(),
                    seller : "owner2".to_string(),
                    list_price:Asset { 
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(1000000) 
                    }
                }
            ]
        );

        //Send NFT to marketplace contract

        let cw721_msg = SellNft{
            list_price:Asset{
                denom:"hope".to_string(),
                amount:Uint128::new(1000000)
            }
        };

        let info = mock_info("nft_address1", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"owner3".to_string(),
            token_id:"Hope.3".to_string(),
            msg:to_binary(&cw721_msg).unwrap()
        });
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let nft_market_datas = query_get_offerings(deps.as_ref()).unwrap();
        assert_eq!(nft_market_datas.offerings,
            vec![            
                QueryOfferingsResult{
                    id :"2".to_string(),
                    token_id:"Hope.2".to_string(),
                    seller : "owner2".to_string(),
                    list_price:Asset { 
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(1000000) 
                    }
                }, QueryOfferingsResult{
                    id :"3".to_string(),
                    token_id:"Hope.3".to_string(),
                    seller : "owner3".to_string(),
                    list_price:Asset { 
                        denom: "hope".to_string(),
                        amount: Uint128::new(1000000) 
                    }
                }
            ]
        );


        let cw721_msg = SellNft{
            list_price:Asset{
                denom:"hope".to_string(),
                amount:Uint128::new(2000000)
            }
        };

        let info = mock_info("nft_address1", &[]);
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
            sender:"owner3".to_string(),
            token_id:"Hope.4".to_string(),
            msg:to_binary(&cw721_msg).unwrap()
        });
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let cw20_msg = BuyNft{
             offering_id:"4".to_string()
        };

        let info = mock_info("token_address1", &[]);
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg{
            sender:"buyer".to_string(),
            amount:Uint128::new(2000000),
            msg:to_binary(&cw20_msg).unwrap()
        });
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        //Buy nft using token

        let cw20_msg = BuyNft{
             offering_id:"3".to_string()
        };

        let info = mock_info("token_address1", &[]);
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg{
            sender:"buyer".to_string(),
            amount:Uint128::new(1000000),
            msg:to_binary(&cw20_msg).unwrap()
        });
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sale_history = query_get_history(deps.as_ref(), "Hope.3".to_string()).unwrap();
        assert_eq!(sale_history,vec![SaleInfo{
            address:"owner3".to_string(),
            denom : "owner".to_string(),
            amount: Uint128::new(0),
            time:mock_env().block.time.seconds()
        },SaleInfo{
            address:"buyer".to_string(),
            denom : "Hope".to_string(),
            amount: Uint128::new(1000000),
            time:mock_env().block.time.seconds()
        }]);

        assert_eq!(4,res.messages.len());
        assert_eq!(res.messages[0].msg, CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "nft_address1".to_string(),
            funds: vec![],
            msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: "buyer".to_string(),
                    token_id: "Hope.3".to_string(),
            }).unwrap(),
        }));
        assert_eq!(res.messages[1].msg, CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token_address1".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "owner3".to_string(),
                    amount:Uint128::new(970000)
            }).unwrap(),
        }));

        assert_eq!(res.messages[2].msg.clone(), CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token_address1".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "admin1".to_string(),
                    amount:Uint128::new(9000)
            }).unwrap(),
        }));


        assert_eq!(res.messages[3].msg, CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token_address1".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "admin2".to_string(),
                    amount:Uint128::new(21000)
            }).unwrap(),
        }));

        let nft_market_datas = query_get_offerings(deps.as_ref()).unwrap();
        assert_eq!(nft_market_datas.offerings,
            vec![            
                QueryOfferingsResult{
                    id :"2".to_string(),
                    token_id:"Hope.2".to_string(),
                    seller : "owner2".to_string(),
                    list_price:Asset { 
                        denom: "ujuno".to_string(),
                        amount: Uint128::new(1000000) 
                    }
                }
            ]
        );

       

        //Buy nft using stable coin 

        let info = mock_info("buyer2", &[Coin{
            denom:"ujuno".to_string(),
            amount:Uint128::new(1000000)
        }]);
        let msg = ExecuteMsg::BuyNft { offering_id: "2".to_string() };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sale_history = query_get_history(deps.as_ref(), "Hope.2".to_string()).unwrap();
        assert_eq!(sale_history,vec![SaleInfo{
            address:"owner2".to_string(),
            denom : "owner".to_string(),
            amount: Uint128::new(0),
            time:mock_env().block.time.seconds()
        },SaleInfo{
            address:"buyer2".to_string(),
            denom : "Juno".to_string(),
            amount: Uint128::new(1000000),
            time:mock_env().block.time.seconds()
        }]);

        assert_eq!(res.messages.len(),4);
        assert_eq!(res.messages[0].msg,CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: state.nft_address.to_string(),
                funds: vec![],
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: "buyer2".to_string(),
                    token_id: "Hope.2".to_string(),
            }).unwrap()
        }));
        assert_eq!(res.messages[1].msg,CosmosMsg::Bank(BankMsg::Send {
                to_address: "owner2".to_string(),
                amount:vec![Coin{
                    denom:"ujuno".to_string(),
                    amount:Uint128::new(970000)
                }]
        }));
        assert_eq!(res.messages[2].msg,CosmosMsg::Bank(BankMsg::Send {
                to_address: "admin1".to_string(),
                amount:vec![Coin{
                    denom:"ujuno".to_string(),
                    amount:Uint128::new(9000)
                }]
        }));
        assert_eq!(res.messages[3].msg,CosmosMsg::Bank(BankMsg::Send {
                to_address: "admin2".to_string(),
                amount:vec![Coin{
                    denom:"ujuno".to_string(),
                    amount:Uint128::new(21000)
                }]
        }));

        let nft_market_datas = query_get_offerings(deps.as_ref()).unwrap();
        assert_eq!(nft_market_datas.offerings,
            vec![]
        );

        let price_info  = query_get_trading(deps.as_ref()).unwrap();
        assert_eq!(price_info,PriceInfo{
            min_juno:Uint128::new(1000000),
            max_juno:Uint128::new(1000000),
            min_hope:Uint128::new(1000000),
            max_hope:Uint128::new(2000000),
            total_hope:Uint128::new(3000000),
            total_juno:Uint128::new(1000000)
        })

    }
}
    