# 多交易对 WebSocket 配置架构

## 🎯 功能概述

新的 WebSocket 配置架构支持同时监听多个交易对，大大提高了系统的灵活性和效率。

## 🆕 主要改进

### 1. 多交易对支持
- **单个交易对**: 保持向后兼容
- **多个交易对**: 支持同时监听多个交易对
- **动态添加**: 支持运行时添加新的交易对

### 2. 配置方法

#### 基础配置 (BaseConfig)
```rust
// 单个交易对
let config = BaseConfig::new("conn_id", "btcusdt");

// 多个交易对
let symbols = vec!["btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string()];
let config = BaseConfig::new_multi("conn_id", symbols);

// 链式添加
let config = BaseConfig::new("conn_id", "btcusdt")
    .with_symbol("ethusdt")
    .with_symbols(vec!["bnbusdt".to_string(), "adausdt".to_string()]);
```

#### 具体配置类型
```rust
// 标记价格配置
let config = MarkPriceConfig::new("btcusdt", "1s");                    // 单个
let config = MarkPriceConfig::new_multi(symbols, "1s");               // 多个
let config = MarkPriceConfig::new("btcusdt", "1s").with_symbol("ethusdt"); // 链式

// K线数据配置
let config = KlineConfig::new("btcusdt", "1m");                       // 单个
let config = KlineConfig::new_multi(symbols, "1m");                   // 多个
let config = KlineConfig::new("btcusdt", "1m").with_symbol("ethusdt"); // 链式

// 订单簿深度配置
let config = DepthConfig::new("btcusdt", "250ms");                    // 单个
let config = DepthConfig::new_multi(symbols, "250ms");                // 多个
let config = DepthConfig::new("btcusdt", "250ms").with_symbol("ethusdt"); // 链式
```

## 🚀 使用示例

### 1. 基础使用

```rust
// 创建 WebSocket 管理器
let (manager, mut rx) = create_websocket_manager().await?;

// 单个交易对配置
let single_config = MarkPriceConfig::new("btcusdt", "1s")
    .base
    .with_auto_reconnect(true)
    .with_max_retries(5);

// 多个交易对配置
let multi_config = KlineConfig::new_multi(
    vec!["btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string()], 
    "1m"
)
.base
.with_auto_reconnect(true)
.with_max_retries(10);

// 启动连接
manager.start_mark_price(single_config).await?;
manager.start_kline(multi_config).await?;
```

### 2. 高级使用

```rust
// 高频交易配置
let hft_config = MarkPriceConfig::new_multi(
    vec!["btcusdt".to_string(), "ethusdt".to_string()], 
    "1s"
)
.base
.with_auto_reconnect(true)
.with_max_retries(50)
.with_retry_delay(Duration::from_millis(50))
.with_connection_timeout(Duration::from_secs(2))
.with_message_timeout(Duration::from_secs(5))
.with_heartbeat(true, Duration::from_secs(5))
.with_tags(vec!["hft".to_string(), "latency_critical".to_string()]);

// 投资组合监控配置
let portfolio_symbols = vec![
    "btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string(),
    "adausdt".to_string(), "dogeusdt".to_string(), "solusdt".to_string(),
    "dotusdt".to_string(), "linkusdt".to_string(), "maticusdt".to_string()
];
let portfolio_config = TickerConfig::new_multi(portfolio_symbols)
    .base
    .with_auto_reconnect(true)
    .with_max_retries(10)
    .with_retry_delay(Duration::from_secs(10))
    .with_heartbeat(true, Duration::from_secs(60))
    .with_tags(vec!["portfolio".to_string(), "monitoring".to_string()]);
```

### 3. 动态管理

```rust
// 初始配置
let mut dynamic_config = MarkPriceConfig::new("btcusdt", "1s")
    .base
    .with_auto_reconnect(true)
    .with_max_retries(5)
    .with_tag("dynamic");

// 动态添加交易对
dynamic_config = dynamic_config
    .with_symbol("ethusdt")
    .with_symbol("bnbusdt")
    .with_symbols(vec!["adausdt".to_string(), "dogeusdt".to_string()]);

// 启动连接
manager.start_mark_price(dynamic_config).await?;
```

## 📊 连接管理

### 查询功能

```rust
// 获取所有连接
let connections = manager.list_connections().await;

// 按交易对查询连接
let btc_connections = manager.get_connections_by_symbol("btcusdt").await;
let eth_connections = manager.get_connections_by_symbol("ethusdt").await;

// 按标签查询连接
let hft_connections = manager.get_connections_by_tag("hft").await;
let portfolio_connections = manager.get_connections_by_tag("portfolio").await;

// 按数据类型查询连接
let mark_price_connections = manager.get_connections_by_type(&WebSocketDataType::MarkPrice).await;
```

### 连接信息

```rust
pub struct ConnectionInfo {
    pub connection_id: String,
    pub symbols: Vec<String>,        // 支持多个交易对
    pub data_type: WebSocketDataType,
    pub status: ConnectionStatus,
    pub created_at: std::time::Instant,
    pub last_message_at: Option<std::time::Instant>,
    pub tags: Vec<String>,
}
```

## 🔧 新增方法

### BaseConfig 新增方法

| 方法 | 描述 | 示例 |
|------|------|------|
| `new_multi()` | 创建多交易对配置 | `BaseConfig::new_multi("conn_id", symbols)` |
| `with_symbol()` | 添加单个交易对 | `.with_symbol("ethusdt")` |
| `with_symbols()` | 添加多个交易对 | `.with_symbols(vec!["btc", "eth"])` |
| `symbol()` | 获取第一个交易对 | `config.symbol()` |
| `symbols()` | 获取所有交易对 | `config.symbols()` |
| `contains_symbol()` | 检查是否包含交易对 | `config.contains_symbol("btcusdt")` |
| `symbol_count()` | 获取交易对数量 | `config.symbol_count()` |

