#!/bin/bash

# 布林带策略测试程序停止脚本

# 设置工作目录
cd "$(dirname "$0")"

PID_FILE="logs/bollinger_test.pid"

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

echo "🛑 正在停止布林带策略测试程序 (PID: $PID)..."

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