use crate::models::{StrategyContext, StrategyPosition};
use hmac::digest::typenum::Max;
use ta::indicators::{Maximum, Minimum, SimpleMovingAverage, StandardDeviation,ZScore};
use ta::{Close, High, Open};
pub struct ZYStrategy {
    pub cxt: StrategyContext,
    pub position: StrategyPosition,
    pub close_std: StandardDeviation,
    pub close_std_mean: SimpleMovingAverage,
    pub close_std_std: StandardDeviation,
    pub close_std_z_score: ZScore,
    pub high_max: Maximum,
    pub low_min: Minimum,
    pub std_thredshold:f64,
    pub 
}
