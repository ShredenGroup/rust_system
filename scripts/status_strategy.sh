#!/bin/bash

# 交易策略程序状态查看脚本

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

echo "📊 ${STRATEGY_DISPLAY_NAME}策略程序状态检查"
echo "=================================================="

if [ ! -f "$PID_FILE" ]; then
    echo "❌ 程序未运行 (PID文件不存在)"
    exit 0
fi

PID=$(cat "$PID_FILE")

if ps -p $PID > /dev/null 2>&1; then
    echo "✅ 程序正在运行"
    echo "🆔 PID: $PID"
    echo "⏰ 启动时间: $(ps -o lstart= -p $PID)"
    echo "💾 内存使用: $(ps -o rss= -p $PID | awk '{print $1/1024 " MB"}')"
    echo "🔄 CPU使用: $(ps -o %cpu= -p $PID)%"
    
    # 显示最新的日志
    echo ""
    echo "📋 最新日志 (最后10行):"
    echo "------------------------------"
    if [ -d "logs" ]; then
        LATEST_LOG=$(ls -t logs/${STRATEGY_NAME}_strategy_*.log 2>/dev/null | head -1)
        if [ -n "$LATEST_LOG" ]; then
            tail -10 "$LATEST_LOG"
        else
            echo "暂无日志文件"
        fi
    fi
else
    echo "❌ 程序未运行 (PID: $PID 不存在)"
    echo "🧹 清理过期的PID文件"
    rm -f "$PID_FILE"
fi

echo ""
echo "📁 日志文件列表:"
if [ -d "logs" ]; then
    ls -la logs/${STRATEGY_NAME}_strategy_*.log 2>/dev/null | head -5
else
    echo "logs目录不存在"
fi

echo ""
echo "🔄 重启程序: ./scripts/start_strategy.sh $STRATEGY_NAME"
echo "🛑 停止程序: ./scripts/stop_strategy.sh $STRATEGY_NAME"
