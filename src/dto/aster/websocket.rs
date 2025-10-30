use crate::common::Exchange;
use crate::models::TradingSymbol;
use crate::common::ts::{BookTickerData as BookTickerDataTrait, TransactionTime, PushTime};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

/// ASTER Book Ticker 数据结构
/// 格式与 Binance 相同
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsterBookTickerData {
    #[serde(rename = "e")]
    pub event_type: String, // 事件类型 "bookTicker"
    
    #[serde(rename = "u")]
    pub order_book_update_id: u64, // order book updateId
    
    #[serde(rename = "E")]
    pub event_time: i64, // event time
    
    #[serde(rename = "T")]
    pub transaction_time: i64, // transaction time
    
    #[serde(rename = "s")]
    pub symbol: TradingSymbol, // symbol
    
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "b")]
    pub best_bid_price: f64, // best bid price (auto-converted from string)
    
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "B")]
    pub best_bid_qty: f64, // best bid qty (auto-converted from string)
    
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "a")]
    pub best_ask_price: f64, // best ask price (auto-converted from string)
    
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "A")]
    pub best_ask_qty: f64, // best ask qty (auto-converted from string)
}

/// 为 ASTER BookTickerData 实现 BookTickerData trait
impl BookTickerDataTrait for AsterBookTickerData {
    fn bid_price(&self) -> f64 {
        self.best_bid_price
    }
    
    fn bid_quantity(&self) -> f64 {
        self.best_bid_qty
    }
    
    fn ask_price(&self) -> f64 {
        self.best_ask_price
    }
    
    fn ask_quantity(&self) -> f64 {
        self.best_ask_qty
    }
    
    fn symbol(&self) -> &str {
        self.symbol.as_str()
    }
    
    fn event_time(&self) -> i64 {
        self.event_time
    }
    
    fn exchange(&self) -> Exchange {
        Exchange::Aster
    }
}

impl TransactionTime for AsterBookTickerData {
    fn transaction_time(&self) -> i64 {
        self.transaction_time
    }
}

impl PushTime for AsterBookTickerData {
    fn push_time(&self) -> i64 {
        self.event_time
    }
}

/// 为 AsterBookTickerData 实现便利方法
impl AsterBookTickerData {
    /// 获取买卖价差
    pub fn spread(&self) -> f64 {
        self.best_ask_price - self.best_bid_price
    }
    
    /// 获取中间价
    pub fn mid_price(&self) -> f64 {
        (self.best_bid_price + self.best_ask_price) / 2.0
    }
}

