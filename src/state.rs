use cosmwasm_std::{Uint128, Decimal,Addr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_storage_plus::{Item,Map};

pub const CONFIG: Item<State> = Item::new("config_state");
pub const MEMBERS : Item<Vec<UserInfo>> = Item::new("config_members");
pub const OFFERINGS: Map<&str, Offering> = Map::new("offerings");
pub const SALEHISTORY : Map<&str, Vec<SaleInfo>> = Map::new("sale");
pub const PRICEINFO : Item<PriceInfo> = Item::new("price_info");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner:String,
    pub token_address:String,
    pub nft_address:String,
    pub offering_id:u64,
    pub royalty_portion:Decimal
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
    pub address: String,
    pub denom:String,
    pub amount:Uint128,
    pub time : u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PriceInfo {
   pub min_juno : Uint128,
   pub max_juno : Uint128,
   pub min_hope : Uint128,
   pub max_hope : Uint128,
   pub total_juno : Uint128,
   pub total_hope: Uint128
}

