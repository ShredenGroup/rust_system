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
    echo "📋 最新日志 (最后5行):"
    echo "------------------------------"
    if [ -d "logs" ]; then
        if [ -f "logs/main.log" ]; then
            echo "📄 主要日志:"
            tail -5 logs/main.log
        fi
        if [ -f "logs/signals.log" ] && [ -s "logs/signals.log" ]; then
            echo ""
            echo "📄 信号日志:"
            tail -5 logs/signals.log
        fi
        if [ -f "logs/orders.log" ] && [ -s "logs/orders.log" ]; then
            echo ""
            echo "📄 订单日志:"
            tail -5 logs/orders.log
        fi
    fi
else
    echo "❌ 程序未运行 (PID: $PID 不存在)"
    echo "🧹 清理过期的PID文件"
    rm -f "$PID_FILE"
fi

echo ""
echo "📁 分类日志文件状态:"
if [ -d "logs" ]; then
    echo "📄 main.log: $(ls -lh logs/main.log 2>/dev/null | awk '{print $5}' || echo '不存在')"
    echo "📄 signals.log: $(ls -lh logs/signals.log 2>/dev/null | awk '{print $5}' || echo '不存在')"
    echo "📄 orders.log: $(ls -lh logs/orders.log 2>/dev/null | awk '{print $5}' || echo '不存在')"
    echo "📄 websocket.log: $(ls -lh logs/websocket.log 2>/dev/null | awk '{print $5}' || echo '不存在')"
else
    echo "logs目录不存在"
fi

echo ""
echo "🔄 重启程序: ./scripts/start_strategy.sh $STRATEGY_NAME"
echo "🛑 停止程序: ./scripts/stop_strategy.sh $STRATEGY_NAME"
