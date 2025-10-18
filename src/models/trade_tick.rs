use crate::common::enums::Exchange;
use crate::models::{TradingSymbol, Side};
use std::collections::VecDeque;
use std::sync::Arc;

/// 逐笔交易数据结构
#[derive(Debug, Clone)]
pub struct TradeTick {
    pub trade_id: u64,
    pub symbol: TradingSymbol,
    pub price: f64,
    pub quantity: f64,
    pub side: Side,
    pub timestamp: i64,
    pub exchange: Exchange,
}

/// 逐笔交易缓冲区
/// 使用 VecDeque 存储，支持高效的双端操作
pub struct TradeTickBuffer {
    trades: VecDeque<Arc<TradeTick>>,
    max_size: usize,
}

impl TradeTickBuffer {
    /// 创建新的交易缓冲区
    pub fn new(max_size: usize) -> Self {
        Self {
            trades: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// 添加新的交易
    pub fn push_trade(&mut self, trade: TradeTick) {
        // 如果缓冲区已满，移除最旧的交易
        if self.trades.len() >= self.max_size {
            self.trades.pop_front();
        }
        
        self.trades.push_back(Arc::new(trade));
    }

    /// 获取最新的 N 笔交易
    pub fn get_recent_trades(&self, count: usize) -> Vec<Arc<TradeTick>> {
        self.trades
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// 获取指定时间范围内的交易
    pub fn get_trades_in_range(&self, start_time: i64, end_time: i64) -> Vec<Arc<TradeTick>> {
        self.trades
            .iter()
            .filter(|trade| trade.timestamp >= start_time && trade.timestamp <= end_time)
            .cloned()
            .collect()
    }

    /// 获取缓冲区大小
    pub fn len(&self) -> usize {
        self.trades.len()
    }

    /// 检查缓冲区是否为空
    pub fn is_empty(&self) -> bool {
        self.trades.is_empty()
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.trades.clear();
    }

    /// 获取所有交易（按时间顺序）
    pub fn get_all_trades(&self) -> Vec<Arc<TradeTick>> {
        self.trades.iter().cloned().collect()
    }
}

/// 为 TradeTick 实现一些便利方法
impl TradeTick {
    /// 计算交易金额
    pub fn amount(&self) -> f64 {
        self.price * self.quantity
    }

    /// 检查是否为买单
    pub fn is_buy(&self) -> bool {
        matches!(self.side, Side::Buy)
    }

    /// 检查是否为卖单
    pub fn is_sell(&self) -> bool {
        matches!(self.side, Side::Sell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::enums::Exchange;
    use crate::models::{TradingSymbol, Side};

    #[test]
    fn test_trade_tick_buffer() {
        let mut buffer = TradeTickBuffer::new(3);
        
        // 添加交易
        let trade1 = TradeTick {
            trade_id: 1,
            symbol: TradingSymbol::BTCUSDT,
            price: 50000.0,
            quantity: 0.1,
            side: Side::Buy,
            timestamp: 1000,
            exchange: Exchange::Binance,
        };
        
        buffer.push_trade(trade1);
        assert_eq!(buffer.len(), 1);
        
        // 测试获取最近交易
        let recent = buffer.get_recent_trades(1);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].trade_id, 1);
    }
}
