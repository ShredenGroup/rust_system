use crate::models::{StrategyContext, StrategyPosition};
use hmac::digest::typenum::Max;
use ta::indicators::{Maximum, Minimum, SimpleMovingAverage, StandardDeviation,ZScore};
use ta::{Close, High, Open};