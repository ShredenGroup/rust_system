use serde::{Deserialize, Serialize};
use ta::{Open, High, Low, Close, Volume, Qav, Tbqav, Not};

/// 标记价格数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkPriceData {
    pub symbol: String,
    pub mark_price: String,
    pub index_price: String,
    pub estimated_settle_price: String,
    pub last_funding_rate: String,
    pub next_funding_time: i64,
    pub interest_rate: String,
    pub time: i64,
}

/// 深度更新数据
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
    pub bids: Vec<[String; 2]>, // Bids to be updated [price, quantity]

    #[serde(rename = "a")]
    pub asks: Vec<[String; 2]>, // Asks to be updated [price, quantity]
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

/// K线详细信息
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
    pub open_price: f64, // Open price

    #[serde(rename = "c")]
    pub close_price: f64, // Close price

    #[serde(rename = "h")]
    pub high_price: f64, // High price

    #[serde(rename = "l")]
    pub low_price: f64, // Low price

    #[serde(rename = "v")]
    pub base_volume: f64, // Base asset volume

    #[serde(rename = "n")]
    pub trade_count: u64, // Number of trades

    #[serde(rename = "x")]
    pub is_closed: bool, // Is this kline closed?

    #[serde(rename = "q")]
    pub quote_volume: f64, // Quote asset volume

    #[serde(rename = "V")]
    pub taker_buy_base_volume: f64, // Taker buy base asset volume

    #[serde(rename = "Q")]
    pub taker_buy_quote_volume: f64, // Taker buy quote asset volume

    #[serde(rename = "B")]
    pub ignore: String, // Ignore field
}

// 为 KlineInfo 实现 ta-rs 的 trait
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
        match self.quote_volume {
            0.0 => None,
            _ => Some(self.quote_volume),
        }
    }
}

impl Tbqav for KlineInfo {
    fn tbqav(&self) -> Option<f64> {
        match self.taker_buy_quote_volume {
            0.0 => None,
            _ => Some(self.taker_buy_quote_volume),
        }
    }
}

impl Not for KlineInfo {
    fn not(&self) -> Option<u64> {
        Some(self.trade_count)
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
                "o": 0.0010,
                "c": 0.0020,
                "h": 0.0025,
                "l": 0.0015,
                "v": 1000.0,
                "n": 100,
                "x": false,
                "q": 1.0000,
                "V": 500.0,
                "Q": 0.500,
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
            "o": 0.0010,
            "c": 0.0020,
            "h": 0.0025,
            "l": 0.0015,
            "v": 1000.0,
            "n": 100,
            "x": false,
            "q": 1.0000,
            "V": 500.0,
            "Q": 0.500,
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
} 