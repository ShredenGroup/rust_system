use serde::{Deserialize,Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceUserConfig{
    pub api_key:String,
    pub secret_key:String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OKXUserConfig{
    pub api_key:String,
    pub secret_key:String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig{
    pub binance_user:Option<BinanceUserConfig>,
    pub okx_user:Option<OKXUserConfig>,
}

/// 从环境变量加载用户配置 (API Keys).
///
/// This function reads the following environment variables:
/// - `RUST_SYSTEM_BINANCE_USER__API_KEY`
/// - `RUST_SYSTEM_BINANCE_USER__SECRET_KEY`
/// - `RUST_SYSTEM_OKX_USER__API_KEY`
/// - `RUST_SYSTEM_OKX_USER__SECRET_KEY`
///
/// It's recommended to call `dotenv::dotenv().ok()` at the start of your application
/// to load these from a `.env` file during development.
pub fn load_user_config_from_env() -> Result<UserConfig, Box<dyn std::error::Error>> {
    let mut user_config = UserConfig::default();

    // Try to load Binance config from environment variables
    match (env::var("BINANCE_USER__API_KEY"), env::var("BINANCE_USER__SECRET_KEY")) {
        (Ok(api_key), Ok(secret_key)) if !api_key.is_empty() && !secret_key.is_empty() => {
            user_config.binance_user = Some(BinanceUserConfig {
                api_key,
                secret_key,
            });
        },
        _ => {
            // Either variables not set, one of them is missing, or they are empty.
            // In any case, we don't load the binance_user.
        }
    }

    // Try to load OKX config from environment variables
    match (env::var("OKX_USER__API_KEY"), env::var("OKX_USER__SECRET_KEY")) {
        (Ok(api_key), Ok(secret_key)) if !api_key.is_empty() && !secret_key.is_empty() => {
            user_config.okx_user = Some(OKXUserConfig {
                api_key,
                secret_key,
            });
        },
        _ => {}
    }
    
    Ok(user_config)
}


/// Loads Binance user configuration specifically from environment variables.
/// Returns an error if the required environment variables are not set.
pub fn load_binance_user_config() -> Result<BinanceUserConfig,Box<dyn std::error::Error>>{
    let user_config = load_user_config_from_env()?;
    user_config.binance_user.ok_or_else(|| "Binance user config not found in environment variables. Please set RUST_SYSTEM_BINANCE_USER__API_KEY and RUST_SYSTEM_BINANCE_USER__SECRET_KEY.".into())
}

/// Loads OKX user configuration specifically from environment variables.
/// Returns an error if the required environment variables are not set.
pub fn load_okx_user_config() -> Result<OKXUserConfig,Box<dyn std::error::Error>>{
    let user_config = load_user_config_from_env()?;
    user_config.okx_user.ok_or_else(|| "OKX user config not found in environment variables. Please set RUST_SYSTEM_OKX_USER__API_KEY and RUST_SYSTEM_OKX_USER__SECRET_KEY.".into())
}
