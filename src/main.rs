use rust_system::common::config::ws_config::ConfigLoader;
use rust_system::exchange_api::binance::ws_manager::{WebSocketMessage, create_websocket_manager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 读取配置文件
    let configs = ConfigLoader::load_from_file("config.toml")?;
    let (manager, mut rx) = create_websocket_manager().await?;

    // 启动所有部分深度 WebSocket 连接
    for config in configs.partial_depth {
        manager.start_partial_depth(config).await?;
    }

    // 异步打印收到的部分深度数据
    println!("等待部分深度数据...");
    while let Some(msg) = rx.recv().await {
        if let WebSocketMessage::PartialDepth(depth) = msg {
            println!("收到部分深度数据: {:?}", depth);
        }
    }

    Ok(())
}
