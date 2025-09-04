use crate::common::ts::{Strategy, IsClosed, Symbol, SymbolEnum, SymbolSetter};
use crate::common::signal::TradingSignal;
use crate::common::TradingSymbol;
use crate::strategy::macd::MacdStrategy;
use crate::strategy::bollinger::BollingerStrategy;
use crate::strategy::q1::Q1Strategy;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio::task;
use ta::{Close, High, Low, Open, Tbbav, Tbqav};
use tracing::{info, warn, error};

pub struct IdGenerator {
    base: u64,
    range_size: u64,
    counter: AtomicU64,
}

impl IdGenerator {
    pub fn new(range: (u64, u64)) -> Result<Self> {
        if range.0 >= range.1 {
            return Err(anyhow::anyhow!("range.0 must be less than range.1"));
        }
        Ok(Self {
            base: range.0,
            range_size: range.1 - range.0,
            counter: AtomicU64::new(0),
        })
    }

    pub fn next_id(&self) -> u64 {
        let seq = self.counter.fetch_add(1, Ordering::Relaxed);
        self.base + (seq % self.range_size)
    }
}

/// 单个品种的策略管理器
/// 负责管理特定品种的所有策略
pub struct SymbolStrategyManager<T: Close + High + Open + Low + Tbbav + Tbqav + Send + Sync + IsClosed + Symbol + 'static> {
    symbol: TradingSymbol,
    strategies: Vec<StrategyEnum>,
    data_receiver: mpsc::Receiver<Arc<T>>,
    strategy_receiver: mpsc::Receiver<StrategyEnum>,
    signal_sender: mpsc::Sender<TradingSignal>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone)]
pub enum StrategyEnum {
    Macd(MacdStrategy),
    Bollinger(BollingerStrategy),
    Q1(Q1Strategy),
    // 可以在这里添加更多策略类型
    // Rsi(RsiStrategy),
}

impl<T> Strategy<&T> for StrategyEnum
where
    T: High + Low + Close + Open + Tbbav + Tbqav + IsClosed,
{
    type Output = Option<TradingSignal>;

    fn on_kline_update(&mut self, input: &T) -> Self::Output {
        match self {
            StrategyEnum::Macd(strategy) => strategy.on_kline_update(input),
            StrategyEnum::Bollinger(strategy) => strategy.on_kline_update(input),
            StrategyEnum::Q1(strategy) => strategy.on_kline_update(input),
        }
    }

    fn name(&self) -> String {
        match self {
            StrategyEnum::Macd(_) => "MACD".to_string(),
            StrategyEnum::Bollinger(_) => "BOLLINGER".to_string(),
            StrategyEnum::Q1(_) => "Q1".to_string(),
        }
    }
}

impl StrategyEnum {
    pub fn name(&self) -> String {
        match self {
            StrategyEnum::Macd(_) => "MACD".to_string(),
            StrategyEnum::Bollinger(_) => "BOLLINGER".to_string(),
            StrategyEnum::Q1(_) => "Q1".to_string(),
        }
    }
}

// 为 StrategyEnum 实现 SymbolSetter trait
impl SymbolSetter for StrategyEnum {
    fn set_symbol(&mut self, symbol: TradingSymbol) {
        match self {
            StrategyEnum::Macd(_strategy) => {
                // MACD 策略可能还没有实现 SymbolSetter，暂时跳过
                // _strategy.set_symbol(symbol);
            },
            StrategyEnum::Bollinger(strategy) => {
                strategy.set_symbol(symbol);
            },
            StrategyEnum::Q1(strategy) => {
                strategy.set_symbol(symbol);
            },
        }
    }
}

impl<T> Strategy<Arc<T>> for StrategyEnum
where
    T: High + Low + Close + Open + Tbbav + Tbqav + Send + Sync + IsClosed + 'static,
{
    type Output = Option<TradingSignal>;

    fn on_kline_update(&mut self, input: Arc<T>) -> Self::Output {
        match self {
            StrategyEnum::Macd(strategy) => strategy.on_kline_update(input.as_ref()),
            StrategyEnum::Bollinger(strategy) => strategy.on_kline_update(input.as_ref()),
            StrategyEnum::Q1(strategy) => strategy.on_kline_update(input.as_ref()),
        }
    }

    fn name(&self) -> String {
        match self {
            StrategyEnum::Macd(_) => "MACD".to_string(),
            StrategyEnum::Bollinger(_) => "BOLLINGER".to_string(),
            StrategyEnum::Q1(_) => "Q1".to_string(),
        }
    }
}

