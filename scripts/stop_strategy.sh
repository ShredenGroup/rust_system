#!/bin/bash

# 交易策略程序停止脚本

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

PID_FILE="logs/${STRATEGY_NAME}_strategy.pid"

if [ ! -f "$PID_FILE" ]; then
    echo "⚠️  PID文件不存在，程序可能没有运行"
    exit 1
fi

PID=$(cat "$PID_FILE")

if ! ps -p $PID > /dev/null 2>&1; then
    echo "🧹 程序未运行，清理PID文件"
    rm -f "$PID_FILE"
    exit 0
fi

echo "🛑 正在停止${STRATEGY_DISPLAY_NAME}策略程序 (PID: $PID)..."

# 尝试优雅停止
kill $PID

# 等待程序停止
sleep 2

# 检查是否已停止
if ps -p $PID > /dev/null 2>&1; then
    echo "⚠️  程序未响应，强制停止..."
    kill -9 $PID
    sleep 1
fi

# 最终检查
if ps -p $PID > /dev/null 2>&1; then
    echo "❌ 无法停止程序 (PID: $PID)"
    exit 1
else
    echo "✅ 程序已停止"
    rm -f "$PID_FILE"
fi

echo ""
echo "🔄 重新启动: ./scripts/start_strategy.sh $STRATEGY_NAME"
