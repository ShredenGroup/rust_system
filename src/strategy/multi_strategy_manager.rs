// 1. 全局管理器
pub struct MultiSymbolStrategyManager {
    // 品种 -> 策略管理器
    symbol_managers: HashMap<String, SymbolStrategyManager>,
    // 数据分发器：WebSocket -> 各品种
    data_distributors: HashMap<String, mpsc::Sender<Arc<KlineInfo>>>,
    // 全局信号收集器
    global_signal_receiver: mpsc::Receiver<EnrichedSignal>,
    global_signal_sender: mpsc::Sender<EnrichedSignal>,
}

// 2. 单品种策略管理器
pub struct SymbolStrategyManager {
    symbol: String,
    strategies: Vec<StrategyEnum>,
    data_receiver: mpsc::Receiver<Arc<KlineInfo>>,
    signal_sender: mpsc::Sender<EnrichedSignal>,
}

// 3. 增强的信号类型
#[derive(Debug, Clone)]
pub struct EnrichedSignal {
    pub symbol: String,           // 品种
    pub strategy_name: String,    // 策略名
    pub signal: Signal,           // 原始信号
    pub metadata: SignalMetadata, // 元数据
}

// 4. 策略配置
#[derive(Debug, Clone)]
pub struct StrategyConfig {
    pub symbol: String,
    pub strategy_type: StrategyType,
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum StrategyType {
    Macd { period: usize },
    Rsi { period: usize },
    Bollinger { period: usize, std_dev: f64 },
}