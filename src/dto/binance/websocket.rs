use crate::common::Exchange;
use crate::common::ts::MarketData;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use ta::{Close, High, Low, Not, Open, Qav, Tbbav, Tbqav, Volume};
use crate::common::ts::IsClosed;
/// 标记价格数据 - 使用 serde_with 自动转换字符串到数值
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkPriceData {
    pub symbol: String,

    #[serde_as(as = "DisplayFromStr")]
    pub mark_price: f64, // 标记价格 (auto-converted from string)

    #[serde_as(as = "DisplayFromStr")]
    pub index_price: f64, // 指数价格 (auto-converted from string)

    #[serde_as(as = "DisplayFromStr")]
    pub estimated_settle_price: f64, // 预估结算价 (auto-converted from string)

    #[serde_as(as = "DisplayFromStr")]
    pub last_funding_rate: f64, // 最新资金费率 (auto-converted from string)

    pub next_funding_time: i64, // 下次资金费时间

    #[serde_as(as = "DisplayFromStr")]
    pub interest_rate: f64, // 利率 (auto-converted from string)

    pub time: i64, // 时间戳
}

/// 深度更新数据 - 使用 serde_with 自动转换价格和数量
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthUpdateData {
    #[serde(rename = "e")]
    pub event_type: String, // "depthUpdate"

    #[serde(rename = "E")]
    pub event_time: i64, // Event time

    #[serde(rename = "T")]
    pub transaction_time: i64, // Transaction time

    #[serde(rename = "s")]
    pub symbol: String, // Symbol

    #[serde(rename = "U")]
    pub first_update_id: i64, // First update ID in event

    #[serde(rename = "u")]
    pub final_update_id: i64, // Final update ID in event

    #[serde(rename = "pu")]
    pub prev_final_update_id: i64, // Final update Id in last stream

    #[serde(rename = "b")]
    #[serde_as(as = "Vec<[DisplayFromStr; 2]>")]
    pub bids: Vec<[f64; 2]>, // Bids to be updated [price, quantity] (auto-converted from strings)

    #[serde(rename = "a")]
    #[serde_as(as = "Vec<[DisplayFromStr; 2]>")]
    pub asks: Vec<[f64; 2]>, // Asks to be updated [price, quantity] (auto-converted from strings)
}

impl DepthUpdateData {
    /// 获取最佳买价
    pub fn best_bid(&self) -> Option<f64> {
        self.bids.first().map(|bid| bid[0])
    }

    /// 获取最佳卖价
    pub fn best_ask(&self) -> Option<f64> {
        self.asks.first().map(|ask| ask[0])
    }

    /// 获取买卖价差
    pub fn spread(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    /// 获取中间价
    pub fn mid_price(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
            _ => None,
        }
    }
}

/// K线数据包装器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineData {
    #[serde(rename = "e")]
    pub event_type: String, // "kline"

    #[serde(rename = "E")]
    pub event_time: i64, // Event time

    #[serde(rename = "s")]
    pub symbol: String, // Symbol

    #[serde(rename = "k")]
    pub kline: KlineInfo,
}

