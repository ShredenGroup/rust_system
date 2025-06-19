#!/usr/bin/env python3
"""
Binance WebSocket 连接延迟测试脚本
测试 WebSocket 连接建立时间和消息接收延迟
"""

import asyncio
import websockets
import time
import statistics
import json
from typing import List, Dict
import argparse

class WebSocketLatencyTester:
    def __init__(self, base_url: str = "wss://fstream.binance.com"):
        """
        初始化 WebSocket 延迟测试器
        
        Args:
            base_url: Binance WebSocket 基础URL
        """
        self.base_url = base_url
    
    async def test_connection_latency(self, symbol: str, interval: str, count: int = 10) -> Dict:
        """
        测试 WebSocket 连接延迟
        
        Args:
            symbol: 交易对符号，如 "btcusdt"
            interval: K线间隔，如 "1m"
            count: 测试次数
            
        Returns:
            包含延迟统计的字典
        """
        stream_name = f"{symbol}@kline_{interval}"
        ws_url = f"{self.base_url}/ws/{stream_name}"
        
        print(f"测试 WebSocket 连接延迟: {ws_url}")
        print(f"测试次数: {count}")
        print("-" * 60)
        
        connection_times = []
        message_latencies = []
        successful_connections = 0
        failed_connections = 0
        
        for i in range(count):
            print(f"测试 #{i + 1}/{count}...")
            
            # 测试连接建立时间
            connection_start = time.time()
            try:
                async with websockets.connect(ws_url, ping_interval=None, ping_timeout=None) as websocket:
                    connection_end = time.time()
                    connection_time_ms = (connection_end - connection_start) * 1000
                    connection_times.append(connection_time_ms)
                    successful_connections += 1
                    
                    print(f"  连接建立: {connection_time_ms:.2f}ms")
                    
                    # 接收几条消息测试消息延迟
                    message_count = 0
                    max_messages = 3
                    
                    while message_count < max_messages:
                        try:
                            # 设置接收超时
                            message_start = time.time()
                            message = await asyncio.wait_for(websocket.recv(), timeout=5.0)
                            message_end = time.time()
                            
                            # 解析消息获取服务器时间
                            try:
                                data = json.loads(message)
                                if 'E' in data:  # 事件时间
                                    server_time = data['E']
                                    current_time = int(time.time() * 1000)
                                    latency = current_time - server_time
                                    message_latencies.append(latency)
                                    
                                    print(f"  消息 #{message_count + 1}: 延迟 {latency}ms")
                            except json.JSONDecodeError:
                                pass
                            
                            message_count += 1
                            
                        except asyncio.TimeoutError:
                            print(f"  消息接收超时")
                            break
                    
                    print(f"  连接 #{i + 1} 完成")
                    
            except Exception as e:
                failed_connections += 1
                print(f"  连接失败: {e}")
            
            # 连接间隔
            if i < count - 1:
                await asyncio.sleep(1.0)
        
        # 计算统计信息
        stats = {
            'total_connections': count,
            'successful_connections': successful_connections,
            'failed_connections': failed_connections,
            'success_rate': successful_connections / count * 100 if count > 0 else 0
        }
        
        if connection_times:
            stats['connection_stats'] = {
                'min': min(connection_times),
                'max': max(connection_times),
                'mean': statistics.mean(connection_times),
                'median': statistics.median(connection_times),
                'std': statistics.stdev(connection_times) if len(connection_times) > 1 else 0
            }
        
        if message_latencies:
            stats['message_stats'] = {
                'min': min(message_latencies),
                'max': max(message_latencies),
                'mean': statistics.mean(message_latencies),
                'median': statistics.median(message_latencies),
                'std': statistics.stdev(message_latencies) if len(message_latencies) > 1 else 0
            }
        
        return stats
    
    def print_results(self, stats: Dict):
        """打印测试结果"""
        print("\n" + "=" * 60)
        print("WebSocket 连接延迟测试结果")
        print("=" * 60)
        
        print(f"总连接数: {stats['total_connections']}")
        print(f"成功连接: {stats['successful_connections']}")
        print(f"失败连接: {stats['failed_connections']}")
        print(f"成功率: {stats['success_rate']:.2f}%")
        
        if 'connection_stats' in stats:
            conn = stats['connection_stats']
            print(f"\n连接建立延迟 (毫秒):")
            print(f"  最小值: {conn['min']:.2f}")
            print(f"  最大值: {conn['max']:.2f}")
            print(f"  平均值: {conn['mean']:.2f}")
            print(f"  中位数: {conn['median']:.2f}")
            print(f"  标准差: {conn['std']:.2f}")
        
        if 'message_stats' in stats:
            msg = stats['message_stats']
            print(f"\n消息接收延迟 (毫秒):")
            print(f"  最小值: {msg['min']:.2f}")
            print(f"  最大值: {msg['max']:.2f}")
            print(f"  平均值: {msg['mean']:.2f}")
            print(f"  中位数: {msg['median']:.2f}")
            print(f"  标准差: {msg['std']:.2f}")
        
        print("=" * 60)

async def main():
    parser = argparse.ArgumentParser(description='Binance WebSocket 连接延迟测试')
    parser.add_argument('--symbol', type=str, default='btcusdt', help='交易对符号 (默认: btcusdt)')
    parser.add_argument('--interval', type=str, default='1m', help='K线间隔 (默认: 1m)')
    parser.add_argument('--count', type=int, default=5, help='测试次数 (默认: 5)')
    parser.add_argument('--url', type=str, default='wss://fstream.binance.com', 
                       help='WebSocket URL (默认: wss://fstream.binance.com)')
    
    args = parser.parse_args()
    
    print("Binance WebSocket 连接延迟测试工具")
    print(f"WebSocket URL: {args.url}")
    print(f"交易对: {args.symbol}")
    print(f"间隔: {args.interval}")
    print(f"测试次数: {args.count}")
    print("-" * 60)
    
    tester = WebSocketLatencyTester(args.url)
    stats = await tester.test_connection_latency(args.symbol, args.interval, args.count)
    tester.print_results(stats)
    
    # 延迟评估
    if 'connection_stats' in stats:
        avg_connection = stats['connection_stats']['mean']
        if avg_connection < 100:
            print("✅ 连接延迟表现优秀！")
        elif avg_connection < 500:
            print("⚠️  连接延迟表现一般")
        else:
            print("❌ 连接延迟过高")
    
    if 'message_stats' in stats:
        avg_message = stats['message_stats']['mean']
        if avg_message < 50:
            print("✅ 消息延迟表现优秀！")
        elif avg_message < 200:
            print("⚠️  消息延迟表现一般")
        else:
            print("❌ 消息延迟过高")

if __name__ == "__main__":
    asyncio.run(main()) 