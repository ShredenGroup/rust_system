use std::sync::atomic::{AtomicU64, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use dashmap::DashMap;

use crate::common::enums::{Exchange, PositionSide, StrategyName};
use crate::models::TradingSymbol;

/// 无锁仓位数据（使用原子类型）
/// 
/// 设计原则：
/// - 所有字段都是原子类型，支持无锁并发读写
/// - 使用 CAS 操作保证原子性
/// - 版本号用于乐观锁检测
#[derive(Debug)]
pub struct LockFreePosition {
    // 仓位数量（使用 i64 存储，实际值 = quantity / 1e8，支持 8 位小数精度）
    quantity: AtomicI64,
    
    // 开仓价格（使用 u64 存储，实际值 = price / 1e8，支持 8 位小数精度）
    entry_price: AtomicU64,
    
    // 已实现盈亏（使用 i64 存储，实际值 = pnl / 1e8）
    realized_pnl: AtomicI64,
    
    // 最后更新时间戳（毫秒）
    last_updated_ts_ms: AtomicU64,
    
    // 版本号（用于乐观锁）
    version: AtomicU64,
}

impl LockFreePosition {
    /// 创建新的无锁仓位
    pub fn new() -> Self {
        Self {
            quantity: AtomicI64::new(0),
            entry_price: AtomicU64::new(0),
            realized_pnl: AtomicI64::new(0),
            last_updated_ts_ms: AtomicU64::new(0),
            version: AtomicU64::new(0),
        }
    }
    
    /// 原子读取仓位数量
    pub fn get_quantity(&self) -> f64 {
        self.quantity.load(Ordering::Acquire) as f64 / 1e8
    }
    
    /// 原子读取开仓价格
    pub fn get_entry_price(&self) -> f64 {
        self.entry_price.load(Ordering::Acquire) as f64 / 1e8
    }
    
    /// 原子读取已实现盈亏
    pub fn get_realized_pnl(&self) -> f64 {
        self.realized_pnl.load(Ordering::Acquire) as f64 / 1e8
    }
    
    /// 原子读取最后更新时间
    pub fn get_last_updated_ts_ms(&self) -> u64 {
        self.last_updated_ts_ms.load(Ordering::Acquire)
    }
    
    /// 原子读取版本号
    pub fn get_version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }
    
    /// 原子快照（一次性读取所有字段，保证一致性）
    pub fn snapshot(&self) -> PositionSnapshot {
        // 使用 Acquire 顺序保证读取顺序
        let version = self.version.load(Ordering::Acquire);
        let quantity = self.quantity.load(Ordering::Acquire);
        let entry_price = self.entry_price.load(Ordering::Acquire);
        let realized_pnl = self.realized_pnl.load(Ordering::Acquire);
        let last_updated = self.last_updated_ts_ms.load(Ordering::Acquire);
        
        // 验证版本号一致性（简单的一致性检查）
        let version_after = self.version.load(Ordering::Acquire);
        if version != version_after {
            // 如果版本号变化，说明在读取过程中有更新，重新读取
            return self.snapshot();
        }
        
        PositionSnapshot {
            quantity: quantity as f64 / 1e8,
            entry_price: entry_price as f64 / 1e8,
            realized_pnl: realized_pnl as f64 / 1e8,
            last_updated_ts_ms: last_updated,
            version,
        }
    }
    
    /// CAS 更新仓位数量
    fn cas_quantity(&self, expected: i64, new: i64) -> bool {
        self.quantity.compare_exchange(expected, new, Ordering::AcqRel, Ordering::Acquire).is_ok()
    }
    
    /// CAS 更新开仓价格
    fn cas_entry_price(&self, expected: u64, new: u64) -> bool {
        self.entry_price.compare_exchange(expected, new, Ordering::AcqRel, Ordering::Acquire).is_ok()
    }
    
    /// 原子增加已实现盈亏
    fn add_realized_pnl(&self, delta: i64) {
        self.realized_pnl.fetch_add(delta, Ordering::AcqRel);
    }
    
    /// 原子更新最后更新时间
    fn update_timestamp(&self, ts_ms: u64) {
        self.last_updated_ts_ms.store(ts_ms, Ordering::Release);
    }
    
    /// 原子递增版本号
    fn increment_version(&self) {
        self.version.fetch_add(1, Ordering::AcqRel);
    }
}

/// 仓位快照（用于一次性读取）
#[derive(Debug, Clone, Copy)]
pub struct PositionSnapshot {
    pub quantity: f64,
    pub entry_price: f64,
    pub realized_pnl: f64,
    pub last_updated_ts_ms: u64,
    pub version: u64,
}

