use serde::{Deserialize, Serialize};

/// K线数据API配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineApiConfig {
    /// 交易对
    pub symbol: String,
    /// K线间隔 (e.g., "1m", "5m", "1h")
    pub interval: String,
    /// 任务执行间隔（秒）
    pub interval_secs: u64,
    /// 是否启用
    pub enabled: bool,
    /// 基础配置
    #[serde(default)]
    pub base: ApiBaseConfig,
}

/// API基础配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiBaseConfig {
    /// 自动重试
    #[serde(default)]
    pub auto_retry: bool,
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 重试延迟（秒）
    #[serde(default = "default_retry_delay_secs")]
    pub retry_delay_secs: u64,
    /// 请求超时（秒）
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// 标签（用于分组和筛选）
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ApiBaseConfig {
    pub fn retry_delay(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.retry_delay_secs)
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_secs)
    }
}

/// 默认配置值
fn default_max_retries() -> u32 { 3 }
fn default_retry_delay_secs() -> u64 { 5 }
fn default_timeout_secs() -> u64 { 30 }

/// API配置加载器
pub struct ApiConfigLoader;

impl ApiConfigLoader {
    /// 从文件加载配置
    pub fn load_from_file(path: &str) -> Result<ApiConfigs, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let configs: ApiConfigs = toml::from_str(&content)?;
        Ok(configs)
    }
}

/// 所有API配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiConfigs {
    /// K线数据配置
    #[serde(default)]
    pub kline: Vec<KlineApiConfig>,
    // 可以添加其他类型的API配置
}

// 配置示例
impl ApiConfigs {
    pub fn example() -> Self {
        Self {
            kline: vec![
                KlineApiConfig {
                    symbol: "BTCUSDT".to_string(),
                    interval: "1m".to_string(),
                    interval_secs: 60,
                    enabled: true,
                    base: ApiBaseConfig {
                        auto_retry: true,
                        max_retries: 3,
                        retry_delay_secs: 5,
                        timeout_secs: 30,
                        tags: vec!["main".to_string()],
                    },
                },
                KlineApiConfig {
                    symbol: "ETHUSDT".to_string(),
                    interval: "5m".to_string(),
                    interval_secs: 300,
                    enabled: true,
                    base: ApiBaseConfig::default(),
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = ApiConfigs::example();
        let serialized = toml::to_string_pretty(&config).unwrap();
        println!("Example config:\n{}", serialized);

        let deserialized: ApiConfigs = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.kline.len(), config.kline.len());
    }

    #[test]
    fn test_base_config_defaults() {
        let base = ApiBaseConfig::default();
        assert!(!base.auto_retry);
        assert_eq!(base.max_retries, 3);
        assert_eq!(base.retry_delay_secs, 5);
        assert_eq!(base.timeout_secs, 30);
        assert!(base.tags.is_empty());
    }
}