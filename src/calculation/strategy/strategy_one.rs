use crate::common::config::ws_config;
use crate::exchange_api::binance::ws_manager;
use std::error;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn error::Error>> {
    let (binance_ws, mut rx) = ws_manager::create_websocket_manager().await?;
    let configs = ws_config::ConfigLoader::load_from_file("./config.rs")?;
    let kline_configs = configs.kline;
    
    for kline_config in kline_configs {
        binance_ws.start_kline(kline_config).await?
    }
    
    // 等待消息
    while let Some(_message) = rx.recv().await {
        // 处理消息
    }
    
    Ok(())
}
