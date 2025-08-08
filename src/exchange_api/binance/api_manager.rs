use crate::common::config::api_config::{KlineApiConfig, ApiBaseConfig};  // 你需要创建这个配置结构
use crate::dto::binance::rest_api::{KlineRequest, KlineResponse};
use crate::exchange_api::binance::api::BinanceFuturesApi;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use serde::{Deserialize, Serialize};

/// API 消息类型
#[derive(Debug, Clone)]
pub enum ApiMessage {
    Kline(Arc<KlineResponse>),
    // 后续可以添加其他类型的API响应
}

/// API 任务类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApiTaskType {
    KlineHistory,
    // 后续可以添加其他类型的API任务
}

/// API 任务信息
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub task_id: String,
    pub task_type: ApiTaskType,
    pub symbol: String,
    pub interval: String,
    pub last_run: Option<i64>,
    pub next_run: Option<i64>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Running,
    Stopped,
    Failed(String),
}

pub struct ApiManager {
    api_client: BinanceFuturesApi,
    tasks: Arc<Mutex<HashMap<String, (JoinHandle<()>, TaskInfo)>>>,
    message_tx: mpsc::UnboundedSender<ApiMessage>,
}

impl ApiManager {
    pub fn new(api_key: String, secret_key: String, message_tx: mpsc::UnboundedSender<ApiMessage>) -> Self {
        Self {
            api_client: BinanceFuturesApi::new(api_key, secret_key),
            tasks: Arc::new(Mutex::new(HashMap::new())),
            message_tx,
        }
    }

    /// 启动K线历史数据定时获取任务
    pub async fn start_kline_history_task(&self, config: KlineApiConfig) -> Result<()> {
        let task_id = format!("kline_history_{}_{}", config.symbol, config.interval);
        let message_tx = self.message_tx.clone();
        let api_client = self.api_client.clone();
        let tasks = self.tasks.clone();
        let task_id_clone = task_id.clone();

        // 创建任务信息
        let task_info = TaskInfo {
            task_id: task_id.clone(),
            task_type: ApiTaskType::KlineHistory,
            symbol: config.symbol.clone(),
            interval: config.interval.clone(),
            last_run: None,
            next_run: None,
            status: TaskStatus::Running,
        };

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.interval_secs));

            loop {
                interval.tick().await;

                // 构建K线请求
                let request = KlineRequest {
                    symbol: config.symbol.clone(),
                    interval: config.interval.clone(),
                    start_time: None,
                    end_time: None,
                    limit: Some("500".to_string()),  // 修改为字符串类型
                };

                // 发送API请求
                match api_client.get_klines(&request).await {
                    Ok(response) => {
                        // 发送结果到通道
                        if let Err(e) = message_tx.send(ApiMessage::Kline(Arc::new(response))) {
                            eprintln!("Failed to send kline data: {}", e);
                            break;
                        }

                        // 更新任务状态
                        let mut tasks = tasks.lock().await;
                        if let Some((_, info)) = tasks.get_mut(&task_id_clone) {
                            info.last_run = Some(chrono::Utc::now().timestamp());
                            info.next_run = Some(chrono::Utc::now().timestamp() + config.interval_secs as i64);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to get kline data: {}", e);
                        // 更新任务状态为失败
                        let mut tasks = tasks.lock().await;
                        if let Some((_, info)) = tasks.get_mut(&task_id_clone) {
                            info.status = TaskStatus::Failed(e.to_string());
                        }
                    }
                }
            }
        });

        // 保存任务信息
        let mut tasks = self.tasks.lock().await;
        tasks.insert(task_id, (handle, task_info));

        Ok(())
    }

    /// 停止指定的任务
    pub async fn stop_task(&self, task_id: &str) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        
        if let Some((handle, _)) = tasks.remove(task_id) {
            handle.abort();
            println!("Stopped task: {}", task_id);
        }
        
        Ok(())
    }

    /// 停止所有任务
    pub async fn stop_all_tasks(&self) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        
        for (task_id, (handle, _)) in tasks.drain() {
            handle.abort();
            println!("Stopped task: {}", task_id);
        }
        
        Ok(())
    }

    /// 获取所有任务信息
    pub async fn list_tasks(&self) -> Vec<TaskInfo> {
        let tasks = self.tasks.lock().await;
        tasks.values().map(|(_, info)| info.clone()).collect()
    }

    /// 获取任务状态
    pub async fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        let tasks = self.tasks.lock().await;
        tasks.get(task_id).map(|(_, info)| info.status.clone())
    }

    /// 一次性获取历史K线数据
    pub async fn get_history_klines(
        &self,
        symbol: String,
        interval: String,
        start_time: Option<String>,
        end_time: Option<String>,
        limit: Option<String>,
    ) -> Result<()> {
        // 构建K线请求
        let request = KlineRequest {
            symbol,
            interval,
            start_time,
            end_time,
            limit,
        };

        // 发送API请求
        match self.api_client.get_klines(&request).await {
            Ok(response) => {
                // 发送结果到通道
                if let Err(e) = self.message_tx.send(ApiMessage::Kline(Arc::new(response))) {
                    eprintln!("Failed to send kline data: {}", e);
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to get kline data: {}", e);
                Err(e)
            }
        }
    }
}

