use crate::dto::mexc::PushDataV3ApiWrapper;
use crate::dto::binance::websocket::BookTickerData;
use crate::models::{TradingSymbol, Exchange};
use crate::common::ts::TransactionTime;
use crate::common::utils::{f2u, s2u};
use std::num::ParseFloatError;

/// 订单tick的基础数据 (u64版本)
#[derive(Debug, Clone, Copy)]
pub struct OrderTickDataU64 {
    pub best_bid_price: u64,
    pub best_ask_price: u64,
    pub best_bid_quantity: u64,
    pub best_ask_quantity: u64,
}

/// 最佳买卖价订单数据结构 (u64版本)
#[derive(Debug, Clone, Copy)]
pub struct OrderTickU64 {
    pub data: OrderTickDataU64,
    pub exchange: Exchange,
    pub symbol: TradingSymbol,
    pub timestamp: u64,
}

impl OrderTickU64 {
    pub fn new_from_mexc(data: PushDataV3ApiWrapper) -> Result<Self, ParseFloatError> {
        if let Some(order_tick) = data.extract_book_ticker_data() {
            let best_bid_price = s2u(&order_tick.bid_price)?;
            let best_ask_price = s2u(&order_tick.ask_price)?;
            let best_bid_quantity = s2u(&order_tick.bid_quantity)?;
            let best_ask_quantity = s2u(&order_tick.ask_quantity)?;
            
            Ok(Self {
                data: OrderTickDataU64 {
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
            data: OrderTickDataU64 {
                best_bid_price: f2u(data.best_bid_price),
                best_ask_price: f2u(data.best_ask_price),
                best_bid_quantity: f2u(data.best_bid_qty),
                best_ask_quantity: f2u(data.best_ask_qty),
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

/// OrderTick 缓冲区 (u64版本)
/// 使用 Vec 存储，支持高效的数据操作
#[derive(Clone)]
pub struct OrderTickBufferU64 {
    ticks: Vec<OrderTickU64>,
    max_size: usize,
}

impl OrderTickBufferU64 {
    /// 创建新的 OrderTick 缓冲区
    pub fn new(max_size: usize) -> Self {
        Self {
            ticks: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// 添加新的 OrderTick
    pub fn push_tick(&mut self, tick: OrderTickU64) {
        // 如果缓冲区已满，移除最旧的 tick
        if self.ticks.len() < self.max_size {
            self.ticks.push(tick);
        }
    }

    /// 获取最新的 N 个 OrderTick（返回引用切片，零拷贝，推荐使用）
    pub fn get_recent_ticks(&self, count: usize) -> &[OrderTickU64] {
        let start = self.ticks.len().saturating_sub(count);
        &self.ticks[start..]
    }

    /// 获取最新的 N 个 OrderTick（如果必须需要拥有数据）
    pub fn get_recent_ticks_owned(&self, count: usize) -> Vec<OrderTickU64> {
        let start = self.ticks.len().saturating_sub(count);
        self.ticks[start..].iter().copied().collect()
    }

    /// 获取最新的 OrderTick
    pub fn get_latest_tick(&self) -> Option<OrderTickU64> {
        self.ticks.last().copied()
    }

    /// 获取指定时间范围内的 OrderTick
    pub fn get_ticks_in_range(&self, start_time: u64, end_time: u64) -> Vec<OrderTickU64> {
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
    pub fn get_all_ticks(&self) -> Vec<OrderTickU64> {
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
    fn test_order_tick_buffer_u64() {
        let mut buffer = OrderTickBufferU64::new(3);

        // 添加 OrderTick
        let tick1 = OrderTickU64 {
            data: OrderTickDataU64 {
                best_bid_price: 50000 * 100_000_000,
                best_ask_price: 50010 * 100_000_000,
                best_bid_quantity: 100 * 100_000_000,
                best_ask_quantity: 200 * 100_000_000,
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
        assert_eq!(latest.unwrap().data.best_bid_price, 50000 * 100_000_000);

        // 测试价差计算
        assert_eq!(latest.unwrap().spread(), 10 * 100_000_000);
        assert_eq!(latest.unwrap().mid_price(), 25005 * 100_000_000);
    }

    #[test]
    fn test_order_tick_calculations_u64() {
        let tick = OrderTickU64 {
            data: OrderTickDataU64 {
                best_bid_price: 50000 * 100_000_000,
                best_ask_price: 50010 * 100_000_000,
                best_bid_quantity: 100 * 100_000_000,
                best_ask_quantity: 200 * 100_000_000,
            },
            exchange: Exchange::Binance,
            symbol: TradingSymbol::BTCUSDT,
            timestamp: 1000,
        };

        assert_eq!(tick.spread(), 10 * 100_000_000);
        assert_eq!(tick.mid_price(), 25005 * 100_000_000);
        assert!(tick.is_valid());
    }
}

