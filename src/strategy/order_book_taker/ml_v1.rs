use crate::models::{Position, PositionManager, StrategyContext, StrategyPosition};
use hmac::digest::typenum::Max;
use ta::indicators::{Maximum, Minimum, SimpleMovingAverage, StandardDeviation,ZScore};
use ta::{Close, High, Open};
use trusty::GradientBoostedDecisionTrees;
pub struct MLV1Strategy {
    pub cxt: StrategyContext,
    pub model: GradientBoostedDecisionTrees,
    pub position_manager:PositionManager
}
impl MLV1Strategy{
    pub fn new(cxt:StrategyContext,model:GradientBoostedDecisionTrees,position_manager:PositionManager) -> Self{
        Self{
            cxt,
            model,
            position_manager
        }
    }
}
impl Strategy for MLV1Strategy{
    fn on_kline_update(&mut self, input: Arc<T>) -> Self::Output {
        
    }
}