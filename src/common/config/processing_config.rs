use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 处理模式枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessingMode {
    #[serde(rename = "stream")]
    Stream,
    #[serde(rename = "batch")]
    Batch,
    #[serde(rename = "inherit")]
    Inherit,
}

/// 批处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// 批处理大小 (消息数量)
    pub batch_size: usize,
    /// 批处理时间窗口 (毫秒)
    pub batch_timeout_ms: u64,
    /// 最大批处理延迟 (毫秒)
    pub max_batch_delay_ms: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            batch_timeout_ms: 1000,
            max_batch_delay_ms: 5000,
        }
    }
}

impl BatchConfig {
    /// 获取批处理时间窗口
    pub fn batch_timeout(&self) -> Duration {
        Duration::from_millis(self.batch_timeout_ms)
    }

    /// 获取最大批处理延迟
    pub fn max_batch_delay(&self) -> Duration {
        Duration::from_millis(self.max_batch_delay_ms)
    }
}

/// 流处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// 处理超时时间 (毫秒)
    pub process_timeout_ms: u64,
    /// 最大并发处理数
    pub max_concurrent: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            process_timeout_ms: 100,
            max_concurrent: 10,
        }
    }
}

impl StreamConfig {
    /// 获取处理超时时间
    pub fn process_timeout(&self) -> Duration {
        Duration::from_millis(self.process_timeout_ms)
    }
}

/// 消费者配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    /// 是否启用
    pub enabled: bool,
    /// 处理模式
    pub mode: ProcessingMode,
    /// 批处理大小 (仅当 mode = "batch" 时生效)
    pub batch_size: Option<usize>,
    /// 批处理时间窗口 (仅当 mode = "batch" 时生效)
    pub batch_timeout_ms: Option<u64>,
}

/// 持久化消费者配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// 基础消费者配置
    #[serde(flatten)]
    pub consumer: ConsumerConfig,
    /// 数据库批处理大小
    pub db_batch_size: usize,
    /// 数据库刷新间隔 (毫秒)
    pub db_flush_interval_ms: u64,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            consumer: ConsumerConfig {
                enabled: true,
                mode: ProcessingMode::Batch,
                batch_size: Some(200),
                batch_timeout_ms: Some(2000),
            },
            db_batch_size: 500,
            db_flush_interval_ms: 5000,
        }
    }
}

impl PersistenceConfig {
    /// 获取数据库刷新间隔
    pub fn db_flush_interval(&self) -> Duration {
        Duration::from_millis(self.db_flush_interval_ms)
    }
}

/// 全局处理策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// 处理模式
    pub mode: ProcessingMode,
    /// 批处理配置
    pub batch: BatchConfig,
    /// 流处理配置
    pub stream: StreamConfig,
    /// 消费者配置
    pub consumers: ConsumersConfig,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            mode: ProcessingMode::Batch,
            batch: BatchConfig::default(),
            stream: StreamConfig::default(),
            consumers: ConsumersConfig::default(),
        }
    }
}

/// 消费者配置集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumersConfig {
    /// 计算模块配置
    pub calculation: ConsumerConfig,
    /// 订单模块配置
    pub order: ConsumerConfig,
    /// 持久化模块配置
    pub persistence: PersistenceConfig,
}

impl Default for ConsumersConfig {
    fn default() -> Self {
        Self {
            calculation: ConsumerConfig {
                enabled: true,
                mode: ProcessingMode::Inherit,
                batch_size: Some(50),
                batch_timeout_ms: Some(500),
            },
            order: ConsumerConfig {
                enabled: true,
                mode: ProcessingMode::Inherit,
                batch_size: Some(10),
                batch_timeout_ms: Some(200),
            },
            persistence: PersistenceConfig::default(),
        }
    }
}

/// 处理策略管理器
#[derive(Debug, Clone)]
pub struct ProcessingStrategy {
    config: ProcessingConfig,
}

impl ProcessingStrategy {
    /// 创建新的处理策略管理器
    pub fn new(config: ProcessingConfig) -> Self {
        Self { config }
    }

    /// 获取全局处理模式
    pub fn global_mode(&self) -> &ProcessingMode {
        &self.config.mode
    }