/// 仓位键值（用于 SkipMap）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HftPositionKey {
    pub exchange: Exchange,
    pub symbol: TradingSymbol,
    pub strategy: StrategyName,
    pub side: PositionSide,
}

impl HftPositionKey {
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

/// 无锁高频交易仓位管理器
/// 
/// 设计特点：
/// - 使用 DashMap 作为高性能并发键值存储（读操作无锁）
/// - 所有仓位操作都是原子化的
/// - 支持 CAS 操作，避免锁竞争
/// - 适合高频交易场景（微秒级延迟）
pub struct HftPositionManager {
    // 高性能并发键值存储：Key -> Arc<LockFreePosition>
    // DashMap 的读操作是无锁的，写操作使用细粒度锁
    positions: Arc<DashMap<HftPositionKey, Arc<LockFreePosition>>>,
    
    // 全局版本号（用于检测并发修改）
    global_version: Arc<AtomicU64>,
}

impl HftPositionManager {
    /// 创建新的无锁仓位管理器
    pub fn new() -> Self {
        Self {
            positions: Arc::new(DashMap::new()),
            global_version: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// 获取或创建仓位（读操作无锁，写操作使用细粒度锁）
    pub fn get_or_create_position(
        &self,
        key: HftPositionKey,
    ) -> Arc<LockFreePosition> {
        // DashMap 的 get_or_insert 使用细粒度锁，但读操作是无锁的
        self.positions.entry(key).or_insert_with(|| {
            Arc::new(LockFreePosition::new())
        }).value().clone()
    }
    
    /// 获取仓位（只读，无锁）
    pub fn get_position(&self, key: &HftPositionKey) -> Option<Arc<LockFreePosition>> {
        self.positions.get(key).map(|entry| entry.value().clone())
    }
    
    /// 检查是否有仓位（无锁）
    pub fn has_position(&self, key: &HftPositionKey) -> bool {
        if let Some(pos) = self.get_position(key) {
            pos.get_quantity().abs() > 1e-12
        } else {
            false
        }
    }
    
    /// 获取可平仓数量（无锁）
    pub fn get_closable_quantity(&self, key: &HftPositionKey) -> f64 {
        if let Some(pos) = self.get_position(key) {
            pos.get_quantity().abs()
        } else {
            0.0
        }
    }
    
    /// 开仓/加仓（无锁 CAS 操作）
    /// 
    /// # Returns
    /// * `Ok(())` - 成功
    /// * `Err(String)` - 失败原因
    pub fn open_position(
        &self,
        key: HftPositionKey,
        quantity: f64,
        price: f64,
    ) -> Result<(), String> {
        if quantity <= 0.0 {
            return Err("数量必须大于0".to_string());
        }
        
        let pos = self.get_or_create_position(key.clone());
        let now = current_timestamp_ms();
        
        // 将 f64 转换为整数存储（8位小数精度）
        let qty_int = (quantity * 1e8) as i64;
        let price_int = (price * 1e8) as u64;
        
        // CAS 循环：原子更新仓位
        loop {
            let snapshot = pos.snapshot();
            let current_qty_int = (snapshot.quantity * 1e8) as i64;
            let current_price_int = (snapshot.entry_price * 1e8) as u64;
            
            if current_qty_int == 0 {
                // 开仓：直接设置
                let new_qty_int = if key.side == PositionSide::Long {
                    qty_int
                } else {
                    -qty_int
                };
                
                // CAS 更新数量
                if pos.cas_quantity(0, new_qty_int) {
                    // 更新价格
                    pos.cas_entry_price(0, price_int);
                    pos.update_timestamp(now);
                    pos.increment_version();
                    self.global_version.fetch_add(1, Ordering::AcqRel);
                    return Ok(());
                }
                // CAS 失败，重试
                continue;
            } else {
                // 加仓：计算加权平均价格
                let current_qty = snapshot.quantity.abs();
                let total_cost = snapshot.entry_price * current_qty + price * quantity;
                let new_qty = current_qty + quantity;
                let new_price = total_cost / new_qty;
                
                let new_qty_int = if snapshot.quantity > 0.0 {
                    (new_qty * 1e8) as i64
                } else {
                    -((new_qty * 1e8) as i64)
                };
                let new_price_int = (new_price * 1e8) as u64;
                
                // CAS 更新（需要同时更新数量和价格）
                if pos.cas_quantity(current_qty_int, new_qty_int) {
                    pos.cas_entry_price(current_price_int, new_price_int);
                    pos.update_timestamp(now);
                    pos.increment_version();
                    self.global_version.fetch_add(1, Ordering::AcqRel);
                    return Ok(());
                }
                // CAS 失败，重试
                continue;
            }
        }
    }
    
    /// 平仓/减仓（无锁 CAS 操作）
    /// 
    /// # Returns
    /// * `Ok(f64)` - 成功，返回已平仓数量
    /// * `Err(String)` - 失败原因
    pub fn close_position(
        &self,
        key: &HftPositionKey,
        quantity: f64,
        price: f64,
    ) -> Result<f64, String> {
        if quantity <= 0.0 {
            return Err("数量必须大于0".to_string());
        }
        
        let pos = match self.get_position(key) {
            Some(p) => p,
            None => return Err("仓位不存在".to_string()),
        };
        
        let now = current_timestamp_ms();
        
        // CAS 循环：原子更新仓位
        loop {
            let snapshot = pos.snapshot();
            let current_qty = snapshot.quantity;
            
            if current_qty.abs() < 1e-12 {
                return Err("仓位为空".to_string());
            }
            
            // 计算可平仓数量
            let qty_to_close = quantity.min(current_qty.abs());
            let remaining_qty = current_qty.abs() - qty_to_close;
            
            // 计算盈亏
            let pnl_per_unit = match key.side {
                PositionSide::Long => price - snapshot.entry_price,
                PositionSide::Short => snapshot.entry_price - price,
                PositionSide::NoPosition => 0.0,
            };
            let pnl_delta = (pnl_per_unit * qty_to_close * 1e8) as i64;
            
            // 更新数量
            let current_qty_int = (current_qty * 1e8) as i64;
            let new_qty_int = if current_qty > 0.0 {
                (remaining_qty * 1e8) as i64
            } else {
                -((remaining_qty * 1e8) as i64)
            };
            
            // CAS 更新
            if pos.cas_quantity(current_qty_int, new_qty_int) {
                // 更新盈亏
                pos.add_realized_pnl(pnl_delta);
                pos.update_timestamp(now);
                pos.increment_version();
                self.global_version.fetch_add(1, Ordering::AcqRel);
                
                // 如果仓位归零，可以选择移除（可选）
                if remaining_qty < 1e-12 {
                    self.positions.remove(key);
                }
                
                return Ok(qty_to_close);
            }
            // CAS 失败，重试
        }
    }
    
    /// 获取仓位快照（无锁）
    pub fn get_position_snapshot(
        &self,
        key: &HftPositionKey,
    ) -> Option<PositionSnapshot> {
        self.get_position(key).map(|pos| pos.snapshot())
    }
    
    /// 获取全局版本号（用于检测并发修改）
    pub fn get_global_version(&self) -> u64 {
        self.global_version.load(Ordering::Acquire)
    }
    
    /// 清除仓位（无锁）
    pub fn clear_position(&self, key: &HftPositionKey) {
        self.positions.remove(key);
        self.global_version.fetch_add(1, Ordering::AcqRel);
    }
}

impl Default for HftPositionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取当前时间戳（毫秒）
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lock_free_position() {
        let pos = LockFreePosition::new();
        
        // 测试基本读写
        assert_eq!(pos.get_quantity(), 0.0);
        assert_eq!(pos.get_entry_price(), 0.0);
        assert_eq!(pos.get_realized_pnl(), 0.0);
        
        // 测试快照
        let snapshot = pos.snapshot();
        assert_eq!(snapshot.quantity, 0.0);
    }
    
    #[test]
    fn test_hft_position_manager() {
        let manager = HftPositionManager::new();
        let key = HftPositionKey::new(
            Exchange::Binance,
            TradingSymbol::BTCUSDT,
            StrategyName::MACD,
            PositionSide::Long,
        );
        
        // 测试开仓
        assert!(manager.open_position(key.clone(), 1.0, 50000.0).is_ok());
        
        // 测试读取
        let snapshot = manager.get_position_snapshot(&key).unwrap();
        assert_eq!(snapshot.quantity, 1.0);
        assert_eq!(snapshot.entry_price, 50000.0);
        
        // 测试平仓
        assert!(manager.close_position(&key, 0.5, 51000.0).is_ok());
        
        let snapshot = manager.get_position_snapshot(&key).unwrap();
        assert_eq!(snapshot.quantity, 0.5);
        assert!(snapshot.realized_pnl > 0.0); // 应该有盈利
    }
}

