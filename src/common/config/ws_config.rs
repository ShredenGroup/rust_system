use serde::{Deserialize, Serialize};
use std::time::Duration;

/// WebSocket 基础配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketBaseConfig {
    /// 是否自动重连
    pub auto_reconnect: bool,
    
    /// 最大重试次数
    pub max_retries: usize,
    
    /// 重试延迟（秒）
    pub retry_delay_secs: u64,
    
    /// 连接超时时间（秒）
    pub connection_timeout_secs: u64,
    
    /// 消息处理超时时间（秒）
    pub message_timeout_secs: u64,
    
    /// 是否启用心跳检测
    pub enable_heartbeat: bool,
    
    /// 心跳间隔（秒）
    pub heartbeat_interval_secs: u64,
    
    /// 自定义标签（用于分组管理）
    pub tags: Vec<String>,
}

impl WebSocketBaseConfig {
    /// 获取重试延迟
    pub fn retry_delay(&self) -> Duration {
        Duration::from_secs(self.retry_delay_secs)
    }
    
    /// 获取连接超时时间
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_secs)
    }
    
    /// 获取消息超时时间
    pub fn message_timeout(&self) -> Duration {
        Duration::from_secs(self.message_timeout_secs)
    }
    
    /// 获取心跳间隔
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_secs(self.heartbeat_interval_secs)
    }
    
    /// 添加标签
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }
    
    /// 添加多个标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }

    pub fn merge(&self, other: &Option<WebSocketBaseConfig>) -> WebSocketBaseConfig {
        if let Some(local) = other {
            WebSocketBaseConfig {
                auto_reconnect: local.auto_reconnect,
                max_retries: local.max_retries,
                retry_delay_secs: local.retry_delay_secs,
                connection_timeout_secs: local.connection_timeout_secs,
                message_timeout_secs: local.message_timeout_secs,
                enable_heartbeat: local.enable_heartbeat,
                heartbeat_interval_secs: local.heartbeat_interval_secs,
                tags: if local.tags.is_empty() { self.tags.clone() } else { local.tags.clone() },
            }
        } else {
            self.clone()
        }
    }
}

// Raw 结构体用于反序列化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkPriceConfigRaw {
    pub symbol: Vec<String>,
    pub interval: String,
    pub base: Option<WebSocketBaseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineConfigRaw {
    pub symbol: Vec<String>,
    pub interval: String,
    pub base: Option<WebSocketBaseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialDepthConfigRaw {
    pub symbol: Vec<String>,
    pub levels: u32,
    pub interval: String,
    pub base: Option<WebSocketBaseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffDepthConfigRaw {
    pub symbol: Vec<String>,
    pub level: u32,
    pub base: Option<WebSocketBaseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfigsRaw {
    pub base: WebSocketBaseConfig,
    #[serde(default)]
    pub mark_price: Vec<MarkPriceConfigRaw>,
    #[serde(default)]
    pub kline: Vec<KlineConfigRaw>,
    #[serde(default)]
    pub partial_depth: Vec<PartialDepthConfigRaw>,
    #[serde(default)]
    pub diff_depth: Vec<DiffDepthConfigRaw>,
}

/// 标记价格配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkPriceConfig {
    pub base: WebSocketBaseConfig,
    pub symbol: Vec<String>,  // 改为支持多个交易对
    pub interval: String,  // 更新间隔，如 "1s", "1m", "5m"
}

impl MarkPriceConfig {
    /// 创建标记价格配置
    pub fn new(symbol: &str, interval: &str, base: WebSocketBaseConfig) -> Self {
        MarkPriceConfig {
            base: base.with_tag("mark_price").with_tag(interval),
            symbol: vec![symbol.to_string()],
            interval: interval.to_string(),
        }
    }
    
    /// 创建多交易对标记价格配置
    pub fn new_multi(symbols: Vec<String>, interval: &str, base: WebSocketBaseConfig) -> Self {
        MarkPriceConfig {
            base: base.with_tag("mark_price").with_tag(interval),
            symbol: symbols,
            interval: interval.to_string(),
        }
    }
}

/// K线数据配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineConfig {
    pub base: WebSocketBaseConfig,
    pub symbol: Vec<String>,  // 改为支持多个交易对
    pub interval: String,  // K线间隔，如 "1m", "5m", "1h", "1d"
}

impl KlineConfig {
    /// 创建K线配置
    pub fn new(symbol: &str, interval: &str, base: WebSocketBaseConfig) -> Self {
        KlineConfig {
            base: base.with_tag("kline").with_tag(interval),
            symbol: vec![symbol.to_string()],
            interval: interval.to_string(),
        }
    }
    
    /// 创建多交易对K线配置
    pub fn new_multi(symbols: Vec<String>, interval: &str, base: WebSocketBaseConfig) -> Self {
        KlineConfig {
            base: base.with_tag("kline").with_tag(interval),
            symbol: symbols,
            interval: interval.to_string(),
        }
    }
}

/// 部分订单簿深度配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialDepthConfig {
    pub base: WebSocketBaseConfig,
    pub symbol: Vec<String>,  // 改为支持多个交易对
    pub levels: u32,  // 深度级别，如 5, 10, 20
    pub interval: String,  // 深度更新间隔，如 "250ms", "500ms", "100ms"
}

