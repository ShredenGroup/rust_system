use serde::{Deserialize,Serialize};
use std::env;
use anyhow::{Result, Context};
use dotenv::dotenv;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MexcUserConfig{
    pub api_key:String,
    pub secret_key:String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig{
    pub binance_user:Option<BinanceUserConfig>,
    pub okx_user:Option<OKXUserConfig>,
    pub mexc_user:Option<MexcUserConfig>,
}

/// 从环境变量或.env文件加载用户配置 (API Keys).
///
/// 加载顺序:
/// 1. 首先尝试从环境变量加载
/// 2. 如果环境变量中没有，则尝试从.env文件加载
///
/// 支持的变量名:
/// - `BINANCE_USER__API_KEY`
/// - `BINANCE_USER__SECRET_KEY`
/// - `OKX_USER__API_KEY`
/// - `OKX_USER__SECRET_KEY`
/// - `MEXC_USER_API_KEY`
/// - `MEXC_USER_SECRET_KEY`
pub fn load_user_config_from_env() -> Result<UserConfig> {
    let mut user_config = UserConfig::default();

    // 如果环境变量中找不到配置，尝试加载.env文件
    if env::var("BINANCE_USER__API_KEY").is_err() || env::var("BINANCE_USER__SECRET_KEY").is_err() {
        dotenv().ok();
    }

    // 尝试加载Binance配置
    match (env::var("BINANCE_USER__API_KEY"), env::var("BINANCE_USER__SECRET_KEY")) {
        (Ok(api_key), Ok(secret_key)) if !api_key.is_empty() && !secret_key.is_empty() => {
            user_config.binance_user = Some(BinanceUserConfig {
                api_key,
                secret_key,
            });
        },
        _ => {
            println!("⚠️  未找到Binance配置，请检查环境变量或.env文件是否包含:");
            println!("   BINANCE_USER__API_KEY");
            println!("   BINANCE_USER__SECRET_KEY");
        }
    }

    // 尝试加载OKX配置
    match (env::var("OKX_USER__API_KEY"), env::var("OKX_USER__SECRET_KEY")) {
        (Ok(api_key), Ok(secret_key)) if !api_key.is_empty() && !secret_key.is_empty() => {
            user_config.okx_user = Some(OKXUserConfig {
                api_key,
                secret_key,
            });
        },
        _ => {
            println!("⚠️  未找到OKX配置，请检查环境变量或.env文件是否包含:");
            println!("   OKX_USER__API_KEY");
            println!("   OKX_USER__SECRET_KEY");
        }
    }

    // 尝试加载MEXC配置
    match (env::var("MEXC_USER_API_KEY"), env::var("MEXC_USER_SECRET_KEY")) {
        (Ok(api_key), Ok(secret_key)) if !api_key.is_empty() && !secret_key.is_empty() => {
            user_config.mexc_user = Some(MexcUserConfig {
                api_key,
                secret_key,
            });
        },
        _ => {
            println!("⚠️  未找到MEXC配置，请检查环境变量或.env文件是否包含:");
            println!("   MEXC_USER_API_KEY");
            println!("   MEXC_USER_SECRET_KEY");
        }
    }
    
    Ok(user_config)
}

/// Loads Binance user configuration specifically from environment variables or .env file.
/// Returns an error if the required configuration is not found.
pub fn load_binance_user_config() -> Result<BinanceUserConfig> {
    let user_config = load_user_config_from_env().context("Failed to load user config")?;
    user_config.binance_user.ok_or_else(|| anyhow::anyhow!(
        "Binance user config not found. Please ensure either:\n\
         1. Environment variables are set:\n\
            - BINANCE_USER__API_KEY\n\
            - BINANCE_USER__SECRET_KEY\n\
         2. Or .env file exists with these variables"
    ))
}

/// Loads OKX user configuration specifically from environment variables or .env file.
/// Returns an error if the required configuration is not found.
pub fn load_okx_user_config() -> Result<OKXUserConfig> {
    let user_config = load_user_config_from_env().context("Failed to load user config")?;
    user_config.okx_user.ok_or_else(|| anyhow::anyhow!(
        "OKX user config not found. Please ensure either:\n\
         1. Environment variables are set:\n\
            - OKX_USER__API_KEY\n\
            - OKX_USER__SECRET_KEY\n\
         2. Or .env file exists with these variables"
    ))
}

/// Loads MEXC user configuration specifically from environment variables or .env file.
/// Returns an error if the required configuration is not found.
pub fn load_mexc_user_config() -> Result<MexcUserConfig> {
    let user_config = load_user_config_from_env().context("Failed to load user config")?;
    user_config.mexc_user.ok_or_else(|| anyhow::anyhow!(
        "MEXC user config not found. Please ensure either:\n\
         1. Environment variables are set:\n\
            - MEXC_USER_API_KEY\n\
            - MEXC_USER_SECRET_KEY\n\
         2. Or .env file exists with these variables"
    ))
}
