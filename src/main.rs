//! 主程序入口，用于启动交易策略。

// 从我们的库中导入必要的模块
use rust_system::factory::{BollingerFactory, Q1Factory};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从命令行参数获取要运行的策略
    let args: Vec<String> = env::args().collect();
    let strategy = args.get(1).map(|s| s.as_str()).unwrap_or("q1");
    
    match strategy {
        "bollinger" => {
            println!("🚀 启动布林带策略...");
            BollingerFactory::setup_logging()?;
            BollingerFactory::run_bollinger_strategy().await?;
        }
        "q1" => {
            println!("🚀 启动Q1策略...");
            Q1Factory::setup_logging()?;
            Q1Factory::run_q1_strategy().await?;
        }
        _ => {
            println!("❌ 未知的策略: {}", strategy);
            println!("支持的策略:");
            println!("  - bollinger: 布林带策略");
            println!("  - q1: Q1策略（默认）");
            return Ok(());
        }
    }
    
    Ok(())
}