/// K线详细信息 - 使用 serde_with 自动转换字符串到 f64
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineInfo {
    #[serde(rename = "t")]
    pub start_time: i64, // Kline start time

    #[serde(rename = "T")]
    pub close_time: i64, // Kline close time

    #[serde(rename = "s")]
    pub symbol: String, // Symbol

    #[serde(rename = "i")]
    pub interval: String, // Interval

    #[serde(rename = "f")]
    pub first_trade_id: i64, // First trade ID

    #[serde(rename = "L")]
    pub last_trade_id: i64, // Last trade ID

    #[serde(rename = "o")]
    #[serde_as(as = "DisplayFromStr")]
    pub open_price: f64, // Open price (auto-converted from string)

    #[serde(rename = "c")]
    #[serde_as(as = "DisplayFromStr")]
    pub close_price: f64, // Close price (auto-converted from string)

    #[serde(rename = "h")]
    #[serde_as(as = "DisplayFromStr")]
    pub high_price: f64, // High price (auto-converted from string)

    #[serde(rename = "l")]
    #[serde_as(as = "DisplayFromStr")]
    pub low_price: f64, // Low price (auto-converted from string)

    #[serde(rename = "v")]
    #[serde_as(as = "DisplayFromStr")]
    pub base_volume: f64, // Base asset volume (auto-converted from string)

    #[serde(rename = "n")]
    pub trade_count: u64, // Number of trades

    #[serde(rename = "x")]
    pub is_closed: bool, // Is this kline closed?

    #[serde(rename = "q")]
    #[serde_as(as = "DisplayFromStr")]
    pub quote_volume: f64, // Quote asset volume (auto-converted from string)

    #[serde(rename = "V")]
    #[serde_as(as = "DisplayFromStr")]
    pub taker_buy_base_volume: f64, // Taker buy base asset volume (auto-converted from string)

    #[serde(rename = "Q")]
    #[serde_as(as = "DisplayFromStr")]
    pub taker_buy_quote_volume: f64, // Taker buy quote asset volume (auto-converted from string)

    #[serde(rename = "B")]
    pub ignore: String, // Ignore field
}

// 现在字段已经是 f64 类型，不再需要转换方法
// impl KlineInfo {
//     // 删除所有 _f64() 方法
// }

// 为 KlineInfo 实现 ta-rs 的 trait - 现在直接使用字段

impl MarketData for KlineInfo {
    fn which_exchange(&self) -> Exchange {
        Exchange::Binance
    }
}
impl Open for KlineInfo {
    fn open(&self) -> f64 {
        self.open_price
    }
}

impl High for KlineInfo {
    fn high(&self) -> f64 {
        self.high_price
    }
}

impl Low for KlineInfo {
    fn low(&self) -> f64 {
        self.low_price
    }
}

impl Close for KlineInfo {
    fn close(&self) -> f64 {
        self.close_price
    }
}

impl Volume for KlineInfo {
    fn volume(&self) -> f64 {
        self.base_volume
    }
}

impl Qav for KlineInfo {
    fn qav(&self) -> Option<f64> {
        let volume = self.quote_volume;
        if volume == 0.0 { None } else { Some(volume) }
    }
}

impl Tbqav for KlineInfo {
    fn tbqav(&self) -> Option<f64> {
        let volume = self.taker_buy_quote_volume;
        if volume == 0.0 { None } else { Some(volume) }
    }
}

// 添加 Tbbav trait 实现
impl Tbbav for KlineInfo {
    fn tbbav(&self) -> Option<f64> {
        let volume = self.taker_buy_base_volume;
        if volume == 0.0 { None } else { Some(volume) }
    }
}

impl Not for KlineInfo {
    fn not(&self) -> Option<u64> {
        Some(self.trade_count)
    }
}

impl IsClosed for KlineInfo {
    fn is_closed(&self) -> bool {
        self.is_closed
    }
}

// 为 KlineInfo 实现 Taker Buy Base Asset Volume trait
// 注意：这个trait可能在ta-rs中不存在，所以我们先注释掉
// impl Tabbav for KlineInfo {
//     fn tabbav(&self) -> Option<f64> {
//         match self.taker_buy_base_volume {
//             0.0 => None,
//             _ => Some(self.taker_buy_base_volume),
//         }
//     }
// }

/// Book Ticker 数据 - 实时推送指定交易对的最佳买卖价格和数量更新
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookTickerData {
    #[serde(rename = "e")]
    pub event_type: String, // "bookTicker"

    #[serde(rename = "u")]
    pub order_book_update_id: u64, // order book updateId

    #[serde(rename = "E")]
    pub event_time: i64, // event time

    #[serde(rename = "T")]
    pub transaction_time: i64, // transaction time

    #[serde(rename = "s")]
    pub symbol: String, // symbol

    #[serde(rename = "b")]
    #[serde_as(as = "DisplayFromStr")]
    pub best_bid_price: f64, // best bid price (auto-converted from string)

    #[serde(rename = "B")]
    #[serde_as(as = "DisplayFromStr")]
    pub best_bid_qty: f64, // best bid qty (auto-converted from string)

    #[serde(rename = "a")]
    #[serde_as(as = "DisplayFromStr")]
    pub best_ask_price: f64, // best ask price (auto-converted from string)

    #[serde(rename = "A")]
    #[serde_as(as = "DisplayFromStr")]
    pub best_ask_qty: f64, // best ask qty (auto-converted from string)
}

