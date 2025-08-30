use crate::common::ts::IsClosed;
use crate::common::ts::Strategy;
use crate::strategy::common::{Signal, SignalType};
use crate::{
    common::config::ws_config::{KlineConfig, WebSocketBaseConfig},
    exchange_api::binance::ws_manager::{WebSocketMessage, create_websocket_manager},
};
use anyhow::Result;
use std::sync::Arc;
use ta::indicators::hbfc_one::HbfcOne;
use ta::indicators::{SimpleMovingAverage, hbfc_one};
use ta::{Close, High, Low, Next, Open, Tbbav, Tbqav};
#[derive(Clone)]
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

// 为引用类型实现 Strategy trait
impl<T> Strategy<&T> for MacdStrategy
where
    T: High + Low + Close + Open + Tbbav + Tbqav,
{
    type Output = Signal;

    fn on_kline_update(&mut self, input: &T) -> Signal {
        let hbfc_val = self.hbfc.next(input);
        let _ema = self.ema.next(input);
        println!("New hbfc_val{:?}", hbfc_val);
        println!("New ema_val{:?}", _ema);

        // 示例逻辑：根据指标值决定信号
        if hbfc_val.is_some() && hbfc_val.unwrap() > 0.5 {
            Signal::buy("BTCUSDT".to_string(), input.close(), 0.1)
        } else if hbfc_val.is_some() && hbfc_val.unwrap() < -0.5 {
            Signal::sell("BTCUSDT".to_string(), input.close(), 0.1)
        } else {
            Signal {
                signal_type: None,
                symbol: "BTCUSDT".to_string(),
                price: input.close(),
                quantity: 0.0,
                timestamp: chrono::Utc::now().timestamp(),
            }
        }
    }
    fn name(&self) -> String {
        "macd".to_string()
    }
}

// 为 Arc<T> 类型实现 Strategy trait
impl<T> Strategy<Arc<T>> for MacdStrategy
where
    T: High + Low + Close + Open + Tbbav + Tbqav + Send + Sync + 'static,
{
    type Output = Signal;

    fn on_kline_update(&mut self, input: Arc<T>) -> Signal {
        let hbfc_val = self.hbfc.next(input.as_ref());
        let _ema = self.ema.next(input.as_ref());

        println!("New hbfc_val{:?}", hbfc_val);
        println!("New ema_val{:?}", _ema);

        // 示例逻辑：根据指标值决定信号
        if hbfc_val.is_some() && hbfc_val.unwrap() > 0.5 {
            Signal::buy("BTCUSDT".to_string(), input.close(), 0.1)
        } else if hbfc_val.is_some() && hbfc_val.unwrap() < -0.5 {
            Signal::sell("BTCUSDT".to_string(), input.close(), 0.1)
        } else {
            Signal {
                signal_type: None,
                symbol: "BTCUSDT".to_string(),
                price: input.close(),
                quantity: 0.0,
                timestamp: chrono::Utc::now().timestamp(),
            }
        }
    }
    fn name(&self) -> String {
        "macd".to_string()
    }
}
