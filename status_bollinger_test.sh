#!/bin/bash

# 布林带策略测试程序状态查看脚本

# 设置工作目录
cd "$(dirname "$0")"

PID_FILE="logs/bollinger_test.pid"

echo "📊 布林带策略测试程序状态检查"
echo "=" * 50

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
    echo "-" * 30
    if [ -d "logs" ]; then
        LATEST_LOG=$(ls -t logs/bollinger_test_*.log 2>/dev/null | head -1)
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
    ls -la logs/bollinger_test_*.log 2>/dev/null | head -5
else
    echo "logs目录不存在"
fi 