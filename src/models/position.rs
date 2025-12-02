use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::enums::{Exchange, PositionSide, StrategyName};
use crate::models::{TradingSymbol, OrderStatus, Order};

/// 仓位信息
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub entry_price: f64,
    pub quantity: f64,
    pub realized_pnl: Option<f64>,
    pub unrealized_pnl: f64,
    pub client_timestamp: u64,
    pub created_timestamp: u64,
    pub last_updated_ts_ms: u64,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            entry_price: 0.0,
            quantity: 0.0,
            realized_pnl: None,
            unrealized_pnl: 0.0,
            client_timestamp: 0,
            created_timestamp: 0,
            last_updated_ts_ms: 0,
        }
    }
}

/// 策略仓位状态（包含待处理订单）
#[derive(Debug, Clone)]
pub struct StrategyPosition {
    pub position: Position,
    pub pending_orders: Vec<Order>,  // 待处理的订单列表
    pub last_updated_ts_ms: u64,
}

impl Default for StrategyPosition {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Self {
            position: Position::default(),
            pending_orders: Vec::new(),
            last_updated_ts_ms: now,
        }
    }
}

/// 仓位键值
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PositionKey {
    pub exchange: Exchange,
    pub symbol: TradingSymbol,
    pub strategy: StrategyName,
    pub side: PositionSide,
}

impl PositionKey {
    pub fn new(
        exchange: Exchange,
        symbol: TradingSymbol,
        strategy: StrategyName,
        side: PositionSide,
    ) -> Self {
        Self {
            exchange,
            symbol,
            strategy,
            side,
        }
    }
}

/// 仓位管理器（增强版，支持防重复下单）
#[derive(Debug, Clone)]
pub struct PositionManager {
    // 仓位数据: (Exchange, Symbol, Strategy) -> StrategyPosition
    inner: Arc<DashMap<(Exchange, TradingSymbol, StrategyName), StrategyPosition>>,
    
    // 版本号（用于乐观锁）
    version: Arc<AtomicU64>,
}

