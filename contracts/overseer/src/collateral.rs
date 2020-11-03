use cosmwasm_std::{
    log, to_binary, Api, CanonicalAddr, CosmosMsg, Decimal, Env, Extern, HandleResponse,
    HandleResult, HumanAddr, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use crate::state::{
    read_config, read_loan, read_whitelist_item, store_loan, Config, Loan, WhitelistItem,
};
use moneymarket::{load_oracle_price, CustodyHandleMsg, MarketHandleMsg, OraclePriceResponse};

pub fn handle_lock_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collaterals: Vec<(HumanAddr, Uint128)>,
) -> HandleResult {
    let borrower_raw = deps.api.canonical_address(&env.message.sender)?;
    let mut loan: Loan = read_loan(&deps.storage, &borrower_raw);

    let collaterals_raw: Vec<(CanonicalAddr, Uint128)> = collaterals
        .iter()
        .map(|c| Ok((deps.api.canonical_address(&c.0)?, c.1)))
        .collect::<StdResult<Vec<(CanonicalAddr, Uint128)>>>()?;

    loan.add_collateral(collaterals_raw);
    store_loan(&mut deps.storage, &borrower_raw, &loan)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    for collateral in collaterals.clone() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: collateral.0,
            send: vec![],
            msg: to_binary(&CustodyHandleMsg::LockCollateral {
                borrower: env.message.sender.clone(),
                amount: collateral.1,
            })?,
        }));
    }

    // Loging stuff, so can be removed
    let collateral_logs: Vec<String> = collaterals
        .iter()
        .map(|c| format!("{}{}", c.1, c.0.to_string()))
        .collect();

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "lock_collateral"),
            log("borrower", env.message.sender),
            log("collaterals", collateral_logs.join(",")),
        ],
        data: None,
    })
}

pub fn handle_unlock_collateral<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    collaterals: Vec<(HumanAddr, Uint128)>,
) -> HandleResult {
    let borrower_raw = deps.api.canonical_address(&env.message.sender)?;
    let mut loan: Loan = read_loan(&deps.storage, &borrower_raw);

    let collaterals_raw: Vec<(CanonicalAddr, Uint128)> = collaterals
        .iter()
        .map(|c| Ok((deps.api.canonical_address(&c.0)?, c.1)))
        .collect::<StdResult<Vec<(CanonicalAddr, Uint128)>>>()?;

    // Underflow check is done in sub_collateral
    loan.sub_collateral(collaterals_raw)?;

    // Compute borrow limit with collaterals except unlock target collaterals
    let borrow_limit = compute_borrow_limit(deps, &loan.collaterals)?;
    if borrow_limit < loan.borrow_amount {
        return Err(StdError::generic_err(
            "Cannot unlock collateral more than minimum LTV",
        ));
    }

    store_loan(&mut deps.storage, &borrower_raw, &loan)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    for collateral in collaterals.clone() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: collateral.0,
            send: vec![],
            msg: to_binary(&CustodyHandleMsg::UnlockCollateral {
                borrower: env.message.sender.clone(),
                amount: collateral.1,
            })?,
        }));
    }

    // Loging stuff, so can be removed
    let collateral_logs: Vec<String> = collaterals
        .iter()
        .map(|c| format!("{}{}", c.1, c.0.to_string()))
        .collect();

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "unlock_collateral"),
            log("borrower", env.message.sender),
            log("collaterals", collateral_logs.join(",")),
        ],
        data: None,
    })
}

pub fn handle_borrow<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;

    let borrower_raw = deps.api.canonical_address(&env.message.sender)?;
    let mut loan: Loan = read_loan(&deps.storage, &borrower_raw);
    loan.borrow_amount += amount;

    let borrow_limit = compute_borrow_limit(deps, &loan.collaterals)?;
    if borrow_limit < loan.borrow_amount {
        return Err(StdError::generic_err("Cannot borrow more than minimum LTV"));
    }

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.human_address(&config.market_contract)?,
            send: vec![],
            msg: to_binary(&MarketHandleMsg::ExecuteLoan {
                borrower: env.message.sender.clone(),
                amount,
            })?,
        })],
        log: vec![
            log("action", "borrow"),
            log("borrower", env.message.sender),
            log("amount", amount),
        ],
        data: None,
    })
}

pub fn handle_liquidiate_collateral<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _borrower: HumanAddr,
) -> HandleResult {
    Ok(HandleResponse::default())
}

fn compute_borrow_limit<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    collaterals: &Vec<(CanonicalAddr, Uint128)>,
) -> StdResult<Uint128> {
    let config: Config = read_config(&deps.storage)?;
    let oracle_contract = deps.api.human_address(&config.oracle_contract)?;

    let mut borrow_limit: Uint128 = Uint128::zero();
    for collateral in collaterals.iter() {
        let collateral_token = collateral.0.clone();
        let collateral_amount = collateral.1;

        let price: OraclePriceResponse = load_oracle_price(
            &deps,
            &oracle_contract,
            config.base_denom.to_string(),
            collateral_token.to_string(),
        )?;

        // TODO check price last_updated

        let item: WhitelistItem = read_whitelist_item(&deps.storage, &collateral.0)?;
        borrow_limit += collateral_amount * item.ltv * price.rate;
    }

    Ok(borrow_limit)
}