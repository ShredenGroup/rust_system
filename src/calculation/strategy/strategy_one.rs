use crate::exchange_api::binance::ws_manager::{create_websocket_manager, WebSocketMessage};
use crate::exchange_api::binance::ws::KlineData;
use std::collections::VecDeque;
use anyhow::Result;

pub struct StrategyOne {
    kline_buffer: VecDeque<KlineData>,
    buffer_size: usize,
}

impl StrategyOne {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            kline_buffer: VecDeque::with_capacity(buffer_size),
            buffer_size,
        }
    }

    pub fn on_kline(&mut self, kline_data: KlineData) {
        if self.kline_buffer.len() == self.buffer_size {
            self.kline_buffer.pop_front();
        }
        self.kline_buffer.push_back(kline_data);

        // 当ringbuffer更新的时候迅速触发策略
        self.trigger_strategy();
    }

    fn trigger_strategy(&self) {
        // 在这里实现策略逻辑
        // 目前只是打印信息
        println!("strategy triggered. Buffer size: {}", self.kline_buffer.len());
        if let Some(latest_kline) = self.kline_buffer.back() {
            println!("Latest kline: {:?}", latest_kline);
        }
    }
}

pub async fn run_strategy_one() -> Result<()> {
    let (manager, mut rx) = create_websocket_manager().await?;
    
    // 从配置文件启动连接
    // Make sure you have a `config.toml` file in the root of the project
    manager.start_from_config("config.toml").await?;

    let mut strategy = StrategyOne::new(100);
    
    // 处理消息
    while let Some(message) = rx.recv().await {
        match message {
            WebSocketMessage::Kline(kline_data) => {
                strategy.on_kline(kline_data);
            },
            _ => {
                // 忽略其他类型的消息
            }
        }
    }

    Ok(())
}
