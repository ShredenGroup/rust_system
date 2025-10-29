use crate::dto::mexc::PushDataV3ApiWrapper;
use crate::dto::binance::websocket::BookTickerData;
use crate::models::{TradingSymbol, Exchange};
use crate::common::ts::TransactionTime;
use std::num::ParseFloatError;

/// 订单tick的基础数据
#[derive(Debug, Clone, Copy)]
pub struct OrderTickData {
    pub best_bid_price: u64,
    pub best_ask_price: u64,
    pub best_bid_quantity: u64,
    pub best_ask_quantity: u64,
}

/// 最佳买卖价订单数据结构
#[derive(Debug, Clone, Copy)]
pub struct OrderTick {
    pub data: OrderTickData,
    pub exchange: Exchange,
    pub symbol: TradingSymbol,
    pub timestamp: u64,
}

impl OrderTick {
    pub fn new_from_mexc(data: PushDataV3ApiWrapper) -> Result<Self, ParseFloatError> {
        if let Some(order_tick) = data.extract_book_ticker_data() {
            let best_bid_price = crate::common::utils::s2u(&order_tick.bid_price)?;
            let best_ask_price = crate::common::utils::s2u(&order_tick.ask_price)?;
            let best_bid_quantity = crate::common::utils::s2u(&order_tick.bid_quantity)?;
            let best_ask_quantity = crate::common::utils::s2u(&order_tick.ask_quantity)?;
            
            Ok(Self {
                data: OrderTickData {
                    best_bid_price,
                    best_ask_price,
                    best_bid_quantity,
                    best_ask_quantity,
                },
                exchange: Exchange::Mexc,
                symbol: data.symbol.unwrap_or(TradingSymbol::BTCUSDT.to_string()).parse().unwrap_or(TradingSymbol::BTCUSDT),
                timestamp: data.create_time.unwrap_or(0) as u64,
            })
        } else {
            Err("No book ticker data available".parse::<f64>().unwrap_err())
        }
    }

    pub fn new_from_binance(data: BookTickerData) -> Self {
        Self {
            data: OrderTickData {
                best_bid_price: crate::common::utils::f2u(data.best_bid_price),
                best_ask_price: crate::common::utils::f2u(data.best_ask_price),
                best_bid_quantity: crate::common::utils::f2u(data.best_bid_qty),
                best_ask_quantity: crate::common::utils::f2u(data.best_ask_qty),
            },
            exchange: Exchange::Binance,
            symbol: data.symbol,
            timestamp: data.transaction_time() as u64,
        }
    }

    /// 计算买卖价差
    pub fn spread(&self) -> u64 {
        if self.data.best_ask_price > self.data.best_bid_price {
            self.data.best_ask_price - self.data.best_bid_price
        } else {
            0
        }
    }

    /// 计算中间价
    pub fn mid_price(&self) -> u64 {
        (self.data.best_bid_price + self.data.best_ask_price) / 2
    }

    /// 检查是否有有效的买卖价
    pub fn is_valid(&self) -> bool {
        self.data.best_bid_price > 0 && self.data.best_ask_price > 0
    }
}

/// OrderTick 缓冲区
/// 使用 Vec 存储，支持高效的数据操作
#[derive(Clone)]
pub struct OrderTickBuffer {
    ticks: Vec<OrderTick>,
    max_size: usize,
}

impl OrderTickBuffer {
    /// 创建新的 OrderTick 缓冲区
    pub fn new(max_size: usize) -> Self {
        Self {
            ticks: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// 添加新的 OrderTick
    pub fn push_tick(&mut self, tick: OrderTick) {
        // 如果缓冲区已满，移除最旧的 tick
        if self.ticks.len() < self.max_size {
            self.ticks.push(tick);
        }

    }

    /// 获取最新的 N 个 OrderTick（返回引用切片，零拷贝，推荐使用）
    pub fn get_recent_ticks(&self, count: usize) -> &[OrderTick] {
        let start = self.ticks.len().saturating_sub(count);
        &self.ticks[start..]
    }

    /// 获取最新的 N 个 OrderTick（如果必须需要拥有数据）
    pub fn get_recent_ticks_owned(&self, count: usize) -> Vec<OrderTick> {
        let start = self.ticks.len().saturating_sub(count);
        self.ticks[start..].iter().copied().collect()
    }

    /// 获取最新的 OrderTick
    pub fn get_latest_tick(&self) -> Option<OrderTick> {
        self.ticks.last().copied()
    }

    /// 获取指定时间范围内的 OrderTick
    pub fn get_ticks_in_range(&self, start_time: u64, end_time: u64) -> Vec<OrderTick> {
        self.ticks
            .iter()
            .filter(|tick| tick.timestamp >= start_time && tick.timestamp <= end_time)
            .cloned()
            .collect()
    }

    /// 获取缓冲区大小
    pub fn len(&self) -> usize {
        self.ticks.len()
    }

    /// 检查缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        self.ticks.is_empty()
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.ticks.clear();
    }

    /// 获取所有 OrderTick（按时间顺序）
    pub fn get_all_ticks(&self) -> Vec<OrderTick> {
        self.ticks.iter().copied().collect()
    }

    /// 计算平均价差
    pub fn average_spread(&self) -> f64 {
        if self.ticks.is_empty() {
            return 0.0;
        }

        let total_spread: u64 = self.ticks.iter().map(|tick| tick.spread()).sum();
        total_spread as f64 / self.ticks.len() as f64
    }

    /// 计算平均中间价
    pub fn average_mid_price(&self) -> f64 {
        if self.ticks.is_empty() {
            return 0.0;
        }

        let total_mid_price: u64 = self.ticks.iter().map(|tick| tick.mid_price()).sum();
        total_mid_price as f64 / self.ticks.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::enums::Exchange;
    use crate::models::TradingSymbol;

    #[test]
    fn test_order_tick_buffer() {
        let mut buffer = OrderTickBuffer::new(3);

        // 添加 OrderTick
        let tick1 = OrderTick {
            data: OrderTickData {
                best_bid_price: 50000,
                best_ask_price: 50010,
                best_bid_quantity: 100,
                best_ask_quantity: 200,
            },
            exchange: Exchange::Binance,
            symbol: TradingSymbol::BTCUSDT,
            timestamp: 1000,
        };

        buffer.push_tick(tick1);
        assert_eq!(buffer.len(), 1);

        // 测试获取最新 tick
        let latest = buffer.get_latest_tick();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().data.best_bid_price, 50000);

        // 测试价差计算
        assert_eq!(latest.unwrap().spread(), 10);
        assert_eq!(latest.unwrap().mid_price(), 25005);
    }

    #[test]
    fn test_order_tick_calculations() {
        let tick = OrderTick {
            data: OrderTickData {
                best_bid_price: 50000,
                best_ask_price: 50010,
                best_bid_quantity: 100,
                best_ask_quantity: 200,
            },
            exchange: Exchange::Binance,
            symbol: TradingSymbol::BTCUSDT,
            timestamp: 1000,
        };

        assert_eq!(tick.spread(), 10);
        assert_eq!(tick.mid_price(), 25005);
        assert!(tick.is_valid());
    }
}
