#!/bin/bash

# 交易策略程序后台启动脚本

# 检查参数
if [ -z "$1" ]; then
    echo "使用方法: $0 <策略名称>"
    echo "支持的策略:"
    echo "  - bollinger: 布林带策略"
    echo "  - q1: Q1策略（默认）"
    exit 1
fi

STRATEGY_NAME="$1"
STRATEGY_DISPLAY_NAME=""

# 设置策略显示名称
case "$STRATEGY_NAME" in
    "bollinger")
        STRATEGY_DISPLAY_NAME="布林带"
        ;;
    "q1")
        STRATEGY_DISPLAY_NAME="Q1"
        ;;
    *)
        echo "❌ 不支持的策略: $STRATEGY_NAME"
        exit 1
        ;;
esac

# 设置工作目录为项目根目录（scripts的上级目录）
cd "$(dirname "$0")/.."

# 创建logs目录（如果不存在）
mkdir -p logs

# 设置PID文件路径
PID_FILE="logs/${STRATEGY_NAME}_strategy.pid"

echo "🚀 启动${STRATEGY_DISPLAY_NAME}策略程序..."
echo "📁 当前工作目录: $(pwd)"
echo "📁 日志目录: logs/"
echo "📋 分类日志文件:"
echo "   • main.log - 主要系统日志"
echo "   • signals.log - 交易信号日志"
echo "   • orders.log - 订单执行日志"
echo "   • websocket.log - WebSocket数据流日志"
echo "🆔 PID文件: $PID_FILE"

# 检查是否已经在运行
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if ps -p $PID > /dev/null 2>&1; then
        echo "⚠️  程序已经在运行 (PID: $PID)"
        echo "   如需重启，请先停止当前程序: ./scripts/stop_strategy.sh $STRATEGY_NAME"
        exit 1
    else
        echo "🧹 清理过期的PID文件"
        rm -f "$PID_FILE"
    fi
fi

# 在后台启动程序（不需要重定向输出，因为程序内部已经处理日志）
nohup cargo run -- $STRATEGY_NAME > /dev/null 2>&1 &

# 保存PID
echo $! > "$PID_FILE"

echo "✅ 程序已启动 (PID: $(cat $PID_FILE))"
echo "📊 实时日志查看命令:"
echo "   • 主要日志: tail -f logs/main.log"
echo "   • 信号日志: tail -f logs/signals.log"
echo "   • 订单日志: tail -f logs/orders.log"
echo "   • WebSocket日志: tail -f logs/websocket.log"
echo "🛑 停止程序: ./scripts/stop_strategy.sh $STRATEGY_NAME"
echo "📋 查看状态: ./scripts/status_strategy.sh $STRATEGY_NAME"
