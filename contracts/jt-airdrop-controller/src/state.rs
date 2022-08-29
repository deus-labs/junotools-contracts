use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128, Uint64};
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    pub escrow_amount: Uint128,
    /// release_height_delta gets added to the current block height
    pub release_height_delta: Uint64,
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

/// ESCROWS: index (airdrop_addr, stage) -> Escrow
pub const ESCROWS: Map<(&Addr, u8), Escrow> = Map::new("escrow");
