#!/bin/bash

# 布林带策略测试程序后台启动脚本

# 设置工作目录
cd "$(dirname "$0")"

# 创建logs目录（如果不存在）
mkdir -p logs

# 设置日志文件路径
LOG_FILE="logs/bollinger_test_$(date +%Y%m%d_%H%M%S).log"
PID_FILE="logs/bollinger_test.pid"

echo "🚀 启动布林带策略测试程序..."
echo "📁 日志文件: $LOG_FILE"
echo "🆔 PID文件: $PID_FILE"

# 检查是否已经在运行
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if ps -p $PID > /dev/null 2>&1; then
        echo "⚠️  程序已经在运行 (PID: $PID)"
        echo "   如需重启，请先停止当前程序: ./stop_bollinger_test.sh"
        exit 1
    else
        echo "🧹 清理过期的PID文件"
        rm -f "$PID_FILE"
    fi
fi

# 在后台启动程序
nohup cargo run --bin bollinger_test > "$LOG_FILE" 2>&1 &

# 保存PID
echo $! > "$PID_FILE"

echo "✅ 程序已启动 (PID: $(cat $PID_FILE))"
echo "📊 实时日志查看: tail -f $LOG_FILE"
echo "🛑 停止程序: ./stop_bollinger_test.sh"
echo "📋 查看状态: ./status_bollinger_test.sh" 