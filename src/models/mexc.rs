use crate::common::ts::{Symbol, SymbolEnum};
use crate::models::{Exchange, TradingSymbol};
use std::collections::BTreeMap;
use crate::dto::mexc::PublicAggreDepthsV3Api;
use ta::Orderbook;
type Level = u8;
#[derive(Debug, Clone)]
pub struct SingleOrderBook {
    pub price: u64,
    pub quantity: u64,
}
pub struct CommonDepeth {
    pub bid_list: BTreeMap<Level, SingleOrderBook>,
    pub ask_list: BTreeMap<Level, SingleOrderBook>,
    pub symbol: TradingSymbol,
    pub timestamp: i64,
    pub exchange: Exchange,
}

// 可以在这里添加其他 MEXC 相关的结构体

