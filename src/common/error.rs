use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to load environment variables: {0}")]
    EnvLoad(#[from] dotenv::Error),
    
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
    
    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),
    
    #[error("Failed to parse configuration: {0}")]
    ParseError(String),
    
    #[error("Failed to read configuration file: {0}")]
    FileRead(#[from] std::io::Error),
    
    #[error("Failed to parse TOML configuration: {0}")]
    TomlParse(#[from] toml::de::Error),
    
    #[error("Failed to serialize TOML configuration: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
} 