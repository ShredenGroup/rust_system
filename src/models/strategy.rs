use crate::common::enums::Exchange;
use crate::models::TradingSymbol;

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

/// 策略上下文 - 所有策略共享的基础信息
/// 
/// 每个策略实例都应该包含一个 StrategyContext，提供：
/// - 交易所信息（exchange）
/// - 交易符号（symbol）
/// - 策略类型（strategy_type）
/// 
/// 注意：仓位管理已移除，因为每个策略的开仓限制不同，应由策略自身管理
#[derive(Debug, Clone)]
pub struct StrategyContext {
    pub exchange: Exchange,
    pub symbol: TradingSymbol,
    pub strategy_type: StrategyType,
}

impl StrategyContext {
    /// 创建新的策略上下文
    pub fn new(
        exchange: Exchange,
        symbol: TradingSymbol,
        strategy_type: StrategyType,
    ) -> Self {
        Self {
            exchange,
            symbol,
            strategy_type,
        }
    }
}
