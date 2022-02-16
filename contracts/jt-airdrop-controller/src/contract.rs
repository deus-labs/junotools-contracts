#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Order, QueryRequest, Response, StdResult, Uint128, WasmQuery,
};
use cw2::set_contract_version;
use cw20::Balance::Native;
use cw20::{Balance, Cw20CoinVerified, Cw20ReceiveMsg};
use cw20_merkle_airdrop::msg::LatestStageResponse;
use cw20_merkle_airdrop::msg::QueryMsg::LatestStage;
use cw_storage_plus::Bound;
use cw_utils::{Expiration, NativeBalance};
use std::ops::Add;

use crate::error::ContractError;
use crate::msg::QueryMsg::ListEscrows;
use crate::msg::{
    ConfigResponse, EscrowResponse, ExecuteMsg, InstantiateMsg, ListEscrowsResponse, QueryMsg,
};
use crate::state::{Config, Escrow, CONFIG, ESCROWS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:jt-airdrop-controller";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let admin = msg
        .admin
        .clone()
        .map_or(Ok(info.sender), |o| deps.api.addr_validate(&o))?;

    let release_addr = msg
        .release_addr
        .map_or(Ok(admin.clone()), |o| deps.api.addr_validate(&o))?;

    let config = Config {
        admin: admin.clone(),
        release_addr: release_addr.clone(),
        escrow_amount: msg.escrow_amount,
        default_release_height: msg.default_release_height,
        allowed_native: msg.allowed_native,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", admin)
        .add_attribute("release_addr", release_addr.as_str())
        .add_attribute("escrow_amount", msg.escrow_amount))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { .. } => unimplemented!(),
        ExecuteMsg::ReleaseLockedFunds { .. } => unimplemented!(),
        ExecuteMsg::LockFunds { .. } => unimplemented!(),
    }
}

pub fn execute_lock_funds(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    airdrop_contract_addr: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    let funds = NativeBalance(info.funds);
    if !funds.has(&Coin {
        denom: cfg.allowed_native,
        amount: cfg.escrow_amount,
    }) {
        return Err(ContractError::InsufficientAmount {});
    }

    let airdrop_addr = deps.api.addr_validate(airdrop_contract_addr.as_str())?;
    let query_msg = LatestStage {};
    let req = QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: airdrop_contract_addr.clone(),
        msg: to_binary(&query_msg)?,
    });
    let res: LatestStageResponse = deps.querier.query(&req)?;
    let escrow = Escrow {
        source: info.sender.clone(),
        expiration: Expiration::AtHeight(cfg.default_release_height.u64() + env.block.height),
        escrow_amount: cfg.escrow_amount,
        latest_stage: res.latest_stage,
        released: false,
    };

    ESCROWS.save(deps.storage, &airdrop_addr, &escrow)?;

    let res = Response::new().add_attributes(vec![
        ("action", "lock_funds"),
        ("amount", &cfg.escrow_amount.to_string()),
        ("sender", &info.sender.to_string()),
        ("airdrop_addr", &airdrop_contract_addr),
    ]);

    Ok(res)
}

pub fn execute_release_funds(
    deps: DepsMut,
    _info: MessageInfo,
    env: Env,
    airdrop_addr: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    let airdrop_addr = deps.api.addr_validate(&airdrop_addr)?;
    let mut escrow = ESCROWS.load(deps.storage, &airdrop_addr)?;
    if escrow.released {
        return Err(ContractError::EscrowAlreadyReleased {});
    }

    // if expired dao can withdraw
    if escrow.expiration.is_expired(&env.block) {
        // update escrow
        escrow.released = true;
        ESCROWS.save(deps.storage, &airdrop_addr, &escrow)?;

        let send_fund_msg = BankMsg::Send {
            to_address: cfg.release_addr.to_string(),
            amount: vec![Coin {
                denom: cfg.allowed_native,
                amount: escrow.escrow_amount,
            }],
        };

        let res = Response::new()
            .add_message(CosmosMsg::from(send_fund_msg))
            .add_attributes(vec![
                ("action", "release_funds"),
                ("escrow_amount", &cfg.escrow_amount.to_string()),
                ("release_addr", &cfg.release_addr.to_string()),
                ("airdrop_addr", &airdrop_addr.to_string()),
            ]);
        return Ok(res);
    }

    // query latest stage
    let query_msg = LatestStage {};
    let req = QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: airdrop_addr.to_string(),
        msg: to_binary(&query_msg)?,
    });
    let stage_res: LatestStageResponse = deps.querier.query(&req)?;

    // if not expired, and stage has passed, this means we can release funds
    if stage_res.latest_stage > escrow.latest_stage {
        let send_fund_msg = BankMsg::Send {
            to_address: escrow.source.to_string(),
            amount: vec![Coin {
                denom: cfg.allowed_native,
                amount: escrow.escrow_amount,
            }],
        };

        // update escrow
        escrow.released = true;
        ESCROWS.save(deps.storage, &airdrop_addr, &escrow)?;

        let res = Response::new()
            .add_message(CosmosMsg::from(send_fund_msg))
            .add_attributes(vec![
                ("action", "release_funds"),
                ("escrow_amount", &cfg.escrow_amount.to_string()),
                ("release_addr", &escrow.source.to_string()),
                ("airdrop_addr", &airdrop_addr.to_string()),
            ]);
        return Ok(res);
    }

    return Err(ContractError::CannotReleaseFunds {});
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Escrow { airdrop_addr } => to_binary(&query_escrow(deps, airdrop_addr)?),
        QueryMsg::ListEscrows { start_after, limit } => {
            to_binary(&query_list_escrows(deps, start_after, limit)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: cfg.admin,
        escrow_amount: cfg.escrow_amount,
        default_release_height: cfg.default_release_height,
        release_addr: cfg.release_addr,
        allowed_native: cfg.allowed_native,
    })
}

fn query_escrow(deps: Deps, airdrop_addr: String) -> StdResult<EscrowResponse> {
    let addr = deps.api.addr_validate(&airdrop_addr)?;
    let escrow = ESCROWS.load(deps.storage, &addr)?;

    Ok(EscrowResponse {
        source: escrow.source.to_string(),
        expiration: escrow.expiration,
        escrow_amount: escrow.escrow_amount,
        latest_stage: escrow.latest_stage,
        released: escrow.released,
    })
}

// Settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn query_list_escrows(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListEscrowsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::exclusive(s.as_bytes()));

    let escrows = ESCROWS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit.into())
        .collect::<StdResult<Vec<_>>>()?;
    let escrows = escrows
        .into_iter()
        .map(|(_, e)| EscrowResponse {
            source: e.source.to_string(),
            expiration: e.expiration,
            escrow_amount: e.escrow_amount,
            latest_stage: e.latest_stage,
            released: e.released,
        })
        .collect();

    Ok(ListEscrowsResponse { escrows })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            admin: None,
            release_addr: None,
            escrow_amount: Default::default(),
            allowed_native: "".to_string(),
            default_release_height: Default::default(),
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            admin: None,
            release_addr: None,
            escrow_amount: Default::default(),
            allowed_native: "".to_string(),
            default_release_height: Default::default(),
        };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}