impl BookTickerData {
    /// 获取买卖价差
    pub fn spread(&self) -> f64 {
        self.best_ask_price - self.best_bid_price
    }

    /// 获取中间价
    pub fn mid_price(&self) -> f64 {
        (self.best_bid_price + self.best_ask_price) / 2.0
    }

    /// 获取价差百分比
    pub fn spread_percentage(&self) -> f64 {
        (self.spread() / self.mid_price()) * 100.0
    }

    /// 检查是否有有效的买卖价格
    pub fn has_valid_prices(&self) -> bool {
        self.best_bid_price > 0.0 && self.best_ask_price > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kline_data_parsing() {
        let json_str = r#"{
            "e": "kline",
            "E": 1638747660000,
            "s": "BTCUSDT",
            "k": {
                "t": 1638747660000,
                "T": 1638747719999,
                "s": "BTCUSDT",
                "i": "1m",
                "f": 100,
                "L": 200,
                "o": "0.0010",
                "c": "0.0020",
                "h": "0.0025",
                "l": "0.0015",
                "v": "1000.0",
                "n": 100,
                "x": false,
                "q": "1.0000",
                "V": "500.0",
                "Q": "0.500",
                "B": "123456"
            }
        }"#;

        let data: KlineData = serde_json::from_str(json_str).unwrap();

        assert_eq!(data.symbol, "BTCUSDT");
        assert_eq!(data.event_type, "kline");
        assert_eq!(data.kline.interval, "1m");
        assert_eq!(data.kline.open_price, 0.0010);
        assert_eq!(data.kline.close_price, 0.0020);
        assert_eq!(data.kline.high_price, 0.0025);
        assert_eq!(data.kline.low_price, 0.0015);
        assert_eq!(data.kline.is_closed, false);
    }

    #[test]
    fn test_kline_info_with_extra_fields() {
        // 测试多余字段被忽略
        let json_str = r#"{
            "t": 1638747660000,
            "T": 1638747719999,
            "s": "BTCUSDT",
            "i": "1m",
            "f": 100,
            "L": 200,
            "o": "0.0010",
            "c": "0.0020",
            "h": "0.0025",
            "l": "0.0015",
            "v": "1000.0",
            "n": 100,
            "x": false,
            "q": "1.0000",
            "V": "500.0",
            "Q": "0.500",
            "B": "123456",
            "extra_field": "should_be_ignored",
            "another_extra": 999
        }"#;

        let result: Result<KlineInfo, _> = serde_json::from_str(json_str);
        assert!(result.is_ok(), "应该成功解析，忽略多余字段");

