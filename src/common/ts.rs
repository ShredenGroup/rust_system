use super::Exchange;
use ndarray::Array;
use ndarray::Ix2;
use super::StrategyName;
pub trait ToArray {
    fn to_ndarray(&self) -> Array<f32, Ix2>;
}

// 重新设计 Strategy trait，使用泛型而不是 trait object
pub trait Strategy<T>: Send + Sync {
    type Output;
    fn on_kline_update(&mut self, input: T) -> Self::Output;
    fn name(&self) -> String;
}

pub trait IsClosed {
    fn is_closed(&self) -> bool;
}

pub trait MarketData {
    fn which_exchange(&self) -> Exchange;
}

pub trait SignalTs{
    fn signal_strategy(&self) -> StrategyName;
}

/// Book Ticker 数据的统一 trait
/// 让 Binance 和 MEXC 的 Book Ticker 数据结构可以兼容使用
pub trait BookTickerData: Send + Sync {
    /// 获取买价
    fn bid_price(&self) -> f64;
    
    /// 获取买量
    fn bid_quantity(&self) -> f64;
    
    /// 获取卖价
    fn ask_price(&self) -> f64;
    
    /// 获取卖量
    fn ask_quantity(&self) -> f64;
    
    /// 获取买卖价差
    fn spread(&self) -> f64 {
        self.ask_price() - self.bid_price()
    }
    
    /// 获取中间价
    fn mid_price(&self) -> f64 {
        (self.bid_price() + self.ask_price()) / 2.0
    }
    
    /// 获取价差百分比
    fn spread_percentage(&self) -> f64 {
        let mid = self.mid_price();
        if mid > 0.0 {
            (self.spread() / mid) * 100.0
        } else {
            0.0
        }
    }
    
    /// 检查是否有有效的买卖价格
    fn has_valid_prices(&self) -> bool {
        self.bid_price() > 0.0 && self.ask_price() > 0.0
    }
    
    /// 获取交易对符号
    fn symbol(&self) -> &str;
    
    /// 获取事件时间戳（毫秒）
    fn event_time(&self) -> i64;
    
    /// 获取交易所
    fn exchange(&self) -> Exchange;
}

/// 交易时间 trait - 获取真实的市场事件发生时间
pub trait TransactionTime {
    fn transaction_time(&self) -> i64;
}

/// 推送时间 trait - 获取消息从交易所推送出来的时间
pub trait PushTime {
    fn push_time(&self) -> i64;
}