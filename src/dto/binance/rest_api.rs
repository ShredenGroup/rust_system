use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;
use ta::{Close, High, Low, Not, Open, Qav, Tbbav, Tbqav, Volume};
use std::str::FromStr;
use crate::common::ts::{IsClosed, Symbol, MarketData, TransactionTime,SymbolEnum};
use crate::common::{Exchange, TradingSymbol};
/// 订单类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Limit,
    Market,
    Stop,
    StopMarket,
    TakeProfit,
    TakeProfitMarket,
    TrailingStopMarket,
}

/// 订单方向枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

/// 时间强制类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeInForce {
    Gtc, // Good Till Cancel
    Ioc, // Immediate or Cancel
    Fok, // Fill or Kill
    Gtx, // Good Till Crossing
    Gtd, // Good Till Date
}

/// K线数据请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineRequest {
    pub symbol: String,
    pub interval: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
}

/// K线数据响应（REST API格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineData {
    /// 交易对符号 - 不从JSON反序列化，需要手动设置
    #[serde(skip)]
    pub symbol: TradingSymbol,
    #[serde(rename = "0")]
    pub open_time: i64,
    #[serde(rename = "1")]
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub open: f64,
    #[serde(rename = "2")]
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub high: f64,
    #[serde(rename = "3")]
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub low: f64,
    #[serde(rename = "4")]
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub close: f64,
    #[serde(rename = "5")]
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub volume: f64,
    #[serde(rename = "6")]
    pub close_time: i64,
    #[serde(rename = "7")]
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub quote_volume: f64,
    #[serde(rename = "8")]
    pub trades_count: u64,
    #[serde(rename = "9")]
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub taker_buy_volume: f64,
    #[serde(rename = "10")]
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub taker_buy_quote_volume: f64,
    #[serde(rename = "11")]
    #[serde(deserialize_with = "deserialize_string_to_f64")]
    pub ignore: f64,
}

/// 辅助函数：将字符串反序列化为 f64
fn deserialize_string_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    f64::from_str(&s).map_err(serde::de::Error::custom)
}

// 为 KlineInfo 实现 ta-rs 的 trait
impl Open for KlineData {
    fn open(&self) -> f64 {
        self.open
    }
}

impl High for KlineData {
    fn high(&self) -> f64 {
        self.high
    }
}

impl Low for KlineData {
    fn low(&self) -> f64 {
        self.low
    }
}

impl Close for KlineData {
    fn close(&self) -> f64 {
        self.close
    }
}

impl Volume for KlineData {
    fn volume(&self) -> f64 {
        self.volume
    }
}

impl Qav for KlineData {
    fn qav(&self) -> Option<f64> {
        match self.quote_volume {
            0.0 => None,
            _ => Some(self.quote_volume),
        }
    }
}

impl Tbqav for KlineData {
    fn tbqav(&self) -> Option<f64> {
        match self.taker_buy_quote_volume {
            0.0 => None,
            _ => Some(self.taker_buy_quote_volume),
        }
    }
}
impl Tbbav for KlineData{
    fn tbbav(&self) -> Option<f64> {
        match self.taker_buy_volume{
            0.0 => None,
            _ => Some(self.taker_buy_volume)
        }
    }
}
impl Not for KlineData {
    fn not(&self) -> Option<u64> {
        Some(self.trades_count)
    }
}
impl IsClosed for KlineData{
    fn is_closed(&self) -> bool {
        true
    }
}

impl Symbol for KlineData {
    fn symbol(&self) -> &str {
        self.symbol.as_str()
    }
}

impl MarketData for KlineData {
    fn which_exchange(&self) -> Exchange {
        Exchange::Binance
    }
}

impl TransactionTime for KlineData {
    fn transaction_time(&self) -> i64 {
        self.close_time
    }
}
impl SymbolEnum for KlineData {
    fn symbol_enum(&self) -> &TradingSymbol {
        &self.symbol
    }
}



/// K线数据响应类型别名
pub type KlineResponse = Vec<KlineData>;

/// 下单请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,

    // 可选参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_client_order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_protect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_order_resp_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_match: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_trade_prevention_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub good_till_date: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recv_window: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

/// 下单响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    #[serde(default)]
    pub client_order_id: Option<String>,
    pub cum_qty: String,
    pub cum_quote: String,
    pub executed_qty: String,
    pub order_id: i64,
    pub avg_price: String,
    pub orig_qty: String,
    pub price: String,
    pub reduce_only: bool,
    pub side: String,
    pub position_side: String,
    pub status: String,
    pub stop_price: String,
    pub close_position: bool,
    pub symbol: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub orig_type: String,
    pub activate_price: Option<String>,
    pub price_rate: Option<String>,
    pub update_time: i64,
    pub working_type: String,
    pub price_protect: bool,
    pub price_match: String,
    pub self_trade_prevention_mode: String,
    pub good_till_date: Option<i64>,
}

