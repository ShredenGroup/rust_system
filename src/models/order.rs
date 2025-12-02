use super::Exchange;
use crate::common::enums::PositionSide;
use crate::common::utils::get_timestamp_ms;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Limit,
    Market,
    StopMarket,
    StopLimit,
}
/// 订单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderStatus {
    Pending,          // 已发出，等待API响应
    Submitted,        // API已接收，等待成交
    Filled,           // 已成交
    Cancelled,        // 已取消
    Failed,           // 失败
    PartialFilled,    // 部分成交
    PartialCancelled, // 部分取消
}

/// 订单
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct OrderInternal {
    pub price: Option<f64>,
    pub order_type: OrderType,
    pub side: PositionSide,
    pub quantity: f64,
    pub amount: f64,
    pub filled_quantity: f64,
    pub filled_amount: f64,
    pub remain_amount: f64,
    pub remain_quantity: f64,
    pub avg_price: f64,
    pub status: OrderStatus,
    pub client_created_at: u64,
    pub created_at: u64,         // 创建时间戳（毫秒）
    pub updated_at: u64,         // 更新时间戳（毫秒）
    pub expires_at: Option<u64>, // 过期时间（可选）
}
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub strategy_id: u8,
    pub client_order_id: String,  // 客户端订单ID（唯一标识）
    pub order_id: Option<String>, // 交易所返回的订单ID
    pub internal: OrderInternal,
    pub exchange: Exchange,
}

impl Order {
    /// 创建新订单
    pub fn new(
        exchange: Exchange,
        symbol: crate::models::TradingSymbol,
        side: PositionSide,
        quantity: f64,
        client_order_id: Option<String>,
    ) -> Self {
        let now = get_timestamp_ms();
        let client_order_id = client_order_id.unwrap_or_else(|| {
            format!("{:?}_{:?}_{}", exchange, symbol, now)
        });
        
        Self {
            strategy_id: 0, // 默认策略ID，需要外部设置
            client_order_id,
            order_id: None,
            exchange,
            internal: OrderInternal {
                price: None,
                order_type: OrderType::Market,
                side,
                quantity,
                amount: 0.0,
                filled_quantity: 0.0,
                filled_amount: 0.0,
                remain_amount: 0.0,
                remain_quantity: quantity,
                avg_price: 0.0,
                status: OrderStatus::Pending,
                client_created_at: now,
                created_at: now,
                updated_at: now,
                expires_at: None,
            },
        }
    }
    
    /// 检查订单是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.internal.expires_at {
            get_timestamp_ms() > expires_at
        } else {
            false
        }
    }
}
/// 订单管理器（支持多级索引）
///
/// 设计说明：
/// - `orders_by_strategy`: 策略层使用，快速查询本策略的订单（高频访问）
/// - `all_orders`: 风控层/监控层使用，全局订单查询和状态同步（通过 client_order_id）
pub struct OrderManager {
    // 策略层索引：按策略ID分组，供策略层快速查询自己的订单
    // 使用场景：策略查询自己的订单状态、判断是否有待处理订单
    orders_by_strategy: HashMap<u8, Vec<Order>>,

    // 全局订单索引：所有订单的全局视图，供风控层/监控层使用
    // 使用场景：
    // 1. 风控层：跨策略风险检查、总仓位限制、异常订单检测
    // 2. 监控层：全局订单统计、订单状态同步（通过 client_order_id 快速查找）
    // 3. WebSocket 订单更新：通过 client_order_id 更新订单状态
    all_orders: Arc<DashMap<String, Order>>,
}

impl OrderManager {
    pub fn new() -> Self {
        Self {
            orders_by_strategy: HashMap::new(),
            all_orders: Arc::new(DashMap::new()),
        }
    }

