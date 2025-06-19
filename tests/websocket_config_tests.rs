use std::time::Duration;
use rust_system::exchange_api::binance::ws_manager::{
    BaseConfig,
    MarkPriceConfig,
    KlineConfig,
    DepthConfig,
    TradeConfig,
    TickerConfig,
    AllTickerConfig,
    WebSocketDataType,
};

#[test]
fn test_base_config_creation() {
    let config = BaseConfig::new("test_conn", "btcusdt");
    
    assert_eq!(config.connection_id, "test_conn");
    assert_eq!(config.symbols, vec!["btcusdt"]);
    assert_eq!(config.auto_reconnect, true);
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.retry_delay, Duration::from_secs(5));
    assert_eq!(config.connection_timeout, Duration::from_secs(10));
    assert_eq!(config.message_timeout, Duration::from_secs(30));
    assert_eq!(config.enable_heartbeat, true);
    assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
    assert_eq!(config.tags, vec![]);
}

#[test]
fn test_base_config_multi_creation() {
    let symbols = vec!["btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string()];
    let config = BaseConfig::new_multi("test_conn", symbols.clone());
    
    assert_eq!(config.connection_id, "test_conn");
    assert_eq!(config.symbols, symbols);
    assert_eq!(config.symbol_count(), 3);
}

#[test]
fn test_base_config_builder_pattern() {
    let config = BaseConfig::new("test_conn", "btcusdt")
        .with_symbol("ethusdt")
        .with_symbols(vec!["bnbusdt".to_string(), "adausdt".to_string()])
        .with_auto_reconnect(false)
        .with_max_retries(10)
        .with_retry_delay(Duration::from_secs(10))
        .with_connection_timeout(Duration::from_secs(20))
        .with_message_timeout(Duration::from_secs(60))
        .with_heartbeat(false, Duration::from_secs(60))
        .with_tag("test")
        .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);
    
    assert_eq!(config.symbols, vec!["btcusdt", "ethusdt", "bnbusdt", "adausdt"]);
    assert_eq!(config.symbol_count(), 4);
    assert!(config.contains_symbol("btcusdt"));
    assert!(config.contains_symbol("ethusdt"));
    assert!(!config.contains_symbol("invalid"));
    assert_eq!(config.auto_reconnect, false);
    assert_eq!(config.max_retries, 10);
    assert_eq!(config.retry_delay, Duration::from_secs(10));
    assert_eq!(config.connection_timeout, Duration::from_secs(20));
    assert_eq!(config.message_timeout, Duration::from_secs(60));
    assert_eq!(config.enable_heartbeat, false);
    assert_eq!(config.heartbeat_interval, Duration::from_secs(60));
    assert_eq!(config.tags, vec!["test", "tag1", "tag2"]);
}

#[test]
fn test_base_config_symbol_methods() {
    let config = BaseConfig::new("test_conn", "btcusdt")
        .with_symbol("ethusdt")
        .with_symbol("bnbusdt");
    
    assert_eq!(config.symbol(), Some(&"btcusdt".to_string()));
    assert_eq!(config.symbols(), &["btcusdt", "ethusdt", "bnbusdt"]);
    assert_eq!(config.symbol_count(), 3);
    assert!(config.contains_symbol("btcusdt"));
    assert!(config.contains_symbol("ethusdt"));
    assert!(config.contains_symbol("bnbusdt"));
    assert!(!config.contains_symbol("invalid"));
}

#[test]
fn test_mark_price_config() {
    let config = MarkPriceConfig::new("btcusdt", "1s");
    
    assert_eq!(config.base.symbols, vec!["btcusdt"]);
    assert_eq!(config.interval, "1s");
    assert_eq!(config.base.connection_id, "mark_price_btcusdt_1s");
    assert!(config.base.tags.contains(&"mark_price".to_string()));
}