// 实现 Default trait
impl Default for OrderRequest {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            position_side: None,
            time_in_force: None,
            quantity: None,
            reduce_only: None,
            price: None,
            new_client_order_id: None,
            stop_price: None,
            close_position: None,
            activation_price: None,
            callback_rate: None,
            working_type: None,
            price_protect: None,
            new_order_resp_type: None,
            price_match: None,
            self_trade_prevention_mode: None,
            good_till_date: None,
            recv_window: None,
            timestamp: None,
        }
    }
}

impl KlineRequest {
    pub fn to_params(&self) -> Result<HashMap<String, String>> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), self.symbol.clone());
        params.insert("interval".to_string(), self.interval.clone());
        if let Some(ref start_time) = self.start_time {
            params.insert("startTime".to_string(), start_time.clone());
        }
        if let Some(ref end_time) = self.end_time {
            params.insert("endTime".to_string(), end_time.clone());
        }
        if let Some(ref limit) = self.limit {
            params.insert("limit".to_string(), limit.clone());
        }
        Ok(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_type_serialization() {
        assert_eq!(
            serde_json::to_string(&OrderType::Market).unwrap(),
            "\"MARKET\""
        );
        assert_eq!(
            serde_json::to_string(&OrderType::Limit).unwrap(),
            "\"LIMIT\""
        );
    }

    #[test]
    fn test_order_side_serialization() {
        assert_eq!(
            serde_json::to_string(&OrderSide::Buy).unwrap(),
            "\"BUY\""
        );
        assert_eq!(
            serde_json::to_string(&OrderSide::Sell).unwrap(),
            "\"SELL\""
        );
    }

    #[test]
    fn test_kline_request_to_params() {
        let request = KlineRequest {
            symbol: "BTCUSDT".to_string(),
            interval: "1m".to_string(),
            start_time: Some("1640995200000".to_string()),
            end_time: None,
            limit: Some("100".to_string()),
        };

        let params = request.to_params().unwrap();
        assert_eq!(params.get("symbol"), Some(&"BTCUSDT".to_string()));
        assert_eq!(params.get("interval"), Some(&"1m".to_string()));
        assert_eq!(params.get("startTime"), Some(&"1640995200000".to_string()));
        assert_eq!(params.get("limit"), Some(&"100".to_string()));
        assert_eq!(params.get("endTime"), None);
    }

    #[test]
    fn test_order_request_default() {
        let request = OrderRequest::default();
        assert_eq!(request.symbol, "");
        assert_eq!(request.side, OrderSide::Buy);
        assert_eq!(request.order_type, OrderType::Market);
        assert_eq!(request.quantity, None);
    }

    #[test]
    fn test_kline_data_string_parsing() {
        let json_str = r#"[
            1640995200000,
            "50000.00",
            "51000.00",
            "49000.00",
            "50500.00",
            "1000.00",
            1640995259999,
            "50250000.00",
            1000,
            "600.00",
            "30150000.00",
            "0.00"
        ]"#;

        let mut kline: KlineData = serde_json::from_str(json_str).unwrap();
        kline.symbol = TradingSymbol::BTCUSDT;
        assert_eq!(kline.symbol.as_str(), "BTCUSDT");
        assert_eq!(kline.open_time, 1640995200000);
        assert_eq!(kline.open, 50000.0);
        assert_eq!(kline.high, 51000.0);
        assert_eq!(kline.low, 49000.0);
        assert_eq!(kline.close, 50500.0);
        assert_eq!(kline.volume, 1000.0);
        assert_eq!(kline.quote_volume, 50250000.0);
        assert_eq!(kline.taker_buy_volume, 600.0);
        assert_eq!(kline.taker_buy_quote_volume, 30150000.0);
        assert_eq!(kline.ignore, 0.0);
    }

    #[test]
    fn test_symbol_trait_implementation() {
        use crate::common::ts::Symbol;
        
        let json_str = r#"[
            1640995200000,
            "50000.00",
            "51000.00",
            "49000.00",
            "50500.00",
            "1000.00",
            1640995259999,
            "50250000.00",
            1000,
            "600.00",
            "30150000.00",
            "0.00"
        ]"#;

        let mut kline: KlineData = serde_json::from_str(json_str).unwrap();
        kline.symbol = TradingSymbol::ETHUSDT;
        assert_eq!(kline.symbol(), "ETHUSDT");
    }

    #[test]
    fn test_manual_symbol_setting() {
        let json_data = r#"[
            [1640995200000, "50000.00", "51000.00", "49000.00", "50500.00", "1000.00", 1640995259999, "50250000.00", 1000, "600.00", "30150000.00", "0.00"],
            [1640995260000, "50500.00", "51500.00", "49500.00", "51000.00", "1100.00", 1640995319999, "55275000.00", 1100, "660.00", "33165000.00", "0.00"]
        ]"#;

        let mut klines: Vec<KlineData> = serde_json::from_str(json_data).unwrap();
        
        // 手动为每个 KlineData 设置 symbol
        for kline in &mut klines {
            kline.symbol = TradingSymbol::BTCUSDT;
        }
        
        assert_eq!(klines.len(), 2);
        assert_eq!(klines[0].symbol.as_str(), "BTCUSDT");
        assert_eq!(klines[1].symbol.as_str(), "BTCUSDT");
        assert_eq!(klines[0].open, 50000.0);
        assert_eq!(klines[1].open, 50500.0);
    }
} 