        let kline = result.unwrap();
        assert_eq!(kline.symbol, "BTCUSDT");
        assert_eq!(kline.open_price, 0.0010);
    }

    #[test]
    fn test_ta_rs_traits() {
        let kline = KlineInfo {
            start_time: 1638747660000,
            close_time: 1638747719999,
            symbol: "BTCUSDT".to_string(),
            interval: "1m".to_string(),
            first_trade_id: 100,
            last_trade_id: 200,
            open_price: 50000.0,
            close_price: 51000.0,
            high_price: 52000.0,
            low_price: 49000.0,
            base_volume: 1000.0,
            trade_count: 100,
            is_closed: true,
            quote_volume: 50000000.0,
            taker_buy_base_volume: 600.0,
            taker_buy_quote_volume: 30000000.0,
            ignore: "ignore".to_string(),
        };

        // 测试 ta-rs traits
        assert_eq!(kline.open(), 50000.0);
        assert_eq!(kline.close(), 51000.0);
        assert_eq!(kline.high(), 52000.0);
        assert_eq!(kline.low(), 49000.0);
        assert_eq!(kline.volume(), 1000.0);
        assert_eq!(kline.qav(), Some(50000000.0));
        assert_eq!(kline.tbqav(), Some(30000000.0));
        assert_eq!(kline.not(), Some(100));
    }

    #[test]
    fn test_mark_price_data_parsing() {
        let json_str = r#"{
            "symbol": "BTCUSDT",
            "mark_price": "50000.00",
            "index_price": "50001.00",
            "estimated_settle_price": "50000.50",
            "last_funding_rate": "0.0001",
            "next_funding_time": 1640995200000,
            "interest_rate": "0.0001",
            "time": 1640995200000
        }"#;

        let data: MarkPriceData = serde_json::from_str(json_str).unwrap();

        assert_eq!(data.symbol, "BTCUSDT");
        assert_eq!(data.mark_price, 50000.0);
        assert_eq!(data.index_price, 50001.0);
        assert_eq!(data.estimated_settle_price, 50000.5);
        assert_eq!(data.last_funding_rate, 0.0001);
        assert_eq!(data.interest_rate, 0.0001);
        assert_eq!(data.next_funding_time, 1640995200000);
        assert_eq!(data.time, 1640995200000);
    }

    #[test]
    fn test_depth_update_data_parsing() {
        let json_str = r#"{
            "e": "depthUpdate",
            "E": 1750216875946,
            "T": 1750216875937,
            "s": "ETHUSDT",
            "U": 7818596781509,
            "u": 7818596794961,
            "pu": 7818596780926,
            "b": [["200.00", "260.401"], ["199.99", "100.0"]],
            "a": [["2521.13", "37.315"], ["2521.14", "50.0"]]
        }"#;

        let data: DepthUpdateData = serde_json::from_str(json_str).unwrap();

        assert_eq!(data.symbol, "ETHUSDT");
        assert_eq!(data.event_type, "depthUpdate");
        assert_eq!(data.bids.len(), 2);
        assert_eq!(data.asks.len(), 2);

        // 测试价格和数量的自动转换
        assert_eq!(data.bids[0][0], 200.0); // 价格
        assert_eq!(data.bids[0][1], 260.401); // 数量
        assert_eq!(data.asks[0][0], 2521.13); // 价格
        assert_eq!(data.asks[0][1], 37.315); // 数量

        // 测试便利方法
        assert_eq!(data.best_bid(), Some(200.0));
        assert_eq!(data.best_ask(), Some(2521.13));
        assert_eq!(data.spread(), Some(2321.13));
        assert_eq!(data.mid_price(), Some(1360.565));
    }

    #[test]
    fn test_book_ticker_data_parsing() {
        let json_str = r#"{
            "e": "bookTicker",
            "u": 400900217,
            "E": 1568014460893,
            "T": 1568014460891,
            "s": "BNBUSDT",
            "b": "25.35190000",
            "B": "31.21000000",
            "a": "25.36520000",
            "A": "40.66000000"
        }"#;

        let data: BookTickerData = serde_json::from_str(json_str).unwrap();

        assert_eq!(data.event_type, "bookTicker");
        assert_eq!(data.order_book_update_id, 400900217);
        assert_eq!(data.event_time, 1568014460893);
        assert_eq!(data.transaction_time, 1568014460891);
        assert_eq!(data.symbol, "BNBUSDT");
        assert_eq!(data.best_bid_price, 25.35190000);
        assert_eq!(data.best_bid_qty, 31.21000000);
        assert_eq!(data.best_ask_price, 25.36520000);
        assert_eq!(data.best_ask_qty, 40.66000000);

        // 测试便利方法
        assert_eq!(data.spread(), 0.0133);
        assert_eq!(data.mid_price(), 25.35855);
        assert!((data.spread_percentage() - 0.0524).abs() < 0.0001);
        assert!(data.has_valid_prices());
    }
}