### 具体配置类型新增方法

| 配置类型 | 新增方法 | 描述 |
|----------|----------|------|
| MarkPriceConfig | `new_multi()` | 创建多交易对标记价格配置 |
| MarkPriceConfig | `with_symbol()` | 添加交易对 |
| MarkPriceConfig | `with_symbols()` | 添加多个交易对 |
| KlineConfig | `new_multi()` | 创建多交易对K线配置 |
| KlineConfig | `with_symbol()` | 添加交易对 |
| KlineConfig | `with_symbols()` | 添加多个交易对 |
| DepthConfig | `new_multi()` | 创建多交易对深度配置 |
| DepthConfig | `with_symbol()` | 添加交易对 |
| DepthConfig | `with_symbols()` | 添加多个交易对 |
| TradeConfig | `new_multi()` | 创建多交易对交易配置 |
| TradeConfig | `with_symbol()` | 添加交易对 |
| TradeConfig | `with_symbols()` | 添加多个交易对 |
| TickerConfig | `new_multi()` | 创建多交易对Ticker配置 |
| TickerConfig | `with_symbol()` | 添加交易对 |
| TickerConfig | `with_symbols()` | 添加多个交易对 |

## 🏷️ 连接ID生成

### 单个交易对
```
mark_price_btcusdt_1s
kline_btcusdt_1m
depth_btcusdt_250ms
```

### 多个交易对
```
mark_price_btcusdt_ethusdt_bnbusdt_1s
kline_btcusdt_ethusdt_1m
depth_btcusdt_ethusdt_bnbusdt_250ms
```

## 📈 性能优化

### 1. 批量处理
- 多个交易对共享同一个连接配置
- 减少重复的配置创建
- 提高内存使用效率

### 2. 连接复用
- 相同配置的交易对可以共享连接
- 减少网络连接数量
- 降低资源消耗

### 3. 消息聚合
- 多个交易对的消息可以批量处理
- 提高消息处理效率
- 减少系统开销

## 🔍 监控和调试

### 连接状态监控
```rust
// 获取连接统计
let connections = manager.list_connections().await;
for conn in connections {
    println!("连接: {}, 交易对: {}, 状态: {:?}", 
             conn.connection_id, 
             conn.symbols.join(","), 
             conn.status);
}
```

### 交易对使用统计
```rust
// 统计各交易对的使用情况
let mut symbol_usage: HashMap<String, usize> = HashMap::new();
for conn in connections {
    for symbol in &conn.symbols {
        let count = symbol_usage.entry(symbol.clone()).or_insert(0);
        *count += 1;
    }
}

for (symbol, count) in symbol_usage {
    println!("{}: 在 {} 个连接中使用", symbol, count);
}
```

## 🎯 最佳实践

### 1. 交易对分组
```rust
// 按功能分组
let major_pairs = vec!["btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string()];
let defi_pairs = vec!["adausdt".to_string(), "linkusdt".to_string(), "maticusdt".to_string()];

let major_config = MarkPriceConfig::new_multi(major_pairs, "1s")
    .base
    .with_tag("major_pairs");

let defi_config = MarkPriceConfig::new_multi(defi_pairs, "1s")
    .base
    .with_tag("defi_pairs");
```

### 2. 配置模板
```rust
// 高频交易模板
fn create_hft_config(symbols: Vec<String>) -> MarkPriceConfig {
    MarkPriceConfig::new_multi(symbols, "1s")
        .base
        .with_auto_reconnect(true)
        .with_max_retries(50)
        .with_retry_delay(Duration::from_millis(50))
        .with_connection_timeout(Duration::from_secs(2))
        .with_message_timeout(Duration::from_secs(5))
        .with_heartbeat(true, Duration::from_secs(5))
        .with_tags(vec!["hft".to_string(), "latency_critical".to_string()])
}

// 数据分析模板
fn create_analysis_config(symbols: Vec<String>) -> KlineConfig {
    KlineConfig::new_multi(symbols, "1h")
        .base
        .with_auto_reconnect(true)
        .with_max_retries(10)
        .with_retry_delay(Duration::from_secs(30))
        .with_connection_timeout(Duration::from_secs(30))
        .with_message_timeout(Duration::from_secs(300))
        .with_heartbeat(true, Duration::from_secs(120))
        .with_tags(vec!["analysis".to_string(), "long_term".to_string()])
}
```

### 3. 动态管理
```rust
// 根据市场情况动态调整
async fn adjust_portfolio(manager: &WebSocketManager, new_symbols: Vec<String>) -> Result<()> {
    // 停止旧连接
    let old_connections = manager.get_connections_by_tag("portfolio").await;
    for conn in old_connections {
        manager.stop_connection(&conn.connection_id).await?;
    }
    
    // 创建新连接
    let new_config = TickerConfig::new_multi(new_symbols)
        .base
        .with_auto_reconnect(true)
        .with_max_retries(10)
        .with_tag("portfolio");
    
    manager.start_depth(new_config).await?;
    Ok(())
}
```

## 🔮 未来扩展

### 1. 智能分组
- 根据交易对相关性自动分组
- 动态调整连接策略
- 智能负载均衡

### 2. 配置验证
- 交易对有效性检查
- 配置冲突检测
- 性能影响评估

### 3. 高级监控
- 实时性能指标
- 自动故障转移
- 智能重连策略

这个新的多交易对架构大大提高了 WebSocket 系统的灵活性和效率，使得同时监控大量交易对变得更加简单和可靠。 