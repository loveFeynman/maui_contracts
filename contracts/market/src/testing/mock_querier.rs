use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Api, CanonicalAddr, Coin, Decimal, Extern, HumanAddr,
    Querier, QuerierResult, QueryRequest, SystemError, Uint128, WasmQuery,
};
use cosmwasm_storage::to_length_prefixed;
use std::collections::HashMap;

use cw20::TokenInfoResponse;
use moneymarket::{
    BorrowLimitResponse, BorrowRateResponse, DistributionParamsResponse, EpochStateResponse,
    LoanAmountResponse, OraclePriceResponse,
};
use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query borrow rate to interest model contract
    BorrowRate {},
    /// Query borrow limit to overseer contract
    BorrowLimit { borrower: HumanAddr },
}

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    canonical_length: usize,
    contract_balance: &[Coin],
) -> Extern<MockStorage, MockApi, WasmMockQuerier> {
    let contract_addr = HumanAddr::from(MOCK_CONTRACT_ADDR);
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(
        MockQuerier::new(&[(&contract_addr, contract_balance)]),
        MockApi::new(canonical_length),
    );

    Extern {
        storage: MockStorage::default(),
        api: MockApi::new(canonical_length),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    tax_querier: TaxQuerier,
    borrow_rate_querier: BorrowRateQuerier,
    borrow_limit_querier: BorrowLimitQuerier,
}

#[derive(Clone, Default)]
pub struct TaxQuerier {
    rate: Decimal,
    // this lets us iterate over all pairs that match the first string
    caps: HashMap<String, Uint128>,
}

impl TaxQuerier {
    pub fn new(rate: Decimal, caps: &[(&String, &Uint128)]) -> Self {
        TaxQuerier {
            rate,
            caps: caps_to_map(caps),
        }
    }
}

pub(crate) fn caps_to_map(caps: &[(&String, &Uint128)]) -> HashMap<String, Uint128> {
    let mut owner_map: HashMap<String, Uint128> = HashMap::new();
    for (denom, cap) in caps.iter() {
        owner_map.insert(denom.to_string(), **cap);
    }
    owner_map
}

#[derive(Clone, Default)]
pub struct DistributionParamsQuerier {
    // this lets us iterate over all pairs that match the first string
    distribution_params: HashMap<HumanAddr, (Decimal, Decimal)>,
}

impl DistributionParamsQuerier {
    pub fn new(distribution_params: &[(&HumanAddr, &(Decimal, Decimal))]) -> Self {
        DistributionParamsQuerier {
            distribution_params: distribution_params_to_map(distribution_params),
        }
    }
}

pub(crate) fn distribution_params_to_map(
    caps: &[(&HumanAddr, &(Decimal, Decimal))],
) -> HashMap<HumanAddr, (Decimal, Decimal)> {
    let mut distribution_params_map: HashMap<HumanAddr, (Decimal, Decimal)> = HashMap::new();
    for (asset_token, distribution_params) in caps.iter() {
        distribution_params_map.insert((*asset_token).clone(), **distribution_params);
    }
    distribution_params_map
}

#[derive(Clone, Default)]
pub struct OraclePriceQuerier {
    // this lets us iterate over all pairs that match the first string
    oracle_price: HashMap<(String, String), (Decimal, u64, u64)>,
}

impl OraclePriceQuerier {
    pub fn new(oracle_price: &[(&(String, String), &(Decimal, u64, u64))]) -> Self {
        OraclePriceQuerier {
            oracle_price: oracle_price_to_map(oracle_price),
        }
    }
}

pub(crate) fn oracle_price_to_map(
    oracle_price: &[(&(String, String), &(Decimal, u64, u64))],
) -> HashMap<(String, String), (Decimal, u64, u64)> {
    let mut oracle_price_map: HashMap<(String, String), (Decimal, u64, u64)> = HashMap::new();
    for (base_quote, oracle_price) in oracle_price.iter() {
        oracle_price_map.insert((*base_quote).clone(), **oracle_price);
    }

    oracle_price_map
}

#[derive(Clone, Default)]
pub struct EpochStateQuerier {
    // this lets us iterate over all pairs that match the first string
    epoch_state: HashMap<HumanAddr, (Uint128, Decimal)>,
}

impl EpochStateQuerier {
    pub fn new(epoch_state: &[(&HumanAddr, &(Uint128, Decimal))]) -> Self {
        EpochStateQuerier {
            epoch_state: epoch_state_to_map(epoch_state),
        }
    }
}

pub(crate) fn epoch_state_to_map(
    epoch_state: &[(&HumanAddr, &(Uint128, Decimal))],
) -> HashMap<HumanAddr, (Uint128, Decimal)> {
    let mut epoch_state_map: HashMap<HumanAddr, (Uint128, Decimal)> = HashMap::new();
    for (market_contract, epoch_state) in epoch_state.iter() {
        epoch_state_map.insert((*market_contract).clone(), **epoch_state);
    }
    epoch_state_map
}

#[derive(Clone, Default)]
pub struct LoanAmountQuerier {
    // this lets us iterate over all pairs that match the first string
    loan_amount: HashMap<HumanAddr, Uint128>,
}

impl LoanAmountQuerier {
    pub fn new(loan_amount: &[(&HumanAddr, &Uint128)]) -> Self {
        LoanAmountQuerier {
            loan_amount: loan_amount_to_map(loan_amount),
        }
    }
}

pub(crate) fn loan_amount_to_map(
    loan_amount: &[(&HumanAddr, &Uint128)],
) -> HashMap<HumanAddr, Uint128> {
    let mut loan_amount_map: HashMap<HumanAddr, Uint128> = HashMap::new();
    for (market_contract, loan_amount) in loan_amount.iter() {
        loan_amount_map.insert((*market_contract).clone(), **loan_amount);
    }
    loan_amount_map
}

#[derive(Clone, Default)]
pub struct BorrowRateQuerier {
    // this lets us iterate over all pairs that match the first string
    borrower_rate: HashMap<HumanAddr, Decimal>,
}

impl BorrowRateQuerier {
    pub fn new(borrower_rate: &[(&HumanAddr, &Decimal)]) -> Self {
        BorrowRateQuerier {
            borrower_rate: borrower_rate_to_map(borrower_rate),
        }
    }
}

pub(crate) fn borrower_rate_to_map(
    borrower_rate: &[(&HumanAddr, &Decimal)],
) -> HashMap<HumanAddr, Decimal> {
    let mut borrower_rate_map: HashMap<HumanAddr, Decimal> = HashMap::new();
    for (market_contract, borrower_rate) in borrower_rate.iter() {
        borrower_rate_map.insert((*market_contract).clone(), **borrower_rate);
    }
    borrower_rate_map
}

#[derive(Clone, Default)]
pub struct BorrowLimitQuerier {
    // this lets us iterate over all pairs that match the first string
    borrow_limit: HashMap<HumanAddr, Uint128>,
}

impl BorrowLimitQuerier {
    pub fn new(borrow_limit: &[(&HumanAddr, &Uint128)]) -> Self {
        BorrowLimitQuerier {
            borrow_limit: borrow_limit_to_map(borrow_limit),
        }
    }
}

pub(crate) fn borrow_limit_to_map(
    borrow_limit: &[(&HumanAddr, &Uint128)],
) -> HashMap<HumanAddr, Uint128> {
    let mut borrow_limit_map: HashMap<HumanAddr, Uint128> = HashMap::new();
    for (market_contract, borrow_limit) in borrow_limit.iter() {
        borrow_limit_map.insert((*market_contract).clone(), **borrow_limit);
    }
    borrow_limit_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => {
                if &TerraRoute::Treasury == route {
                    match query_data {
                        TerraQuery::TaxRate {} => {
                            let res = TaxRateResponse {
                                rate: self.tax_querier.rate,
                            };
                            Ok(to_binary(&res))
                        }
                        TerraQuery::TaxCap { denom } => {
                            let cap = self
                                .tax_querier
                                .caps
                                .get(denom)
                                .copied()
                                .unwrap_or_default();
                            let res = TaxCapResponse { cap };
                            Ok(to_binary(&res))
                        }
                        _ => panic!("DO NOT ENTER HERE"),
                    }
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match from_binary(&msg).unwrap() {
                    QueryMsg::BorrowRate {} => {
                        match self.borrow_rate_querier.borrower_rate.get(&contract_addr) {
                            Some(v) => Ok(to_binary(&BorrowRateResponse { rate: *v })),
                            None => Err(SystemError::InvalidRequest {
                                error: "No borrow rate exists".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
                    QueryMsg::BorrowLimit { borrower } => {
                        match self.borrow_limit_querier.borrow_limit.get(&borrower) {
                            Some(v) => Ok(to_binary(&BorrowLimitResponse {
                                borrower,
                                borrow_limit: *v,
                            })),
                            None => Err(SystemError::InvalidRequest {
                                error: "No borrow limit exists".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
                }
            }
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new<A: Api>(base: MockQuerier<TerraQueryWrapper>, _api: A) -> Self {
        WasmMockQuerier {
            base,
            tax_querier: TaxQuerier::default(),
            borrow_rate_querier: BorrowRateQuerier::default(),
            borrow_limit_querier: BorrowLimitQuerier::default(),
        }
    }

    // configure the tax mock querier
    pub fn with_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
        self.tax_querier = TaxQuerier::new(rate, caps);
    }

    pub fn with_borrow_rate(&mut self, borrow_rate: &[(&HumanAddr, &Decimal)]) {
        self.borrow_rate_querier = BorrowRateQuerier::new(borrow_rate);
    }

    pub fn with_borrow_limit(&mut self, borrow_limit: &[(&HumanAddr, &Uint128)]) {
        self.borrow_limit_querier = BorrowLimitQuerier::new(borrow_limit);
    }
}
