#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    QueryRequest, Response, StdResult, Uint128, Uint64, WasmQuery,
};
use cw2::set_contract_version;

use cw20_merkle_airdrop::msg::LatestStageResponse;
use cw20_merkle_airdrop::msg::QueryMsg::LatestStage;
use cw_storage_plus::{Bound, PrimaryKey};
use cw_utils::{Expiration, NativeBalance};

use crate::error::ContractError;

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
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            admin,
            release_addr,
            escrow_amount,
            default_release_height,
            allowed_native,
        } => execute_update_config(
            deps,
            info,
            env,
            admin,
            release_addr,
            escrow_amount,
            default_release_height,
            allowed_native,
        ),
        ExecuteMsg::ReleaseLockedFunds { airdrop_addr } => {
            execute_release_funds(deps, info, env, airdrop_addr)
        }
        ExecuteMsg::LockFunds { airdrop_addr } => execute_lock_funds(deps, info, env, airdrop_addr),
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    admin: Option<String>,
    release_addr: Option<String>,
    escrow_amount: Option<Uint128>,
    default_release_height: Option<Uint64>,
    allowed_native: Option<String>,
) -> Result<Response, ContractError> {
    // authorize owner
    let mut cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(admin) = admin {
        cfg.admin = deps.api.addr_validate(&admin)?;
    }
    if let Some(release_addr) = release_addr {
        cfg.release_addr = deps.api.addr_validate(&release_addr)?;
    }
    if let Some(escrow_amount) = escrow_amount {
        cfg.escrow_amount = escrow_amount;
    }
    if let Some(default_release_height) = default_release_height {
        cfg.default_release_height = default_release_height;
    }
    if let Some(allowed_native) = allowed_native {
        cfg.allowed_native = allowed_native;
    }

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attribute("action", "update_config"))
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

    let start_after = start_after.and_then(|s| deps.api.addr_validate(&s).ok());
    let start = start_after.map(|s| Bound::Exclusive(s.joined_key()));

    let escrows = ESCROWS
        .range_raw(deps.storage, start, None, Order::Ascending)
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
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Addr, Empty, Uint64, WasmMsg};
    use cw20::Cw20Coin;
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

    pub fn contract_jt_airdrop_controller() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_cw20_merkle_airdrop() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_merkle_airdrop::contract::execute,
            cw20_merkle_airdrop::contract::instantiate,
            cw20_merkle_airdrop::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_cw20_base() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(contract)
    }

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &Addr::unchecked(USER),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: ESCROW_AMOUNT.into(),
                    }],
                )
                .unwrap();
        })
    }

    const ADMIN: &str = "ADMIN";
    const USER: &str = "USER";
    const RELEASE_ADDR: &str = "RELEASE_ADDR";
    const NATIVE_DENOM: &str = "ujunox";
    const ESCROW_AMOUNT: u128 = 100;
    const DEFAULT_RELEASE: u64 = 10;

    fn proper_instantiate() -> (App, String, String, String) {
        let mut app = mock_app();
        let cw20_merkle_id = app.store_code(contract_cw20_merkle_airdrop());
        let cw20_base_id = app.store_code(contract_cw20_base());
        let jt_airdrop_controller_id = app.store_code(contract_jt_airdrop_controller());

        let msg = cw20_base::msg::InstantiateMsg {
            name: "TEST".to_string(),
            symbol: "TEST".to_string(),
            decimals: 3,
            initial_balances: vec![Cw20Coin {
                address: USER.to_string(),
                amount: Uint128::new(1000000),
            }],
            mint: None,
            marketing: None,
        };
        let cw20_base_addr = app
            .instantiate_contract(
                cw20_base_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "test",
                None,
            )
            .unwrap();

        let merkle_root = "b45c1ea28b26adb13e412933c9e055b01fdf7585304b00cd8f1cb220aa6c5e88";
        let msg = cw20_merkle_airdrop::msg::InstantiateMsg {
            owner: Some(ADMIN.to_string()),
            cw20_token_address: cw20_base_addr.to_string(),
        };
        let cw20_airdrop_addr = app
            .instantiate_contract(
                cw20_merkle_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "test",
                None,
            )
            .unwrap();

        let register_msg = cw20_merkle_airdrop::msg::ExecuteMsg::RegisterMerkleRoot {
            merkle_root: merkle_root.to_string(),
            expiration: None,
            start: None,
            total_amount: None,
        };

        let cosmos_msg: CosmosMsg<Empty> = CosmosMsg::from(WasmMsg::Execute {
            contract_addr: cw20_airdrop_addr.clone().to_string(),
            msg: to_binary(&register_msg).unwrap(),
            funds: vec![],
        });

        let msg = InstantiateMsg {
            admin: Some(ADMIN.to_string()),
            release_addr: Some(RELEASE_ADDR.to_string()),
            escrow_amount: Uint128::new(ESCROW_AMOUNT),
            allowed_native: NATIVE_DENOM.to_string(),
            default_release_height: Uint64::new(DEFAULT_RELEASE),
        };
        let jt_controller_addr = app
            .instantiate_contract(
                jt_airdrop_controller_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "test",
                None,
            )
            .unwrap();
        (
            app,
            cw20_base_addr.to_string(),
            cw20_airdrop_addr.to_string(),
            jt_controller_addr.to_string(),
        )
    }

    #[test]
    fn lock_funds() {
        let (mut app, cw20_base_addr, cw20_airdrop_addr, jt_controller_addr) = proper_instantiate();

        // cannot send without tokens
        let msg = ExecuteMsg::LockFunds {
            airdrop_addr: cw20_airdrop_addr,
        };
        let cosmos_msg = CosmosMsg::from(WasmMsg::Execute {
            contract_addr: jt_controller_addr.clone(),
            msg: to_binary(&msg).unwrap(),
            funds: vec![],
        });
        let err = app.execute(Addr::unchecked(USER), cosmos_msg).unwrap_err();
        assert!(matches!(
            err.downcast().unwrap(),
            ContractError::InsufficientAmount {}
        ));

        let cosmos_msg = CosmosMsg::from(WasmMsg::Execute {
            contract_addr: jt_controller_addr,
            msg: to_binary(&msg).unwrap(),
            funds: vec![Coin {
                denom: NATIVE_DENOM.to_string(),
                amount: Uint128::new(ESCROW_AMOUNT),
            }],
        });
        app.execute(Addr::unchecked(USER), cosmos_msg).unwrap();
    }
}