impl PartialDepthConfig {
    /// 创建部分深度配置
    pub fn new(symbol: &str, levels: u32, interval: &str, base: WebSocketBaseConfig) -> Self {
        PartialDepthConfig {
            base: base.with_tag("partial_depth").with_tag(&levels.to_string()).with_tag(interval),
            symbol: vec![symbol.to_string()],
            levels,
            interval: interval.to_string(),
        }
    }
    
    /// 创建多交易对部分深度配置
    pub fn new_multi(symbols: Vec<String>, levels: u32, interval: &str, base: WebSocketBaseConfig) -> Self {
        PartialDepthConfig {
            base: base.with_tag("partial_depth").with_tag(&levels.to_string()).with_tag(interval),
            symbol: symbols,
            levels,
            interval: interval.to_string(),
        }
    }
}

/// 订单簿深度差异配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffDepthConfig {
    pub base: WebSocketBaseConfig,
    pub symbol: Vec<String>,  // 改为支持多个交易对
    pub level: u32,  // 深度级别，如 5, 10, 20
}

impl DiffDepthConfig {
    /// 创建深度差异配置
    pub fn new(symbol: &str, level: u32, base: WebSocketBaseConfig) -> Self {
        DiffDepthConfig {
            base: base.with_tag("diff_depth").with_tag(&level.to_string()),
            symbol: vec![symbol.to_string()],
            level,
        }
    }
    
    /// 创建多交易对深度差异配置
    pub fn new_multi(symbols: Vec<String>, level: u32, base: WebSocketBaseConfig) -> Self {
        DiffDepthConfig {
            base: base.with_tag("diff_depth").with_tag(&level.to_string()),
            symbol: symbols,
            level,
        }
    }
}

/// WebSocket 配置集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfigs {
    /// 标记价格配置列表
    pub mark_price: Vec<MarkPriceConfig>,
    
    /// K线数据配置列表
    pub kline: Vec<KlineConfig>,
    
    /// 部分订单簿深度配置列表
    pub partial_depth: Vec<PartialDepthConfig>,
    
    /// 订单簿深度差异配置列表
    pub diff_depth: Vec<DiffDepthConfig>,

    pub base: WebSocketBaseConfig,
}

impl Default for WebSocketConfigs {
    fn default() -> Self {
        Self {
            mark_price: vec![],
            kline: vec![],
            partial_depth: vec![],
            diff_depth: vec![],
            base: WebSocketBaseConfig {
                auto_reconnect: true,
                max_retries: 5,
                retry_delay_secs: 5,
                connection_timeout_secs: 10,
                message_timeout_secs: 30,
                enable_heartbeat: true,
                heartbeat_interval_secs: 30,
                tags: vec![],
            },
        }
    }
}

/// 配置加载器
pub struct ConfigLoader;

impl ConfigLoader {
    /// 从文件加载配置
    pub fn load_from_file(path: &str) -> Result<WebSocketConfigs, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        println!("content: {}", content);
        let raw: WebSocketConfigsRaw = toml::from_str(&content)?;
        let base = raw.base.clone();
        let mark_price = raw.mark_price.into_iter().map(|item| MarkPriceConfig {
            base: base.merge(&item.base),
            symbol: item.symbol,
            interval: item.interval,
        }).collect();
        let kline = raw.kline.into_iter().map(|item| KlineConfig {
            base: base.merge(&item.base),
            symbol: item.symbol,
            interval: item.interval,
        }).collect();
        let partial_depth = raw.partial_depth.into_iter().map(|item| PartialDepthConfig {
            base: base.merge(&item.base),
            symbol: item.symbol,
            levels: item.levels,
            interval: item.interval,
        }).collect();
        let diff_depth = raw.diff_depth.into_iter().map(|item| DiffDepthConfig {
            base: base.merge(&item.base),
            symbol: item.symbol,
            level: item.level,
        }).collect();
        Ok(WebSocketConfigs {
            mark_price,
            kline,
            partial_depth,
            diff_depth,
            base,
        })
    }
    
    /// 保存配置到文件
    pub fn save_to_file(configs: &WebSocketConfigs, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(configs)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// 创建默认配置
    pub fn create_default_configs() -> WebSocketConfigs {
        let default_base = WebSocketBaseConfig {
            auto_reconnect: true,
            max_retries: 5,
            retry_delay_secs: 5,
            connection_timeout_secs: 10,
            message_timeout_secs: 30,
            enable_heartbeat: true,
            heartbeat_interval_secs: 30,
            tags: vec![],
        };
        
        WebSocketConfigs {
            mark_price: vec![
                MarkPriceConfig::new("btcusdt", "1s", default_base.clone()),
            ],
            kline: vec![
                KlineConfig::new("btcusdt", "1m", default_base.clone()),
                KlineConfig::new("btcusdt", "1h", default_base.clone()),
            ],
            partial_depth: vec![
                PartialDepthConfig::new("btcusdt", 5, "250ms", default_base.clone()),
                PartialDepthConfig::new("btcusdt", 10, "100ms", default_base.clone()),
            ],
            diff_depth: vec![
                DiffDepthConfig::new("btcusdt", 5, default_base.clone()),
                DiffDepthConfig::new("btcusdt", 10, default_base.clone()),
            ],
            base: default_base,
        }
    }
} 