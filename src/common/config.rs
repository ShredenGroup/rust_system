use crate::common::error::ConfigError;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: Option<String>, // 某些交易所需要
    pub sandbox: bool, // 是否使用沙盒环境
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub default_quantity: f64,
    pub max_position_size: f64,
    pub risk_percentage: f64,
    pub enable_trading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub symbols: Vec<String>,
    pub interval: String,
    pub auto_reconnect: bool,
    pub max_retries: u32,
    pub retry_delay: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub enabled: bool,
    pub symbols: Vec<String>,
    pub min_volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MACrossoverConfig {
    pub enabled: bool,
    pub short_period: u32,
    pub long_period: u32,
    pub symbols: Vec<String>,
    pub min_volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSIStrategyConfig {
    pub enabled: bool,
    pub period: u32,
    pub oversold: f64,
    pub overbought: f64,
    pub symbols: Vec<String>,
    pub min_volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BollingerBandsConfig {
    pub enabled: bool,
    pub period: u32,
    pub std_dev: f64,
    pub symbols: Vec<String>,
    pub min_volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskManagementConfig {
    pub max_daily_loss: f64,
    pub max_position_size_per_symbol: f64,
    pub stop_loss_percentage: f64,
    pub take_profit_percentage: f64,
    pub max_open_positions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub telegram_enabled: bool,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub email_enabled: bool,
    pub email_smtp_server: Option<String>,
    pub email_username: Option<String>,
    pub email_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategiesConfig {
    pub ma_crossover: MACrossoverConfig,
    pub rsi_strategy: RSIStrategyConfig,
    pub bollinger_bands: BollingerBandsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub environment: String,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub exchanges: Vec<ExchangeConfig>,
    pub trading: TradingConfig,
    pub logging: LoggingConfig,
    pub websocket: WebSocketConfig,
    pub strategies: StrategiesConfig,
    pub risk_management: RiskManagementConfig,
    pub notifications: NotificationConfig,
}

impl AppConfig {
    /// 从 TOML 文件加载配置
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        // 加载 .env 文件
        dotenv::dotenv().ok();
        
        let content = fs::read_to_string(path)?;
        let mut config: AppConfig = toml::from_str(&content)?;
        
        // 替换环境变量
        config.replace_env_vars()?;
        
        Ok(config)
    }
    
    /// 从默认路径加载配置
    pub fn load() -> Result<Self, ConfigError> {
        // 加载 .env 文件
        dotenv::dotenv().ok();
        
        let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
        Self::load_from_file(config_path)
    }
    
    /// 替换配置中的环境变量
    fn replace_env_vars(&mut self) -> Result<(), ConfigError> {
        // 替换数据库 URL
        self.database.url = replace_env_var(&self.database.url)?;
        
        // 替换 Redis URL
        self.redis.url = replace_env_var(&self.redis.url)?;
        
        // 替换交易所配置
        for exchange in &mut self.exchanges {
            exchange.api_key = replace_env_var(&exchange.api_key)?;
            exchange.secret_key = replace_env_var(&exchange.secret_key)?;
            if let Some(ref mut passphrase) = exchange.passphrase {
                *passphrase = replace_env_var(passphrase)?;
            }
            exchange.base_url = replace_env_var(&exchange.base_url)?;
        }
        
        // 替换通知配置
        if let Some(ref mut token) = self.notifications.telegram_bot_token {
            *token = replace_env_var(token)?;
        }
        if let Some(ref mut chat_id) = self.notifications.telegram_chat_id {
            *chat_id = replace_env_var(chat_id)?;
        }
        if let Some(ref mut username) = self.notifications.email_username {
            *username = replace_env_var(username)?;
        }
        if let Some(ref mut password) = self.notifications.email_password {
            *password = replace_env_var(password)?;
        }
        
        Ok(())
    }
    
    /// 检查是否为生产环境
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
    
    /// 检查是否为开发环境
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }
    
    /// 获取启用的策略列表
    pub fn get_enabled_strategies(&self) -> Vec<String> {
        let mut strategies = Vec::new();
        
        if self.strategies.ma_crossover.enabled {
            strategies.push("ma_crossover".to_string());
        }
        if self.strategies.rsi_strategy.enabled {
            strategies.push("rsi_strategy".to_string());
        }
        if self.strategies.bollinger_bands.enabled {
            strategies.push("bollinger_bands".to_string());
        }
        
        strategies
    }
    
    /// 获取指定策略的交易对
    pub fn get_strategy_symbols(&self, strategy_name: &str) -> Option<Vec<String>> {
        match strategy_name {
            "ma_crossover" => Some(self.strategies.ma_crossover.symbols.clone()),
            "rsi_strategy" => Some(self.strategies.rsi_strategy.symbols.clone()),
            "bollinger_bands" => Some(self.strategies.bollinger_bands.symbols.clone()),
            _ => None,
        }
    }
    
    /// 获取所有交易对（去重）
    pub fn get_all_symbols(&self) -> Vec<String> {
        let mut symbols = std::collections::HashSet::new();
        
        // 添加 WebSocket 交易对
        symbols.extend(self.websocket.symbols.clone());
        
        // 添加策略交易对
        symbols.extend(self.strategies.ma_crossover.symbols.clone());
        symbols.extend(self.strategies.rsi_strategy.symbols.clone());
        symbols.extend(self.strategies.bollinger_bands.symbols.clone());
        
        symbols.into_iter().collect()
    }
}

/// 替换字符串中的环境变量
fn replace_env_var(s: &str) -> Result<String, ConfigError> {
    if s.starts_with("${") && s.ends_with("}") {
        let var_name = &s[2..s.len()-1];
        env::var(var_name).map_err(|_| ConfigError::MissingEnvVar(var_name.to_string()))
    } else {
        Ok(s.to_string())
    }
}

// 全局配置实例
use std::sync::OnceLock;

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

pub fn get_config() -> &'static AppConfig {
    CONFIG.get_or_init(|| {
        AppConfig::load().expect("Failed to load configuration")
    })
}

pub fn init_config() -> Result<(), ConfigError> {
    let config = AppConfig::load()?;
    CONFIG.set(config).map_err(|_| ConfigError::ParseError("Configuration already initialized".to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_replace_env_var() {
        // 设置测试环境变量
        unsafe {
            env::set_var("TEST_VAR", "test_value");
        }
        
        let result = replace_env_var("${TEST_VAR}").unwrap();
        assert_eq!(result, "test_value");
        
        let result = replace_env_var("no_env_var").unwrap();
        assert_eq!(result, "no_env_var");
    }
    
    #[test]
    fn test_load_config() {
        // 这个测试需要 config.toml 文件存在
        if Path::new("config.toml").exists() {
            let config = AppConfig::load();
            assert!(config.is_ok());
        }
    }
} 