// 便捷的工厂函数
pub async fn create_api_manager(
    api_key: String,
    secret_key: String,
) -> Result<(ApiManager, mpsc::UnboundedReceiver<ApiMessage>)> {
    let (tx, rx) = mpsc::unbounded_channel();
    let manager = ApiManager::new(api_key, secret_key, tx);
    Ok((manager, rx))
}

// 使用示例
pub async fn example_usage() -> Result<()> {
    let api_key = "your_api_key".to_string();
    let secret_key = "your_secret_key".to_string();
    
    let (manager, mut rx) = create_api_manager(api_key, secret_key).await?;
    
    // 配置K线数据获取任务
    let config = KlineApiConfig {
        symbol: "BTCUSDT".to_string(),
        interval: "1m".to_string(),
        interval_secs: 60,  // 每60秒获取一次数据
        enabled: true,
        base: ApiBaseConfig {
            auto_retry: true,
            max_retries: 3,
            retry_delay_secs: 5,
            timeout_secs: 30,
            tags: vec!["main".to_string()],
        },
    };
    
    // 启动定时任务
    manager.start_kline_history_task(config).await?;

    // 一次性获取历史数据
    manager.get_history_klines(
        "BTCUSDT".to_string(),
        "1m".to_string(),
        Some("1640995200000".to_string()),  // 开始时间
        None,  // 结束时间
        Some("500".to_string()),  // 限制数量
    ).await?;
    
    // 处理接收到的数据
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            match message {
                ApiMessage::Kline(kline_data) => {
                    println!("Received kline data: {:?}", kline_data);
                }
            }
        }
    });
    
    // 运行一段时间后停止
    tokio::time::sleep(Duration::from_secs(300)).await;
    manager.stop_all_tasks().await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    // 辅助函数：创建测试配置
    fn create_test_config() -> KlineApiConfig {
        KlineApiConfig {
            symbol: "BTCUSDT".to_string(),
            interval: "1m".to_string(),
            interval_secs: 1, // 测试用1秒间隔
            enabled: true,
            base: ApiBaseConfig {
                auto_retry: true,
                max_retries: 3,
                retry_delay_secs: 1,
                timeout_secs: 5,
                tags: vec!["test".to_string()],
            },
        }
    }

    #[tokio::test]
    async fn test_api_manager_creation() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let manager = ApiManager::new("test_key".to_string(), "test_secret".to_string(), tx);
        
        let tasks = manager.list_tasks().await;
        assert_eq!(tasks.len(), 0, "New manager should have no tasks");
    }

    #[tokio::test]
    async fn test_start_and_stop_task() {
        let (manager, mut rx) = create_api_manager(
            "test_key".to_string(),
            "test_secret".to_string(),
        ).await.unwrap();

        let config = create_test_config();
        let task_id = format!("kline_history_{}_{}", config.symbol, config.interval);

        // 启动任务
        manager.start_kline_history_task(config).await.unwrap();
        
        // 验证任务已创建
        let tasks = manager.list_tasks().await;
        assert_eq!(tasks.len(), 1, "Should have one task running");
        
        // 验证任务状态
        let status = manager.get_task_status(&task_id).await.unwrap();
        match status {
            TaskStatus::Running => (),
            _ => panic!("Task should be in Running state"),
        }

        // 等待接收一些数据
        let received_data = tokio::spawn(async move {
            let mut count = 0;
            let timeout = sleep(Duration::from_secs(5));
            tokio::pin!(timeout);

            loop {
                tokio::select! {
                    Some(msg) = rx.recv() => {
                        match msg {
                            ApiMessage::Kline(_) => count += 1,
                        }
                        if count >= 2 {
                            break count;
                        }
                    }
                    _ = &mut timeout => break count,
                }
            }
        }).await.unwrap();

        assert!(received_data > 0, "Should have received some kline data");

        // 停止任务
        manager.stop_task(&task_id).await.unwrap();
        
        // 验证任务已停止
        let tasks = manager.list_tasks().await;
        assert_eq!(tasks.len(), 0, "All tasks should be stopped");
    }

    #[tokio::test]
    async fn test_multiple_tasks() {
        let (manager, _rx) = create_api_manager(
            "test_key".to_string(),
            "test_secret".to_string(),
        ).await.unwrap();

        // 创建多个配置
        let configs = vec![
            KlineApiConfig {
                symbol: "BTCUSDT".to_string(),
                interval: "1m".to_string(),
                interval_secs: 1,
                enabled: true,
                base: ApiBaseConfig::default(),
            },
            KlineApiConfig {
                symbol: "ETHUSDT".to_string(),
                interval: "1m".to_string(),
                interval_secs: 1,
                enabled: true,
                base: ApiBaseConfig::default(),
            },
        ];

        // 启动多个任务
        for config in configs {
            manager.start_kline_history_task(config).await.unwrap();
        }

        // 验证任务数量
        let tasks = manager.list_tasks().await;
        assert_eq!(tasks.len(), 2, "Should have two tasks running");

        // 停止所有任务
        manager.stop_all_tasks().await.unwrap();
        
        // 验证所有任务已停止
        let tasks = manager.list_tasks().await;
        assert_eq!(tasks.len(), 0, "All tasks should be stopped");
    }

    #[tokio::test]
    async fn test_task_error_handling() {
        let (manager, _rx) = create_api_manager(
            "invalid_key".to_string(),  // 使用无效的API密钥
            "invalid_secret".to_string(),
        ).await.unwrap();

        let config = create_test_config();
        let task_id = format!("kline_history_{}_{}", config.symbol, config.interval);

        // 启动任务
        manager.start_kline_history_task(config).await.unwrap();

        // 等待一段时间让任务执行
        sleep(Duration::from_secs(2)).await;

        // 检查任务状态是否变为失败
        let status = manager.get_task_status(&task_id).await.unwrap();
        match status {
            TaskStatus::Failed(_) => (),
            _ => panic!("Task should be in Failed state"),
        }
    }

    #[tokio::test]
    async fn test_task_info_fields() {
        let (manager, _rx) = create_api_manager(
            "test_key".to_string(),
            "test_secret".to_string(),
        ).await.unwrap();

        let config = create_test_config();
        let task_id = format!("kline_history_{}_{}", config.symbol, config.interval);

        // 启动任务
        manager.start_kline_history_task(config.clone()).await.unwrap();

        // 获取任务信息
        let tasks = manager.list_tasks().await;
        let task = tasks.iter().find(|t| t.task_id == task_id).unwrap();

        // 验证任务信息字段
        assert_eq!(task.symbol, config.symbol);
        assert_eq!(task.interval, config.interval);
        assert_eq!(task.task_type, ApiTaskType::KlineHistory);
        
        // 停止任务
        manager.stop_task(&task_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_history_klines() {
        let (manager, mut rx) = create_api_manager(
            "test_key".to_string(),
            "test_secret".to_string(),
        ).await.unwrap();

        // 发送一次性请求
        manager.get_history_klines(
            "BTCUSDT".to_string(),
            "1m".to_string(),
            Some("1640995200000".to_string()),
            None,
            Some("10".to_string()),
        ).await.unwrap();

        // 等待接收数据
        let received_data = tokio::spawn(async move {
            if let Some(message) = rx.recv().await {
                match message {
                    ApiMessage::Kline(data) => {
                        assert!(!data.is_empty(), "Should receive some kline data");
                        true
                    }
                }
            } else {
                false
            }
        }).await.unwrap();

        assert!(received_data, "Should have received kline data");
    }
}