impl PositionManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
            version: Arc::new(AtomicU64::new(0)),
        }
    }
    
    pub fn shared(&self) -> Arc<DashMap<(Exchange, TradingSymbol, StrategyName), StrategyPosition>> {
        Arc::clone(&self.inner)
    }
    
    /// 检查是否可以开仓（原子操作）
    /// 
    /// # Returns
    /// * `Ok(Order)` - 如果可以开仓，返回待处理订单
    /// * `Err(String)` - 如果不能开仓，返回原因
    pub fn try_open_position(
        &self,
        exchange: Exchange,
        symbol: TradingSymbol,
        strategy: StrategyName,
        side: PositionSide,
        quantity: f64,
        client_order_id: Option<String>,
    ) -> Result<Order, String> {
        let key = (exchange, symbol, strategy);
        
        // 原子操作：检查并更新
        let mut strategy_pos = self.inner
            .entry(key)
            .or_insert_with(|| StrategyPosition::default());
        
        // 1. 检查是否有持仓
        if strategy_pos.position.quantity.abs() > 1e-12 {
            return Err(format!(
                "已有持仓: {:.4}, 方向: {:?}",
                strategy_pos.position.quantity,
                side
            ));
        }
        
        // 2. 清理过期的订单
        strategy_pos.pending_orders.retain(|order| !order.is_expired());
        
        // 3. 检查是否有待处理的同方向订单
        let has_pending = strategy_pos.pending_orders.iter().any(|order| {
            matches!(order.internal.status, OrderStatus::Pending | OrderStatus::Submitted) &&
            order.internal.side == side
        });
        
        if has_pending {
            return Err("有待处理的开仓订单，请等待".to_string());
        }
        
        // 4. 创建待处理订单并记录
        let pending_order = Order::new(exchange, symbol, side, quantity, client_order_id);
        strategy_pos.pending_orders.push(pending_order.clone());
        
        // 5. 更新版本号（乐观锁）
        self.version.fetch_add(1, Ordering::Release);
        
        Ok(pending_order)
    }
    
    /// 确认订单提交（API 返回订单ID）
    pub fn confirm_order_submission(
        &self,
        exchange: Exchange,
        symbol: TradingSymbol,
        strategy: StrategyName,
        client_order_id: &str,
        order_id: i64,
    ) -> Result<(), String> {
        let key = (exchange, symbol, strategy);
        
        if let Some(mut strategy_pos) = self.inner.get_mut(&key) {
            // 查找对应的待处理订单
            for order in strategy_pos.pending_orders.iter_mut() {
                if order.client_order_id == client_order_id && order.internal.status == OrderStatus::Pending {
                    order.order_id = Some(order_id.to_string());
                    order.internal.status = OrderStatus::Submitted;
                    return Ok(());
                }
            }
            Err("未找到对应的待处理订单".to_string())
        } else {
            Err("未找到策略仓位记录".to_string())
        }
    }
    
    /// 订单成交回调（更新仓位）
    pub fn on_order_filled(
        &self,
        exchange: Exchange,
        symbol: TradingSymbol,
        strategy: StrategyName,
        order_id: i64,
        filled_quantity: f64,
        fill_price: f64,
        side: PositionSide,
    ) -> Result<(), String> {
        let key = (exchange, symbol, strategy);
        
        if let Some(mut strategy_pos) = self.inner.get_mut(&key) {
            // 1. 找到对应的订单并标记为已成交
            let mut found = false;
            for order in strategy_pos.pending_orders.iter_mut() {
                if order.order_id == Some(order_id.to_string()) {
                    order.internal.status = OrderStatus::Filled;
                    found = true;
                    break;
                }
            }
            
            if !found {
                return Err(format!("未找到订单ID: {}", order_id));
            }
            
            // 2. 更新仓位
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            let pos = &mut strategy_pos.position;
            if pos.quantity == 0.0 {
                // 开仓
                pos.entry_price = fill_price;
                pos.quantity = filled_quantity * if side == PositionSide::Long { 1.0 } else { -1.0 };
            } else {
                // 加仓或减仓
                if (pos.quantity > 0.0 && side == PositionSide::Long) ||
                   (pos.quantity < 0.0 && side == PositionSide::Short) {
                    // 加仓：加权平均成本
                    let total_cost = pos.entry_price * pos.quantity.abs() + fill_price * filled_quantity;
                    let total_qty = pos.quantity.abs() + filled_quantity;
                    pos.entry_price = total_cost / total_qty;
                    pos.quantity = pos.quantity.abs() + filled_quantity;
                    if side == PositionSide::Short {
                        pos.quantity = -pos.quantity;
                    }
                } else {
                    // 减仓：计算盈亏
                    let qty_to_reduce = filled_quantity.min(pos.quantity.abs());
                    let pnl_per_unit = match side {
                        PositionSide::Long => fill_price - pos.entry_price,
                        PositionSide::Short => pos.entry_price - fill_price,
                        PositionSide::NoPosition => 0.0,
                    };
                    let pnl_to_add = pnl_per_unit * qty_to_reduce;
                    pos.realized_pnl = Some(pos.realized_pnl.unwrap_or(0.0) + pnl_to_add);
                    pos.quantity = (pos.quantity.abs() - qty_to_reduce) * if pos.quantity > 0.0 { 1.0 } else { -1.0 };
                }
            }
            
            pos.last_updated_ts_ms = now;
            strategy_pos.last_updated_ts_ms = now;
            
            // 3. 清理已成交的订单（可选：保留历史记录）
            strategy_pos.pending_orders.retain(|order| {
                !matches!(order.internal.status, OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Failed)
            });
            
            // 4. 更新版本号
            self.version.fetch_add(1, Ordering::Release);
            
            Ok(())
        } else {
            Err("未找到策略仓位记录".to_string())
        }
    }
    
    /// 订单失败回调
    pub fn on_order_failed(
        &self,
        exchange: Exchange,
        symbol: TradingSymbol,
        strategy: StrategyName,
        order_id: Option<i64>,
        client_order_id: Option<&str>,
        _error_code: i32,
    ) {
        let key = (exchange, symbol, strategy);
        
        if let Some(mut strategy_pos) = self.inner.get_mut(&key) {
            // 标记订单为失败
            for order in strategy_pos.pending_orders.iter_mut() {
                let match_order = if let Some(oid) = order_id {
                    order.order_id == Some(oid.to_string())
                } else if let Some(cid) = client_order_id {
                    order.client_order_id == cid
                } else {
                    false
                };
                
                if match_order {
                    order.internal.status = OrderStatus::Failed;
                    break;
                }
            }
            
            // 清理失败订单
            strategy_pos.pending_orders.retain(|order| {
                !matches!(order.internal.status, OrderStatus::Failed | OrderStatus::Cancelled)
            });
            
            self.version.fetch_add(1, Ordering::Release);
        }
    }
    
    /// 检查是否有可用仓位（考虑待处理订单）
    pub fn can_open_position(
        &self,
        exchange: Exchange,
        symbol: TradingSymbol,
        strategy: StrategyName,
        side: PositionSide,
    ) -> bool {
        let key = (exchange, symbol, strategy);
        
        if let Some(strategy_pos) = self.inner.get(&key) {
            // 1. 检查是否有持仓
            if strategy_pos.position.quantity.abs() > 1e-12 {
                return false;
            }
            
            // 2. 清理过期订单
            // (注意：这里不能修改，所以只是检查)
            
            // 3. 检查是否有待处理的同方向订单
            let has_pending = strategy_pos.pending_orders.iter().any(|order| {
                !order.is_expired() &&
                matches!(order.internal.status, OrderStatus::Pending | OrderStatus::Submitted) &&
                order.internal.side == side
            });
            
            !has_pending
        } else {
            true  // 没有记录，可以开仓
        }
    }
    
    /// 获取当前仓位（排除待处理订单的影响）
    pub fn get_position(
        &self,
        exchange: Exchange,
        symbol: TradingSymbol,
        strategy: StrategyName,
    ) -> Option<Position> {
        self.inner.get(&(exchange, symbol, strategy))
            .map(|entry| entry.position)
    }
    
    /// 获取策略仓位状态（包含待处理订单）
    pub fn get_strategy_position(
        &self,
        exchange: Exchange,
        symbol: TradingSymbol,
        strategy: StrategyName,
    ) -> Option<StrategyPosition> {
        self.inner.get(&(exchange, symbol, strategy))
            .map(|entry| entry.clone())
    }
    
    /// 获取版本号（用于乐观锁）
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }
    
    /// 硬重置仓位（用于异常恢复）
    pub fn clear_position(
        &self,
        exchange: Exchange,
        symbol: TradingSymbol,
        strategy: StrategyName,
    ) {
        self.inner.remove(&(exchange, symbol, strategy));
        self.version.fetch_add(1, Ordering::Release);
    }
}

impl Default for PositionManager {
    fn default() -> Self {
        Self::new()
    }
}
