use ta::{Close, High, Low, Open, Tbbav, Tbqav};
use crate::common::ts::{IsClosed, Symbol, SymbolEnum};
use crate::models::TradingSymbol;

/// 定义一个统一的数据类型，可以包含不同来源的KlineData
#[derive(Debug, Clone)]
pub enum UnifiedKlineData {
    WebSocket(crate::dto::binance::websocket::KlineData),
    Api(crate::dto::binance::rest_api::KlineData),
}

// 为统一类型实现必要的traits
impl Close for UnifiedKlineData {
    fn close(&self) -> f64 {
        match self {
            UnifiedKlineData::WebSocket(data) => data.close(),
            UnifiedKlineData::Api(data) => data.close(),
        }
    }
}

impl High for UnifiedKlineData {
    fn high(&self) -> f64 {
        match self {
            UnifiedKlineData::WebSocket(data) => data.high(),
            UnifiedKlineData::Api(data) => data.high(),
        }
    }
}

impl Low for UnifiedKlineData {
    fn low(&self) -> f64 {
        match self {
            UnifiedKlineData::WebSocket(data) => data.low(),
            UnifiedKlineData::Api(data) => data.low(),
        }
    }
}

impl Open for UnifiedKlineData {
    fn open(&self) -> f64 {
        match self {
            UnifiedKlineData::WebSocket(data) => data.open(),
            UnifiedKlineData::Api(data) => data.open(),
        }
    }
}

impl Tbbav for UnifiedKlineData {
    fn tbbav(&self) -> Option<f64> {
        match self {
            UnifiedKlineData::WebSocket(data) => data.tbbav(),
            UnifiedKlineData::Api(data) => data.tbbav(),
        }
    }
}

impl Tbqav for UnifiedKlineData {
    fn tbqav(&self) -> Option<f64> {
        match self {
            UnifiedKlineData::WebSocket(data) => data.tbqav(),
            UnifiedKlineData::Api(data) => data.tbqav(),
        }
    }
}

impl IsClosed for UnifiedKlineData {
    fn is_closed(&self) -> bool {
        match self {
            UnifiedKlineData::WebSocket(data) => data.is_closed(),
            UnifiedKlineData::Api(data) => data.is_closed(),
        }
    }
}

impl Symbol for UnifiedKlineData {
    fn symbol(&self) -> &str {
        match self {
            UnifiedKlineData::WebSocket(data) => data.symbol(),
            UnifiedKlineData::Api(data) => data.symbol(),
        }
    }
}

impl SymbolEnum for UnifiedKlineData {
    fn symbol_enum(&self) -> &TradingSymbol {
        match self {
            UnifiedKlineData::WebSocket(data) => data.symbol_enum(),
            UnifiedKlineData::Api(data) => &data.symbol,
        }
    }
}
