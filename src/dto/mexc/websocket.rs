use prost::Message;
use ta::{Close, High, Low, Open, Volume};
use crate::common::ts::{MarketData, BookTickerData, TransactionTime, PushTime};
use crate::common::Exchange;

/// MEXC 官方 PushDataV3ApiWrapper 结构 - 简化版本
#[derive(Clone, PartialEq, Message)]
pub struct PushDataV3ApiWrapper {
    #[prost(string, tag = "1")]
    pub channel: String, // 频道名称
    
    #[prost(string, optional, tag = "3")]
    pub symbol: Option<String>, // 交易对
    
    #[prost(string, optional, tag = "4")]
    pub symbol_id: Option<String>, // 交易对ID
    
    #[prost(int64, optional, tag = "5")]
    pub create_time: Option<i64>, // 消息生成时间
    
    #[prost(int64, optional, tag = "6")]
    pub send_time: Option<i64>, // 消息推送时间
    
    // 暂时直接包含K线数据，后续可以优化为 oneof
    #[prost(message, optional, tag = "308")]
    pub public_spot_kline: Option<PublicSpotKlineV3Api>,
    
    // 成交数据
    #[prost(message, optional, tag = "314")]
    pub public_aggre_deals: Option<PublicAggreDealsV3Api>,
    
    // Book Ticker 数据
    #[prost(message, optional, tag = "315")]
    pub public_aggre_book_ticker: Option<PublicAggreBookTickerV3Api>,
}

/// MEXC K线数据 - 严格按照官方 .proto 文件定义
#[derive(Clone, PartialEq, Message)]
pub struct PublicSpotKlineV3Api {
    #[prost(string, tag = "1")]
    pub interval: String, // K线周期(Min1,Min5,Min15,Min30,Min60,Hour4,Hour8,Day1,Week1,Month1)
    
    #[prost(int64, tag = "2")]
    pub window_start: i64, // 窗口开始时间戳(秒时间戳)
    
    #[prost(string, tag = "3")]
    pub opening_price: String, // 开盘价
    
    #[prost(string, tag = "4")]
    pub closing_price: String, // 收盘价
    
    #[prost(string, tag = "5")]
    pub highest_price: String, // 最高价
    
    #[prost(string, tag = "6")]
    pub lowest_price: String, // 最低价
    
    #[prost(string, tag = "7")]
    pub volume: String, // 成交量
    
    #[prost(string, tag = "8")]
    pub amount: String, // 成交额
    
    #[prost(int64, tag = "9")]
    pub window_end: i64, // 窗口结束时间戳(秒时间戳)
}

/// MEXC 成交数据 - 严格按照官方 .proto 文件定义
#[derive(Clone, PartialEq, Message)]
pub struct PublicAggreDealsV3Api {
    #[prost(message, repeated, tag = "1")]
    pub deals: Vec<PublicAggreDealsV3ApiItem>,
    
    #[prost(string, tag = "2")]
    pub event_type: String, // 事件类型
}

/// MEXC 成交数据项
#[derive(Clone, PartialEq, Message)]
pub struct PublicAggreDealsV3ApiItem {
    #[prost(string, tag = "1")]
    pub price: String, // 成交价格
    
    #[prost(string, tag = "2")]
    pub quantity: String, // 成交数量
    
    #[prost(int32, tag = "3")]
    pub trade_type: i32, // 交易类型 1:买 2:卖
    
    #[prost(int64, tag = "4")]
    pub time: i64, // 成交时间
}

// 为 PublicSpotKlineV3Api 实现 ta-rs trait
impl MarketData for PublicSpotKlineV3Api {
    fn which_exchange(&self) -> Exchange {
        Exchange::Mexc
    }
}

impl Open for PublicSpotKlineV3Api {
    fn open(&self) -> f64 {
        self.opening_price.parse().unwrap_or(0.0)
    }
}

impl High for PublicSpotKlineV3Api {
    fn high(&self) -> f64 {
        self.highest_price.parse().unwrap_or(0.0)
    }
}

impl Low for PublicSpotKlineV3Api {
    fn low(&self) -> f64 {
        self.lowest_price.parse().unwrap_or(0.0)
    }
}

impl Close for PublicSpotKlineV3Api {
    fn close(&self) -> f64 {
        self.closing_price.parse().unwrap_or(0.0)
    }
}

