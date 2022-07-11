use cosmwasm_std::{Uint128, Decimal};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_storage_plus::{Item,Map};

pub const CONFIG: Item<State> = Item::new("config_state");
pub const MEMBERS : Map<&str,Vec<UserInfo>> = Map::new("config_members");
pub const OFFERINGS: Map<(&str,&str), Offering> = Map::new("offerings");
pub const SALEHISTORY : Map<(&str,&str), SaleInfo> = Map::new("sale");
pub const PRICEINFO : Map<&str,PriceInfo> = Map::new("price_info");
pub const COLLECTIONINFO : Map<&str, CollectionInfo> = Map::new("collection_info");
pub const TOKENADDRESS : Map<&str, String> = Map::new("token_address");
pub const TVL:Map<(&str,&str),Uint128> = Map::new("tvl_config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner:String,
    pub new : bool,
   
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Offering {
    pub token_id: String,
    pub seller: String,
    pub list_price: Asset,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Asset {
    pub denom:String,
    pub amount:Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct UserInfo {
    pub address: String,
    pub portion:Decimal
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SaleInfo {
    pub from :String,
    pub to: String,
    pub denom:String,
    pub amount:Uint128,
    pub time : u64,
    pub nft_address:String,
    pub token_id:String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PriceInfo {
   pub total_juno : Uint128,
   pub total_hope: Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]

pub struct CollectionInfo{
    pub nft_address :String,
    pub offering_id:u64,
    pub royalty_portion:Decimal,
    pub sale_id : u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TvlInfo {
   pub denom : String,
   pub amount: Uint128
}