    /// 添加订单
    ///
    /// 同时写入两个索引：
    /// 1. `orders_by_strategy`: 供策略层快速查询
    /// 2. `all_orders`: 供风控层/监控层全局查询和状态同步
    ///
    /// 性能说明：DashMap 是线程安全的，insert 操作很快（内部锁分段，通常 < 100ns）
    pub fn add_order(&mut self, order: Order) {
        let client_order_id = order.client_order_id.clone();
        let strategy_id = order.strategy_id;

        // 1. 快速同步写入 orders_by_strategy（主线程追求速度）
        self.orders_by_strategy
            .entry(strategy_id)
            .or_insert_with(Vec::new)
            .push(order.clone());

        // 2. 同步写入 all_orders
        // DashMap::insert 是线程安全的，使用内部锁分段，开销很小（通常 < 100ns）
        // 比创建异步任务的开销（通常 > 1μs）要小得多
        self.all_orders.insert(client_order_id, order);
    }

    // ========== 策略层接口（高频访问） ==========

    /// 获取指定策略的所有订单（策略层使用）
    ///
    /// 从 `orders_by_strategy` 查询，性能最优（O(1) 哈希查找）
    /// 使用场景：策略查询自己的订单状态、判断是否有待处理订单
    pub fn get_orders_by_strategy(&self, strategy_id: u8) -> Vec<Order> {
        self.orders_by_strategy
            .get(&strategy_id)
            .map(|orders| orders.clone())
            .unwrap_or_default()
    }

    /// 获取指定策略的活跃订单（Pending/Submitted 状态）
    ///
    /// 策略层使用，用于判断是否可以开新仓
    pub fn get_active_orders_by_strategy(&self, strategy_id: u8) -> Vec<Order> {
        self.orders_by_strategy
            .get(&strategy_id)
            .map(|orders| {
                orders
                    .iter()
                    .filter(|order| {
                        matches!(
                            order.internal.status,
                            OrderStatus::Pending
                                | OrderStatus::Submitted
                                | OrderStatus::PartialFilled
                        )
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    // ========== 风控层/监控层接口（全局查询） ==========

    /// 通过 client_order_id 查询订单（风控层/监控层使用）
    ///
    /// 从 `all_orders` 查询，用于：
    /// - WebSocket 订单更新时通过 client_order_id 快速定位订单
    /// - 风控层检查特定订单的状态
    pub fn get_order_by_id(&self, client_order_id: &str) -> Option<Order> {
        self.all_orders
            .get(client_order_id)
            .map(|entry| entry.value().clone())
    }

    /// 更新订单状态（订单状态同步使用）
    ///
    /// 当收到 WebSocket 订单更新时，通过 client_order_id 更新订单状态
    /// 同时更新两个索引，保持数据一致性
    pub fn update_order_status(&mut self, client_order_id: &str, status: OrderStatus) -> bool {
        // 1. 更新全局索引
        if let Some(mut entry) = self.all_orders.get_mut(client_order_id) {
            entry.internal.status = status;
            entry.internal.updated_at = get_timestamp_ms();

            // 2. 更新策略索引
            let strategy_id = entry.strategy_id;
            if let Some(orders) = self.orders_by_strategy.get_mut(&strategy_id) {
                if let Some(order) = orders
                    .iter_mut()
                    .find(|o| o.client_order_id == client_order_id)
                {
                    order.internal.status = status;
                    order.internal.updated_at = entry.internal.updated_at;
                }
            }

            true
        } else {
            false
        }
    }

    /// 获取所有订单（监控层使用）
    ///
    /// 用于全局订单统计、监控面板展示
    pub fn get_all_orders(&self) -> Vec<Order> {
        self.all_orders
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// 获取所有订单数量（风控层使用）
    ///
    /// 用于风控检查：总订单数限制、异常检测
    pub fn total_orders(&self) -> usize {
        self.all_orders.len()
    }

    /// 获取活跃订单数量（风控层使用）
    ///
    /// 用于风控检查：活跃订单数限制
    pub fn active_orders_count(&self) -> usize {
        self.all_orders
            .iter()
            .filter(|entry| {
                matches!(
                    entry.value().internal.status,
                    OrderStatus::Pending | OrderStatus::Submitted | OrderStatus::PartialFilled
                )
            })
            .count()
    }
}

impl Default for OrderManager {
    fn default() -> Self {
        Self::new()
    }
}
