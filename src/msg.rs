
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::{state::{Asset,UserInfo, TvlInfo, SaleInfo}, package::QueryOfferingsResult};
use cosmwasm_std::{Decimal};
use cw721::Cw721ReceiveMsg;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
  pub  owner:String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
 ReceiveNft(Cw721ReceiveMsg),
 Receive(Cw20ReceiveMsg),
 BuyNft{offering_id:String,nft_address:String},
 WithdrawNft{offering_id:String,nft_address:String},
 ChangeOwner{address:String},
 AddTokenAddress{symbol:String,address:String},
 AddCollection{royalty_portion:Decimal,members:Vec<UserInfo>,nft_address:String,offering_id:u64,sale_id:u64},
 UpdateCollection{royalty_portion:Decimal,members:Vec<UserInfo>,nft_address:String},
 FixNft{address:String,token_id:String},
 SetOfferings{address:String,offering:Vec<QueryOfferingsResult>},
 SetTvl{address:String,tvl:Vec<TvlInfo>},
 Migrate{address:String,dest:String,token_id : Vec<String>},
 SetSaleHistory{address:String,history:Vec<SaleInfo>}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns a human-readable representation of the arbiter.
    GetStateInfo {},
    GetMembers{address:String},
    GetOfferingId{address:String},
    GetSaleHistory{address:String,id:Vec<String>},
    GetOfferingPage{id :Vec<String>,address:String },
    GetTradingInfo{address:String},
    GetCollectionInfo{address:String},
    GetTvl{address:String,symbol:String},
    GetTvlAll{address:String,symbols:Vec<String>}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SellNft {
    pub list_price: Asset,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct BuyNft {
    pub offering_id: String,
    pub nft_address : String
}