/// 全局策略管理器
/// 负责接收K线数据并分发到对应的品种策略管理器
pub struct StrategyManager<T: Close + High + Open + Low + Tbbav + Tbqav + Send + Sync + IsClosed + Symbol + SymbolEnum + 'static> {
    /// 品种策略管理器映射表
    symbol_managers: HashMap<TradingSymbol, SymbolManagerInfo<T>>,
    /// 数据接收器 - 从数据层接收K线数据
    data_receiver: mpsc::Receiver<Arc<T>>,
    /// 信号发送器 - 将策略信号发送到订单管理器
    signal_sender: mpsc::Sender<TradingSignal>,
    /// ID生成器
    id_generator: Arc<IdGenerator>,
    /// 是否正在运行
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

/// 品种管理器信息
struct SymbolManagerInfo<T: Close + High + Open + Low + Tbbav + Tbqav + Send + Sync + IsClosed + Symbol + 'static> {
    /// 数据发送器 - 向品种策略管理器发送数据
    data_sender: mpsc::Sender<Arc<T>>,
    /// 策略发送器 - 向品种策略管理器发送策略
    strategy_sender: mpsc::Sender<StrategyEnum>,
    /// 任务句柄
    thread_handle: Option<tokio::task::JoinHandle<()>>,
}