    /// 获取计算模块的处理模式
    pub fn calculation_mode(&self) -> ProcessingMode {
        match &self.config.consumers.calculation.mode {
            ProcessingMode::Inherit => self.config.mode.clone(),
            mode => mode.clone(),
        }
    }

    /// 获取订单模块的处理模式
    pub fn order_mode(&self) -> ProcessingMode {
        match &self.config.consumers.order.mode {
            ProcessingMode::Inherit => self.config.mode.clone(),
            mode => mode.clone(),
        }
    }

    /// 获取持久化模块的处理模式
    pub fn persistence_mode(&self) -> ProcessingMode {
        match &self.config.consumers.persistence.consumer.mode {
            ProcessingMode::Inherit => self.config.mode.clone(),
            mode => mode.clone(),
        }
    }

    /// 获取计算模块的批处理配置
    pub fn calculation_batch_config(&self) -> Option<BatchConfig> {
        if self.calculation_mode() == ProcessingMode::Batch {
            Some(BatchConfig {
                batch_size: self.config.consumers.calculation.batch_size.unwrap_or(self.config.batch.batch_size),
                batch_timeout_ms: self.config.consumers.calculation.batch_timeout_ms.unwrap_or(self.config.batch.batch_timeout_ms),
                max_batch_delay_ms: self.config.batch.max_batch_delay_ms,
            })
        } else {
            None
        }
    }

    /// 获取订单模块的批处理配置
    pub fn order_batch_config(&self) -> Option<BatchConfig> {
        if self.order_mode() == ProcessingMode::Batch {
            Some(BatchConfig {
                batch_size: self.config.consumers.order.batch_size.unwrap_or(self.config.batch.batch_size),
                batch_timeout_ms: self.config.consumers.order.batch_timeout_ms.unwrap_or(self.config.batch.batch_timeout_ms),
                max_batch_delay_ms: self.config.batch.max_batch_delay_ms,
            })
        } else {
            None
        }
    }

    /// 获取持久化模块的批处理配置
    pub fn persistence_batch_config(&self) -> Option<BatchConfig> {
        if self.persistence_mode() == ProcessingMode::Batch {
            Some(BatchConfig {
                batch_size: self.config.consumers.persistence.consumer.batch_size.unwrap_or(self.config.batch.batch_size),
                batch_timeout_ms: self.config.consumers.persistence.consumer.batch_timeout_ms.unwrap_or(self.config.batch.batch_timeout_ms),
                max_batch_delay_ms: self.config.batch.max_batch_delay_ms,
            })
        } else {
            None
        }
    }

    /// 获取流处理配置
    pub fn stream_config(&self) -> &StreamConfig {
        &self.config.stream
    }

    /// 获取持久化配置
    pub fn persistence_config(&self) -> &PersistenceConfig {
        &self.config.consumers.persistence
    }

    /// 检查模块是否启用
    pub fn is_calculation_enabled(&self) -> bool {
        self.config.consumers.calculation.enabled
    }

    pub fn is_order_enabled(&self) -> bool {
        self.config.consumers.order.enabled
    }

    pub fn is_persistence_enabled(&self) -> bool {
        self.config.consumers.persistence.consumer.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_mode_serialization() {
        let mode = ProcessingMode::Batch;
        let serialized = serde_json::to_string(&mode).unwrap();
        assert_eq!(serialized, "\"batch\"");
        
        let deserialized: ProcessingMode = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, ProcessingMode::Batch);
    }

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.batch_timeout_ms, 1000);
        assert_eq!(config.max_batch_delay_ms, 5000);
    }

    #[test]
    fn test_processing_strategy() {
        let config = ProcessingConfig::default();
        let strategy = ProcessingStrategy::new(config);
        
        assert_eq!(strategy.global_mode(), &ProcessingMode::Batch);
        assert_eq!(strategy.calculation_mode(), ProcessingMode::Batch);
        assert_eq!(strategy.order_mode(), ProcessingMode::Batch);
        assert_eq!(strategy.persistence_mode(), ProcessingMode::Batch);
    }
} 