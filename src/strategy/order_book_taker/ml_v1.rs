use crate::models::{StrategyContext, StrategyPosition};
use hmac::digest::typenum::Max;
use ta::indicators::{Maximum, Minimum, SimpleMovingAverage, StandardDeviation,ZScore};
use ta::{Close, High, Open};
use trusty::GradientBoostedDecisionTrees;
pub struct MLV1Strategy {
    pub cxt: StrategyContext,
    pub model: GradientBoostedDecisionTrees,
}
impl MLV1Strategy{
    pub fn new(cxt:StrategyContext,model:GradientBoostedDecisionTrees) -> Self{
        Self{
            cxt,
            model,
        }
    }
}