impl Volume for PublicSpotKlineV3Api {
    fn volume(&self) -> f64 {
        self.volume.parse().unwrap_or(0.0)
    }
}

// 从 PushDataV3ApiWrapper 提取数据的辅助函数
impl PushDataV3ApiWrapper {
    pub fn extract_kline_data(&self) -> Option<&PublicSpotKlineV3Api> {
        self.public_spot_kline.as_ref()
    }
    
    pub fn extract_deals_data(&self) -> Option<&PublicAggreDealsV3Api> {
        self.public_aggre_deals.as_ref()
    }
    
    /// 提取 Book Ticker 数据
    pub fn extract_book_ticker_data(&self) -> Option<&PublicAggreBookTickerV3Api> {
        self.public_aggre_book_ticker.as_ref()
    }
}

/// MEXC Book Ticker 数据 - 严格按照官方 .proto 文件定义
#[derive(Clone, PartialEq, Message)]
pub struct PublicAggreBookTickerV3Api {
    #[prost(string, tag = "1")]
    pub bid_price: String, // Best bid price
    
    #[prost(string, tag = "2")]
    pub bid_quantity: String, // Best bid quantity
    
    #[prost(string, tag = "3")]
    pub ask_price: String, // Best ask price
    
    #[prost(string, tag = "4")]
    pub ask_quantity: String, // Best ask quantity
}


/// 为 PublicAggreBookTickerV3Api 实现 BookTickerData trait
impl BookTickerData for PublicAggreBookTickerV3Api {
    fn bid_price(&self) -> f64 {
        self.bid_price.parse().unwrap_or(0.0)
    }
    
    fn bid_quantity(&self) -> f64 {
        self.bid_quantity.parse().unwrap_or(0.0)
    }
    
    fn ask_price(&self) -> f64 {
        self.ask_price.parse().unwrap_or(0.0)
    }
    
    fn ask_quantity(&self) -> f64 {
        self.ask_quantity.parse().unwrap_or(0.0)
    }
    
    fn symbol(&self) -> &str {
        // 这里需要从外部传入，暂时返回空字符串
        // 在实际使用时，可以通过包装器或者修改结构体来提供
        ""
    }
    
    fn event_time(&self) -> i64 {
        // 这里需要从外部传入，暂时返回0
        // 在实际使用时，可以通过包装器或者修改结构体来提供
        0
    }
    
    fn exchange(&self) -> Exchange {
        Exchange::Mexc
    }
}

/// 为 PushDataV3ApiWrapper 实现 BookTickerData trait
impl BookTickerData for PushDataV3ApiWrapper {
    fn bid_price(&self) -> f64 {
        self.public_aggre_book_ticker
            .as_ref()
            .map(|ticker| ticker.bid_price.parse().unwrap_or(0.0))
            .unwrap_or(0.0)
    }
    
    fn bid_quantity(&self) -> f64 {
        self.public_aggre_book_ticker
            .as_ref()
            .map(|ticker| ticker.bid_quantity.parse().unwrap_or(0.0))
            .unwrap_or(0.0)
    }
    
    fn ask_price(&self) -> f64 {
        self.public_aggre_book_ticker
            .as_ref()
            .map(|ticker| ticker.ask_price.parse().unwrap_or(0.0))
            .unwrap_or(0.0)
    }
    
    fn ask_quantity(&self) -> f64 {
        self.public_aggre_book_ticker
            .as_ref()
            .map(|ticker| ticker.ask_quantity.parse().unwrap_or(0.0))
            .unwrap_or(0.0)
    }
    
    fn symbol(&self) -> &str {
        self.symbol.as_deref().unwrap_or("")
    }
    
    fn event_time(&self) -> i64 {
        self.send_time.unwrap_or(0)
    }
    
    fn exchange(&self) -> Exchange {
        Exchange::Mexc
    }
}

impl TransactionTime for PushDataV3ApiWrapper {
    fn transaction_time(&self) -> i64 {
        // 如果 create_time 不存在，使用 send_time 作为备选
        // 因为 MEXC 的 Book Ticker 数据本身就是实时更新的
        self.create_time.unwrap_or_else(|| self.send_time.unwrap_or(0))
    }
}

impl PushTime for PushDataV3ApiWrapper {
    fn push_time(&self) -> i64 {
        self.send_time.unwrap_or(0)
    }
}