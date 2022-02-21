use cosmwasm_std::{Addr, Uint128, Uint64};
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Admin if none set to sender
    pub admin: Option<String>,
    /// ReleaseAddr if not set, set to admin
    pub release_addr: Option<String>,
    pub escrow_amount: Uint128,
    pub allowed_native: String,
    /// release_height_delta gets added to the current block height
    pub release_height_delta: Uint64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig {
        admin: Option<String>,
        release_addr: Option<String>,
        escrow_amount: Option<Uint128>,
        release_height_delta: Option<Uint64>,
        allowed_native: Option<String>,
    },
    ReleaseLockedFunds {
        airdrop_addr: String,
        stage: u8,
    },
    LockFunds {
        airdrop_addr: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Escrow {
        airdrop_addr: String,
        stage: u8,
    },
    ListEscrows {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub admin: Addr,
    /// ReleaseAddr is the address of the released unproven airdrops
    pub release_addr: Addr,
    pub escrow_amount: Uint128,
    /// release height is current_height + default_heighjt
    pub release_height_delta: Uint64,
    pub allowed_native: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EscrowResponse {
    pub source: String,
    pub expiration: Expiration,
    pub escrow_amount: Uint128,
    pub latest_stage: u8,
    pub released: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListEscrowsResponse {
    pub escrows: Vec<EscrowResponse>,
}