impl<T: Close + High + Open + Low + Tbbav + Tbqav + Send + Sync + IsClosed + Symbol + SymbolEnum + 'static> StrategyManager<T> {
    pub fn new(
        data_receiver: mpsc::Receiver<Arc<T>>, 
        signal_sender: mpsc::Sender<TradingSignal>,
        id_generator: Arc<IdGenerator>
    ) -> Self {
        Self {
            symbol_managers: HashMap::new(),
            data_receiver,
            signal_sender,
            id_generator,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// 为指定品种添加策略
    pub async fn add_strategy(&mut self, symbol: TradingSymbol, mut strategy: StrategyEnum) -> Result<()> {
        // 🚀 自动设置策略的交易符号！
        strategy.set_symbol(symbol.clone());
        
        if !self.symbol_managers.contains_key(&symbol) {
            self.create_symbol_manager(symbol.clone())?;
        }
        
        // 向品种管理器发送策略
        if let Some(manager_info) = self.symbol_managers.get(&symbol) {
            if let Err(e) = manager_info.strategy_sender.send(strategy.clone()).await {
                return Err(anyhow::anyhow!("Failed to send strategy to symbol manager for {}: {}", symbol.as_str(), e));
            }
            info!("Added strategy {} for symbol {} (auto-configured)", strategy.name(), symbol.as_str());
        } else {
            return Err(anyhow::anyhow!("Symbol manager not found for {}", symbol.as_str()));
        }
        
        Ok(())
    }

    /// 创建品种策略管理器
    fn create_symbol_manager(&mut self, symbol: TradingSymbol) -> Result<()> {
        let (data_sender, data_receiver) = mpsc::channel::<Arc<T>>(1000);
        let (strategy_sender, strategy_receiver) = mpsc::channel::<StrategyEnum>(100);
        
        // 直接使用主信号发送器，不再创建中间通道
        let main_signal_sender = self.signal_sender.clone();
        
        let symbol_manager = SymbolStrategyManager::new(
            symbol.clone(),
            data_receiver,
            strategy_receiver,
            main_signal_sender,
        );
        
        // 启动品种管理器任务
        let task_handle = symbol_manager.start_task();
        
        let manager_info = SymbolManagerInfo {
            data_sender,
            strategy_sender,
            thread_handle: Some(task_handle),
        };
        
        self.symbol_managers.insert(symbol.clone(), manager_info);
        info!("Created symbol manager for {}", symbol.as_str());
        
        Ok(())
    }

    /// 启动策略管理器 - 主循环
    pub async fn run(&mut self) -> Result<()> {
        self.is_running.store(true, std::sync::atomic::Ordering::Relaxed);
        info!("Strategy manager started");
        
        // 信号现在直接从各个 SymbolStrategyManager 发送到主 signal_sender
        info!("Strategies will send signals directly to main signal handler");
        
        // 主数据分发循环
        while self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
            match self.data_receiver.recv().await {
                Some(data) => {
                    let symbol = data.symbol_enum().clone(); // 克隆符号用于查找
                    
                    // 将数据分发到对应的品种管理器
                    if let Some(manager_info) = self.symbol_managers.get(&symbol) {
                        if let Err(e) = manager_info.data_sender.send(data).await {
                            warn!("Failed to send data to symbol manager for {}: {}", symbol.as_str(), e);
                        }
                    } else {
                        warn!("No strategy manager found for symbol: {}. Available symbols: {:?}", 
                            symbol.as_str(), 
                            self.symbol_managers.keys().map(|k| k.as_str()).collect::<Vec<_>>());
                    }
                },
                None => {
                    error!("Data receiver disconnected");
                    break;
                }
            }
        }
        
        info!("Strategy manager stopped");
        Ok(())
    }


    
    /// 停止策略管理器
    pub fn stop(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// 获取所有品种的策略列表
    pub fn list_strategies(&self) -> HashMap<TradingSymbol, Vec<String>> {
        let mut result = HashMap::new();
        for (symbol, _) in &self.symbol_managers {
            // TODO: 实现获取每个品种的策略列表
            result.insert(symbol.clone(), vec!["MACD".to_string()]);
        }
        result
    }
    
    /// 获取所有支持的品种
    pub fn get_supported_symbols(&self) -> Vec<TradingSymbol> {
        self.symbol_managers.keys().cloned().collect()
    }
}

// 为 SymbolStrategyManager 实现方法
impl<T: Close + High + Open + Low + Tbbav + Tbqav + Send + Sync + IsClosed + Symbol + 'static> SymbolStrategyManager<T> {
    pub fn new(
        symbol: TradingSymbol,
        data_receiver: mpsc::Receiver<Arc<T>>,
        strategy_receiver: mpsc::Receiver<StrategyEnum>,
        signal_sender: mpsc::Sender<TradingSignal>,
    ) -> Self {
        Self {
            symbol,
            strategies: Vec::new(),
            data_receiver,
            strategy_receiver,
            signal_sender,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// 添加策略
    pub fn add_strategy(&mut self, strategy: StrategyEnum) {
        let strategy_name = strategy.name();
        self.strategies.push(strategy);
        info!("Added strategy {} for symbol {}", strategy_name, self.symbol.as_str());
    }
    
    /// 启动品种策略管理器任务
    pub fn start_task(mut self) -> tokio::task::JoinHandle<()> {
        task::spawn(async move {
            if let Err(e) = self.run().await {
                error!("Symbol strategy manager error for {}: {}", self.symbol.as_str(), e);
            }
        })
    }
    
    /// 运行品种策略管理器
    async fn run(&mut self) -> Result<()> {
        self.is_running.store(true, std::sync::atomic::Ordering::Relaxed);
        info!("Symbol strategy manager started for {}", self.symbol.as_str());
        
        while self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
            // 使用 tokio 通道处理
            
            // 首先尝试接收新策略
            match self.strategy_receiver.try_recv() {
                Ok(strategy) => {
                    let strategy_name = strategy.name();
                    self.strategies.push(strategy);
                    info!("Added strategy {} for symbol {}", strategy_name, self.symbol.as_str());
                },
                Err(mpsc::error::TryRecvError::Empty) => {
                    // 没有新策略，继续处理数据
                },
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    warn!("Strategy receiver disconnected for {}", self.symbol.as_str());
                }
            }
            
            // 处理数据
            match tokio::time::timeout(std::time::Duration::from_millis(100), self.data_receiver.recv()).await {
                Ok(Some(data)) => {
                    // 将数据分发给所有策略
                    for strategy in &mut self.strategies {
                        if let Some(signal) = strategy.on_kline_update(data.clone()) {
                            info!("🎯 策略 {} 生成信号: 交易对={}, 方向={:?}, 数量={}", 
                                strategy.name(), signal.symbol, signal.side, signal.quantity);
                            if let Err(e) = self.signal_sender.send(signal).await {
                                warn!("Failed to send signal for {}: {}", self.symbol.as_str(), e);
                            } else {
                                info!("✅ 信号已发送到主管理器: 交易对={}", self.symbol.as_str());
                            }
                        }
                    }
                },
                Ok(None) => {
                    // 通道关闭
                    if self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
                        error!("Data receiver disconnected for {}", self.symbol.as_str());
                    }
                    break;
                },
                Err(_) => {
                    // 超时，继续循环
                    continue;
                }
            }
        }
        
        info!("Symbol strategy manager stopped for {}", self.symbol.as_str());
        Ok(())
    }
    
    /// 停止策略管理器
    pub fn stop(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// 获取策略列表
    pub fn list_strategies(&self) -> Vec<String> {
        self.strategies.iter().map(|s| s.name()).collect()
    }
    
    /// 获取品种
    pub fn get_symbol(&self) -> &TradingSymbol {
        &self.symbol
    }
}
