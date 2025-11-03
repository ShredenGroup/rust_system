use std::sync::Arc;
use crate::common::enums::{Exchange, PositionSide, StrategyName};
use crate::models::{TradingSymbol, PositionManager};
use crate::models::position::PositionKey;

/// 策略类型
#[derive(Copy, Clone, Debug)]
pub enum StrategyType {
    Bollinger,
    Macd,
    Q1,
}

/// 策略设置
#[derive(Copy, Clone, Debug)]
pub struct StrategySetting {
    pub strategy_type: StrategyType,
    pub risk_amount: Option<u64>,
    pub risk_percentage: Option<u64>,
    pub leverage: Option<u64>,
}

impl StrategySetting {
    pub fn new(
        strategy_type: StrategyType,
        risk_amount: Option<u64>,
        risk_percentage: Option<u64>,
        leverage: Option<u64>,
    ) -> Self {
        Self {
            strategy_type,
            risk_amount,
            risk_percentage,
            leverage,
        }
    }
}

/// 策略上下文 - 所有策略共享的基础信息和资源
/// 
/// 每个策略实例都应该包含一个 StrategyContext，提供：
/// - 策略标识信息（strategy_name, symbol, exchange）
/// - 共享的仓位管理器（PositionManager）
/// - 便捷方法用于构建 PositionKey 等
#[derive(Debug, Clone)]
pub struct StrategyContext {
    pub strategy_name: StrategyName,
    pub symbol: TradingSymbol,
    pub exchange: Exchange,
    pub position_manager: Arc<PositionManager>,
}

impl StrategyContext {
    /// 创建新的策略上下文
    pub fn new(
        strategy_name: StrategyName,
        symbol: TradingSymbol,
        exchange: Exchange,
        position_manager: Arc<PositionManager>,
    ) -> Self {
        Self {
            strategy_name,
            symbol,
            exchange,
            position_manager,
        }
    }
    
    /// 便捷方法：构建 PositionKey
    pub fn position_key(&self, side: PositionSide) -> PositionKey {
        PositionKey::new(
            self.exchange,
            self.symbol,
            self.strategy_name,
            side,
        )
    }
    
    /// 便捷方法：检查是否可以开仓
    pub fn can_open_position(&self, side: PositionSide) -> bool {
        self.position_manager.can_open_position(
            self.exchange,
            self.symbol,
            self.strategy_name,
            side,
        )
    }
    
    /// 便捷方法：获取当前仓位
    pub fn get_position(&self) -> Option<crate::models::position::Position> {
        self.position_manager.get_position(
            self.exchange,
            self.symbol,
            self.strategy_name,
        )
    }
    
    /// 便捷方法：尝试开仓（原子操作，防重复下单）
    pub fn try_open_position(
        &self,
        side: PositionSide,
        quantity: f64,
        client_order_id: Option<String>,
    ) -> Result<crate::models::position::PendingOrder, String> {
        self.position_manager.try_open_position(
            self.exchange,
            self.symbol,
            self.strategy_name,
            side,
            quantity,
            client_order_id,
        )
    }
    
    /// 便捷方法：确认订单提交
    pub fn confirm_order_submission(
        &self,
        client_order_id: &str,
        order_id: i64,
    ) -> Result<(), String> {
        self.position_manager.confirm_order_submission(
            self.exchange,
            self.symbol,
            self.strategy_name,
            client_order_id,
            order_id,
        )
    }
    
    /// 便捷方法：订单成交回调
    pub fn on_order_filled(
        &self,
        order_id: i64,
        filled_quantity: f64,
        fill_price: f64,
        side: PositionSide,
    ) -> Result<(), String> {
        self.position_manager.on_order_filled(
            self.exchange,
            self.symbol,
            self.strategy_name,
            order_id,
            filled_quantity,
            fill_price,
            side,
        )
    }
    
    /// 便捷方法：订单失败回调
    pub fn on_order_failed(
        &self,
        order_id: Option<i64>,
        client_order_id: Option<&str>,
        error_code: i32,
    ) {
        self.position_manager.on_order_failed(
            self.exchange,
            self.symbol,
            self.strategy_name,
            order_id,
            client_order_id,
            error_code,
        )
    }
}
