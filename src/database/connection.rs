use anyhow::Result;
use redis::Client;
use redis::aio::MultiplexedConnection;
use sea_orm::{Database, DatabaseConnection};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DatabaseManager {
    conn: Arc<DatabaseConnection>,
}

#[derive(Clone)]
pub struct RedisManager {
    redis_conn: Arc<Mutex<MultiplexedConnection>>,
}

impl DatabaseManager {
    pub async fn new(database_url: &str) -> Result<Self> {
        let conn = Database::connect(database_url).await?;
        Ok(DatabaseManager {
            conn: Arc::new(conn),
        })
    }
}

impl RedisManager {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        let redis_conn = client.get_multiplexed_tokio_connection().await?;
        Ok(RedisManager {
            redis_conn: Arc::new(Mutex::new(redis_conn)),
        })
    }

    // 获取 Redis 连接的可变引用（异步版本）
    pub async fn get_connection_mut(&self) -> tokio::sync::MutexGuard<MultiplexedConnection> {
        self.redis_conn.lock().await
    }
}
