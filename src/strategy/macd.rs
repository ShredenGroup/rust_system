use crate::common::ts::Strategy;
use crate::dto::binance::rest_api::KlineData;
use crate::strategy::common::{Signal, SignalType};
use crate::{
    common::config::ws_config::{ConfigLoader, KlineConfig, WebSocketBaseConfig},
    exchange_api::binance::ws_manager::{WebSocketMessage, create_websocket_manager},
};
use anyhow::Result;
use ta::indicators::hbfc_one::HbfcOne;
use ta::indicators::{SimpleMovingAverage, hbfc_one};
use ta::{Close, High, Low, Next, Open, Tbbav, Tbqav};
use tokio::signal;

pub struct MacdStrategy {
    pub ema: SimpleMovingAverage,
    pub hbfc: hbfc_one::HbfcOne,
}

impl MacdStrategy {
    pub fn new(period: usize) -> Result<Self> {
        let ema = SimpleMovingAverage::new(period)?;
        let hbfc = HbfcOne::new();
        Ok(Self { ema, hbfc })
    }
}

impl<T> Strategy<T> for MacdStrategy
where
    T: High + Low + Close + Open + Tbbav + Tbqav,
{
    type Output = Signal;
    fn on_kline_update(&mut self, input: &T) -> Signal {
        let hbfc_val = self.hbfc.next(input);
        println!("New hbfc_val{:?}", hbfc_val);
        let ema = self.ema.next(input);
        
        // 示例逻辑：根据指标值决定信号
        if hbfc_val > 0.5 {
            Signal::buy("BTCUSDT".to_string(), input.close(), 0.1)
        } else if hbfc_val < -0.5 {
            Signal::sell("BTCUSDT".to_string(), input.close(), 0.1)
        } else {
            Signal::hold()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