#[test]
fn test_mark_price_config_multi() {
    let symbols = vec!["btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string()];
    let config = MarkPriceConfig::new_multi(symbols.clone(), "1s");
    
    assert_eq!(config.base.symbols, symbols);
    assert_eq!(config.interval, "1s");
    assert_eq!(config.base.connection_id, "mark_price_btcusdt_ethusdt_bnbusdt_1s");
    assert!(config.base.tags.contains(&"mark_price".to_string()));
}

#[test]
fn test_mark_price_config_with_symbols() {
    let config = MarkPriceConfig::new("btcusdt", "1s")
        .with_symbol("ethusdt")
        .with_symbols(vec!["bnbusdt".to_string(), "adausdt".to_string()]);
    
    assert_eq!(config.base.symbols, vec!["btcusdt", "ethusdt", "bnbusdt", "adausdt"]);
    assert_eq!(config.base.symbol_count(), 4);
}

#[test]
fn test_kline_config() {
    let config = KlineConfig::new("btcusdt", "1m");
    
    assert_eq!(config.base.symbols, vec!["btcusdt"]);
    assert_eq!(config.interval, "1m");
    assert_eq!(config.base.connection_id, "kline_btcusdt_1m");
    assert!(config.base.tags.contains(&"kline".to_string()));
    assert!(config.base.tags.contains(&"1m".to_string()));
}

#[test]
fn test_kline_config_multi() {
    let symbols = vec!["btcusdt".to_string(), "ethusdt".to_string()];
    let config = KlineConfig::new_multi(symbols.clone(), "1m");
    
    assert_eq!(config.base.symbols, symbols);
    assert_eq!(config.interval, "1m");
    assert_eq!(config.base.connection_id, "kline_btcusdt_ethusdt_1m");
    assert!(config.base.tags.contains(&"kline".to_string()));
    assert!(config.base.tags.contains(&"1m".to_string()));
}

#[test]
fn test_depth_config() {
    let config = DepthConfig::new("btcusdt", "250ms");
    
    assert_eq!(config.base.symbols, vec!["btcusdt"]);
    assert_eq!(config.interval, "250ms");
    assert_eq!(config.base.connection_id, "depth_btcusdt_250ms");
    assert!(config.base.tags.contains(&"depth".to_string()));
    assert!(config.base.tags.contains(&"250ms".to_string()));
}

#[test]
fn test_depth_config_multi() {
    let symbols = vec!["btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string()];
    let config = DepthConfig::new_multi(symbols.clone(), "250ms");
    
    assert_eq!(config.base.symbols, symbols);
    assert_eq!(config.interval, "250ms");
    assert_eq!(config.base.connection_id, "depth_btcusdt_ethusdt_bnbusdt_250ms");
    assert!(config.base.tags.contains(&"depth".to_string()));
    assert!(config.base.tags.contains(&"250ms".to_string()));
}

#[test]
fn test_trade_config() {
    let config = TradeConfig::new("btcusdt");
    
    assert_eq!(config.base.symbols, vec!["btcusdt"]);
    assert_eq!(config.base.connection_id, "trade_btcusdt");
    assert!(config.base.tags.contains(&"trade".to_string()));
}

#[test]
fn test_trade_config_multi() {
    let symbols = vec!["btcusdt".to_string(), "ethusdt".to_string()];
    let config = TradeConfig::new_multi(symbols.clone());
    
    assert_eq!(config.base.symbols, symbols);
    assert_eq!(config.base.connection_id, "trade_btcusdt_ethusdt");
    assert!(config.base.tags.contains(&"trade".to_string()));
}

#[test]
fn test_ticker_config() {
    let config = TickerConfig::new("btcusdt");
    
    assert_eq!(config.base.symbols, vec!["btcusdt"]);
    assert_eq!(config.base.connection_id, "ticker_btcusdt");
    assert!(config.base.tags.contains(&"ticker".to_string()));
}

#[test]
fn test_ticker_config_multi() {
    let symbols = vec!["btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string()];
    let config = TickerConfig::new_multi(symbols.clone());
    
    assert_eq!(config.base.symbols, symbols);
    assert_eq!(config.base.connection_id, "ticker_btcusdt_ethusdt_bnbusdt");
    assert!(config.base.tags.contains(&"ticker".to_string()));
}

#[test]
fn test_all_ticker_config() {
    let config = AllTickerConfig::new();
    
    assert_eq!(config.base.symbols, vec!["!ticker@arr"]);
    assert_eq!(config.base.connection_id, "all_ticker");
    assert!(config.base.tags.contains(&"all_ticker".to_string()));
}

#[test]
fn test_config_with_custom_settings() {
    let config = MarkPriceConfig::new("btcusdt", "1s")
        .with_symbol("ethusdt")
        .base
        .with_auto_reconnect(true)
        .with_max_retries(20)
        .with_retry_delay(Duration::from_millis(100))
        .with_connection_timeout(Duration::from_secs(5))
        .with_message_timeout(Duration::from_secs(10))
        .with_heartbeat(true, Duration::from_secs(10))
        .with_tags(vec!["hft".to_string(), "latency_critical".to_string()]);
    
    assert_eq!(config.base.symbols, vec!["btcusdt", "ethusdt"]);
    assert_eq!(config.base.auto_reconnect, true);
    assert_eq!(config.base.max_retries, 20);
    assert_eq!(config.base.retry_delay, Duration::from_millis(100));
    assert_eq!(config.base.connection_timeout, Duration::from_secs(5));
    assert_eq!(config.base.message_timeout, Duration::from_secs(10));
    assert_eq!(config.base.enable_heartbeat, true);
    assert_eq!(config.base.heartbeat_interval, Duration::from_secs(10));
    assert!(config.base.tags.contains(&"hft".to_string()));
    assert!(config.base.tags.contains(&"latency_critical".to_string()));
}

#[test]
fn test_websocket_data_type() {
    let mark_price = WebSocketDataType::MarkPrice;
    let kline = WebSocketDataType::Kline;
    let depth = WebSocketDataType::Depth;
    let trade = WebSocketDataType::Trade;
    let ticker = WebSocketDataType::Ticker;
    let all_ticker = WebSocketDataType::AllTicker;
    
    assert_ne!(mark_price, kline);
    assert_ne!(kline, depth);
    assert_ne!(depth, trade);
    assert_ne!(trade, ticker);
    assert_ne!(ticker, all_ticker);
}

#[test]
fn test_config_clone() {
    let original = MarkPriceConfig::new("btcusdt", "1s")
        .with_symbol("ethusdt")
        .base
        .with_auto_reconnect(true)
        .with_max_retries(10)
        .with_tag("test");
    
    let cloned = original.clone();
    
    assert_eq!(original.base.connection_id, cloned.base.connection_id);
    assert_eq!(original.base.symbols, cloned.base.symbols);
    assert_eq!(original.base.auto_reconnect, cloned.base.auto_reconnect);
    assert_eq!(original.base.max_retries, cloned.base.max_retries);
    assert_eq!(original.base.tags, cloned.base.tags);
    assert_eq!(original.interval, cloned.interval);
}

#[test]
fn test_config_debug() {
    let config = MarkPriceConfig::new("btcusdt", "1s")
        .with_symbol("ethusdt");
    let debug_str = format!("{:?}", config);
    
    // 确保 Debug 实现正常工作
    assert!(debug_str.contains("btcusdt"));
    assert!(debug_str.contains("ethusdt"));
    assert!(debug_str.contains("1s"));
    assert!(debug_str.contains("mark_price"));
}

// 集成测试：测试配置的完整流程
#[tokio::test]
async fn test_config_integration() {
    // 创建单个交易对配置
    let mark_price_config = MarkPriceConfig::new("btcusdt", "1s")
        .base
        .with_auto_reconnect(true)
        .with_max_retries(5)
        .with_tag("test");
    
    // 创建多个交易对配置
    let kline_config = KlineConfig::new_multi(
        vec!["btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string()], 
        "1m"
    )
    .base
    .with_auto_reconnect(true)
    .with_max_retries(5)
    .with_tag("test");
    
    // 验证配置创建成功
    assert_eq!(mark_price_config.base.symbols, vec!["btcusdt"]);
    assert_eq!(kline_config.base.symbols, vec!["btcusdt", "ethusdt", "bnbusdt"]);
    assert_eq!(mark_price_config.interval, "1s");
    assert_eq!(kline_config.interval, "1m");
    
    // 验证标签
    assert!(mark_price_config.base.tags.contains(&"mark_price".to_string()));
    assert!(mark_price_config.base.tags.contains(&"test".to_string()));
    assert!(kline_config.base.tags.contains(&"kline".to_string()));
    assert!(kline_config.base.tags.contains(&"test".to_string()));
}

// 性能测试：测试大量配置的创建
#[test]
fn test_config_creation_performance() {
    use std::time::Instant;
    
    let start = Instant::now();
    
    // 创建 1000 个配置
    for i in 0..1000 {
        let symbol = format!("btcusdt{}", i);
        let _config = MarkPriceConfig::new(&symbol, "1s")
            .with_symbol("ethusdt")
            .with_symbol("bnbusdt")
            .base
            .with_auto_reconnect(true)
            .with_max_retries(5)
            .with_tag("performance_test");
    }
    
    let duration = start.elapsed();
    
    // 确保创建 1000 个配置的时间在合理范围内（比如小于 100ms）
    assert!(duration.as_millis() < 100, "配置创建太慢: {:?}", duration);
}

// 内存测试：测试配置的内存使用
#[test]
fn test_config_memory_usage() {
    use std::mem;
    
    let base_config = BaseConfig::new("test", "btcusdt");
    let mark_price_config = MarkPriceConfig::new("btcusdt", "1s");
    let kline_config = KlineConfig::new("btcusdt", "1m");
    let depth_config = DepthConfig::new("btcusdt", "250ms");
    
    // 打印各种配置的大小（用于调试）
    println!("BaseConfig size: {} bytes", mem::size_of::<BaseConfig>());
    println!("MarkPriceConfig size: {} bytes", mem::size_of::<MarkPriceConfig>());
    println!("KlineConfig size: {} bytes", mem::size_of::<KlineConfig>());
    println!("DepthConfig size: {} bytes", mem::size_of::<DepthConfig>());
    
    // 确保配置大小合理（小于 1KB）
    assert!(mem::size_of::<BaseConfig>() < 1024);
    assert!(mem::size_of::<MarkPriceConfig>() < 1024);
    assert!(mem::size_of::<KlineConfig>() < 1024);
    assert!(mem::size_of::<DepthConfig>() < 1024);
}

// 测试多交易对配置的各种场景
#[test]
fn test_multi_symbol_scenarios() {
    // 测试空交易对列表
    let empty_symbols: Vec<String> = vec![];
    let config = BaseConfig::new_multi("test", empty_symbols);
    assert_eq!(config.symbol_count(), 0);
    assert_eq!(config.symbol(), None);
    
    // 测试单个交易对
    let single_symbol = vec!["btcusdt".to_string()];
    let config = BaseConfig::new_multi("test", single_symbol);
    assert_eq!(config.symbol_count(), 1);
    assert_eq!(config.symbol(), Some(&"btcusdt".to_string()));
    
    // 测试多个交易对
    let multi_symbols = vec!["btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string()];
    let config = BaseConfig::new_multi("test", multi_symbols);
    assert_eq!(config.symbol_count(), 3);
    assert_eq!(config.symbol(), Some(&"btcusdt".to_string()));
    assert!(config.contains_symbol("btcusdt"));
    assert!(config.contains_symbol("ethusdt"));
    assert!(config.contains_symbol("bnbusdt"));
    assert!(!config.contains_symbol("invalid"));
}

// 测试连接ID生成
#[test]
fn test_connection_id_generation() {
    // 单个交易对
    let config = MarkPriceConfig::new("btcusdt", "1s");
    assert_eq!(config.base.connection_id, "mark_price_btcusdt_1s");
    
    // 多个交易对
    let symbols = vec!["btcusdt".to_string(), "ethusdt".to_string()];
    let config = MarkPriceConfig::new_multi(symbols, "1s");
    assert_eq!(config.base.connection_id, "mark_price_btcusdt_ethusdt_1s");
    
    // 更多交易对
    let symbols = vec!["btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string()];
    let config = KlineConfig::new_multi(symbols, "1m");
    assert_eq!(config.base.connection_id, "kline_btcusdt_ethusdt_bnbusdt_1m");
} 