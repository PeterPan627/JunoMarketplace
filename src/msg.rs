
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::state::{Asset,UserInfo};
use cosmwasm_std::{Decimal, Uint128};
use cw721::Cw721ReceiveMsg;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
  pub  owner:String,
  pub token_address:String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
 ReceiveNft(Cw721ReceiveMsg),
 Receive(Cw20ReceiveMsg),
 BuyNft{offering_id:String,nft_address:String},
 WithdrawNft{offering_id:String,nft_address:String},
 ChangeOwner{address:String},
 SetTokenAddress{address:String},
 AddCollection{royalty_portion:Decimal,members:Vec<UserInfo>,nft_address:String},
 UpdateCollection{royalty_portion:Decimal,members:Vec<UserInfo>,nft_address:String},
 FixNft{address:String,token_id:String}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns a human-readable representation of the arbiter.
    GetStateInfo {},
    GetMembers{address:String},
    GetOfferingId{},
    GetSaleHistory{address:String,token_id:String},
    GetOfferingPage{id :Vec<String>,address:String },
    GetTradingInfo{address:String},
    GetCollectionInfo{address:String},
   
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
