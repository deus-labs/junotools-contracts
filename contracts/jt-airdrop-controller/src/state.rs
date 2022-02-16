use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128, Uint64};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    /// ReleaseAddr is the address of the released unproven airdrops
    pub release_addr: Addr,
    pub escrow_amount: Uint128,
    /// release height is current_height + default_heighjt
    pub default_release_height: Uint64,
    pub allowed_native: String,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Escrow {
    /// if refunded, funds go to the source
    pub source: Addr,
    pub expiration: Expiration,
    pub escrow_amount: Uint128,
    pub latest_stage: u8,
    pub released: bool,
}

pub const ESCROWS: Map<&Addr, Escrow> = Map::new("escrow");
