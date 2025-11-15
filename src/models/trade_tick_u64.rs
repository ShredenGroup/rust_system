use crate::dto::binance::websocket::BinanceTradeData;
use crate::models::{Exchange, Side, TradingSymbol};
use crate::common::utils::f2u;


/// 逐笔交易数据结构 (u64版本)
#[derive(Debug, Clone, Copy)]
pub struct TradeTickU64 {
    pub trade_id: u64,
    pub symbol: TradingSymbol,
    pub price: u64,
    pub quantity: u64,
    pub side: Side,
    pub timestamp: u64,
    pub exchange: Exchange,
}


/// 逐笔交易缓冲区 (u64版本)
/// 使用 VecDeque 存储，支持高效的双端操作
#[derive(Clone)]
pub struct TradeTickBufferU64 {
    trades: Vec<TradeTickU64>,
    max_size: usize,
}

impl TradeTickBufferU64 {
    /// 创建新的交易缓冲区
    pub fn new(max_size: usize) -> Self {
        Self {
            trades: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// 添加新的交易（简单追加，不删除旧数据）
    pub fn push_trade(&mut self, trade: TradeTickU64) {
        if self.trades.len() < self.max_size {
            self.trades.push(trade);
        }
    }

    /// 获取最新的 N 笔交易（返回引用切片，零拷贝，推荐使用）
    pub fn get_recent_trades(&self, count: usize) -> &[TradeTickU64] {
        let start = self.trades.len().saturating_sub(count);
        &self.trades[start..]
    }

    /// 获取最新的 N 笔交易（如果必须需要拥有数据）
    pub fn get_recent_trades_owned(&self, count: usize) -> Vec<TradeTickU64> {
        let start = self.trades.len().saturating_sub(count);
        self.trades[start..].iter().copied().collect()
    }

    /// 获取最新的 N 笔交易（返回迭代器，延迟求值）
    pub fn recent_trades_iter(&self, count: usize) -> impl Iterator<Item = &TradeTickU64> {
        let start = self.trades.len().saturating_sub(count);
        self.trades[start..].iter()
    }

    /// 获取指定时间范围内的交易
    pub fn get_trades_in_range(&self, start_time: u64, end_time: u64) -> Vec<TradeTickU64> {
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
    pub fn get_all_trades(&self) -> Vec<TradeTickU64> {
        self.trades.iter().cloned().collect()
    }

    /// 克隆整个缓冲区（用于快照）
    pub fn clone_buffer(&self) -> TradeTickBufferU64 {
        TradeTickBufferU64 {
            trades: self.trades.clone(),
            max_size: self.max_size,
        }
    }
}

/// 为 TradeTickU64 实现一些便利方法
impl TradeTickU64 {
    /// 从 BinanceTradeData 创建 TradeTickU64
    pub fn new_from_binance(data: BinanceTradeData) -> Self {
        Self {
            trade_id: data.trade_id,
            symbol: data.symbol,
            price: f2u(data.price),
            quantity: f2u(data.quantity),
            side: if data.is_buy() { Side::Buy } else { Side::Sell },
            timestamp: data.trade_time as u64,
            exchange: Exchange::Binance,
        }
    }

    /// 计算交易金额
    pub fn amount(&self) -> u64 {
        // 注意：这里需要先除以 PARSE_DECIMAL 再相乘，避免溢出
        // 或者使用更大的整数类型
        // 简化版本：直接相乘，但需要注意精度
        (self.price as u128 * self.quantity as u128 / 100_000_000) as u64
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
    use crate::models::{Side, TradingSymbol};

    #[test]
    fn test_trade_tick_buffer_u64() {
        let mut buffer = TradeTickBufferU64::new(3);

        // 添加交易
        let trade1 = TradeTickU64 {
            trade_id: 1,
            symbol: TradingSymbol::BTCUSDT,
            price: 50000 * 100_000_000,
            quantity: (0.1 * 100_000_000.0) as u64,
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

