use anyhow::Result;
use std::fs;
use tracing::info;
use tracing_subscriber::{
    fmt::{self, format::Writer, time::FormatTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

/// 自定义时间格式
struct CustomTimeFormat;

impl FormatTime for CustomTimeFormat {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", chrono::Local::now().format("%H:%M:%S%.3f"))
    }
}

/// 简化的日志配置
pub struct SimpleLoggingConfig {
    pub log_dir: String,
    pub enable_console: bool,
}

impl Default for SimpleLoggingConfig {
    fn default() -> Self {
        Self {
            log_dir: "logs".to_string(),
            enable_console: true,
        }
    }
}

/// 简化的日志管理器
pub struct SimpleLoggingManager {
    config: SimpleLoggingConfig,
}

impl SimpleLoggingManager {
    pub fn new(config: SimpleLoggingConfig) -> Self {
        Self { config }
    }

    /// 初始化简化的日志系统
    pub fn init(&self) -> Result<()> {
        // 获取当前工作目录并创建绝对路径
        let current_dir = std::env::current_dir()?;
        let log_dir = current_dir.join(&self.config.log_dir);
        
        // 创建日志目录
        if !log_dir.exists() {
            fs::create_dir_all(&log_dir)?;
        }

        // 创建主要日志文件
        let main_appender = RollingFileAppender::new(
            Rotation::DAILY,
            &log_dir,
            "main.log",
        );
        
        let signal_appender = RollingFileAppender::new(
            Rotation::DAILY,
            &log_dir,
            "signals.log",
        );
        
        let order_appender = RollingFileAppender::new(
            Rotation::DAILY,
            &log_dir,
            "orders.log",
        );
        
        let websocket_appender = RollingFileAppender::new(
            Rotation::DAILY,
            &log_dir,
            "websocket.log",
        );

        // 创建过滤器 - 按target分离日志
        let main_filter = EnvFilter::new("info")
            .add_directive("signals=off".parse().unwrap())
            .add_directive("orders=off".parse().unwrap())
            .add_directive("websocket=off".parse().unwrap());
        
        let signal_filter = EnvFilter::new("off")
            .add_directive("signals=info".parse().unwrap());
        
        let order_filter = EnvFilter::new("off")
            .add_directive("orders=info".parse().unwrap());
        
        let websocket_filter = EnvFilter::new("off")
            .add_directive("websocket=info".parse().unwrap());

        // 创建各层的格式化器
        let main_layer = fmt::layer()
            .with_writer(main_appender)
            .with_timer(CustomTimeFormat)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false)
            .with_filter(main_filter);

        let signal_layer = fmt::layer()
            .with_writer(signal_appender)
            .with_timer(CustomTimeFormat)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false)
            .with_filter(signal_filter);

        let order_layer = fmt::layer()
            .with_writer(order_appender)
            .with_timer(CustomTimeFormat)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false)
            .with_filter(order_filter);

        let websocket_layer = fmt::layer()
            .with_writer(websocket_appender)
            .with_timer(CustomTimeFormat)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false)
            .with_filter(websocket_filter);

        // 控制台输出层（可选）
        let console_layer = if self.config.enable_console {
            Some(
                fmt::layer()
                    .with_timer(CustomTimeFormat)
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_file(false)
                    .with_line_number(false)
                    .with_filter(EnvFilter::new("info"))
            )
        } else {
            None
        };

        // 初始化订阅器
        let registry = tracing_subscriber::registry()
            .with(main_layer)
            .with(signal_layer)
            .with(order_layer)
            .with(websocket_layer);

        if let Some(console) = console_layer {
            registry.with(console).init();
        } else {
            registry.init();
        }

        info!("🚀 简化日志系统初始化完成");
        info!("📁 日志目录: {}", self.config.log_dir);
        info!("📋 日志文件:");
        info!("   • main.log - 主要系统日志");
        info!("   • signals.log - 交易信号");
        info!("   • orders.log - 订单执行");
        info!("   • websocket.log - WebSocket数据流（仅警告/错误）");

        Ok(())
    }
}

/// 便捷的日志宏，用于不同日志类型
#[macro_export]
macro_rules! signal_log {
    ($level:ident, $($arg:tt)*) => {
        tracing::$level!(target: "signals", $($arg)*);
    };
}

#[macro_export]
macro_rules! order_log {
    ($level:ident, $($arg:tt)*) => {
        tracing::$level!(target: "orders", $($arg)*);
    };
}

#[macro_export]
macro_rules! websocket_log {
    ($level:ident, $($arg:tt)*) => {
        tracing::$level!(target: "websocket", $($arg)*);
    };
}

#[macro_export]
macro_rules! strategy_log {
    ($level:ident, $($arg:tt)*) => {
        tracing::$level!(target: "strategy", $($arg)*);
    };
}

#[macro_export]
macro_rules! error_log {
    ($level:ident, $($arg:tt)*) => {
        tracing::$level!(target: "errors", $($arg)*);
    };
}

/// 系统日志宏
#[macro_export]
macro_rules! system_log {
    ($level:ident, $($arg:tt)*) => {
        tracing::$level!(target: "system", $($arg)*);
    };
}
