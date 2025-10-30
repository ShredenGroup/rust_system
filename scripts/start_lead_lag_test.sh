#!/usr/bin/env bash
set -euo pipefail

# Absolute path to the project root
PROJECT_ROOT="/home/litterpigger/quant/rust_system"

cd "$PROJECT_ROOT"

# Ensure logs directory exists
mkdir -p "$PROJECT_ROOT/logs"

LOG_FILE="$PROJECT_ROOT/logs/lead_lag_test.log"
PID_FILE="$PROJECT_ROOT/logs/lead_lag_test.pid"

# Start in background with nohup; forward any extra args
nohup cargo run --release --bin lead_lag_test "$@" >> "$LOG_FILE" 2>&1 &
echo $! > "$PID_FILE"
echo "lead_lag_test started in background. PID=$(cat "$PID_FILE"), logging to $LOG_FILE"


