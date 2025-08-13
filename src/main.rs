//! 主程序入口，用于启动布林带策略。

// 从我们的库中导入必要的模块
use rust_system::factory::BollingerFactory;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动布林带策略...");
    
    // 设置日志系统
    BollingerFactory::setup_logging()?;
    
    // 运行布林带策略
    BollingerFactory::run_bollinger_strategy().await?;
    
    Ok(())
}

