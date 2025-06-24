//! 主程序入口，用于启动和集成一个简单的事件驱动策略。

// 从我们的库中导入必要的模块
use rust_system::{
    exchange_api::binance::ws_manager::{create_websocket_manager, WebSocketMessage},
    common::config::ws_config::ConfigLoader,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("主程序启动");
    Ok(())
} 