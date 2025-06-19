#!/usr/bin/env python3
"""
Binance 永续合约 API 延迟测试脚本
测试 /fapi/v1/time 接口的响应时间
"""

import requests
import time
import statistics
from typing import List, Dict
import json
from datetime import datetime
import argparse

class BinanceLatencyTester:
    def __init__(self, base_url: str = "https://fapi.binance.com"):
        """
        初始化延迟测试器
        
        Args:
            base_url: Binance API 基础URL
        """
        self.base_url = base_url
        self.session = requests.Session()
        # 设置请求头
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
            'Accept': 'application/json',
            'Content-Type': 'application/json'
        })
    
    def test_single_request(self) -> Dict:
        """
        测试单次请求的延迟
        
        Returns:
            包含延迟信息的字典
        """
        url = f"{self.base_url}/fapi/v1/time"
        
        start_time = time.time()
        try:
            response = self.session.get(url, timeout=10)
            end_time = time.time()
            
            latency_ms = (end_time - start_time) * 1000
            
            if response.status_code == 200:
                server_time = response.json().get('serverTime', 0)
                return {
                    'success': True,
                    'latency_ms': latency_ms,
                    'status_code': response.status_code,
                    'server_time': server_time,
                    'local_time': int(time.time() * 1000),
                    'time_diff_ms': abs(int(time.time() * 1000) - server_time)
                }
            else:
                return {
                    'success': False,
                    'latency_ms': latency_ms,
                    'status_code': response.status_code,
                    'error': f"HTTP {response.status_code}"
                }
                
        except requests.exceptions.Timeout:
            return {
                'success': False,
                'latency_ms': 10000,  # 10秒超时
                'error': 'Timeout'
            }
        except requests.exceptions.RequestException as e:
            return {
                'success': False,
                'latency_ms': 0,
                'error': str(e)
            }
    
    def test_multiple_requests(self, count: int = 100, interval: float = 0.1) -> Dict:
        """
        测试多次请求的延迟统计
        
        Args:
            count: 请求次数
            interval: 请求间隔（秒）
            
        Returns:
            包含统计信息的字典
        """
        print(f"开始测试 {count} 次请求，间隔 {interval} 秒...")
        
        latencies = []
        successful_requests = 0
        failed_requests = 0
        time_diffs = []
        
        for i in range(count):
            result = self.test_single_request()
            
            if result['success']:
                successful_requests += 1
                latencies.append(result['latency_ms'])
                if 'time_diff_ms' in result:
                    time_diffs.append(result['time_diff_ms'])
                
                # 显示进度
                if (i + 1) % 10 == 0:
                    print(f"进度: {i + 1}/{count} (成功率: {successful_requests/(i+1)*100:.1f}%)")
            else:
                failed_requests += 1
                print(f"请求 {i + 1} 失败: {result.get('error', 'Unknown error')}")
            
            # 请求间隔
            if i < count - 1:
                time.sleep(interval)
        
        # 计算统计信息
        stats = {
            'total_requests': count,
            'successful_requests': successful_requests,
            'failed_requests': failed_requests,
            'success_rate': successful_requests / count * 100 if count > 0 else 0
        }
        
        if latencies:
            stats.update({
                'latency_stats': {
                    'min': min(latencies),
                    'max': max(latencies),
                    'mean': statistics.mean(latencies),
                    'median': statistics.median(latencies),
                    'std': statistics.stdev(latencies) if len(latencies) > 1 else 0,
                    'p95': sorted(latencies)[int(len(latencies) * 0.95)] if latencies else 0,
                    'p99': sorted(latencies)[int(len(latencies) * 0.99)] if latencies else 0
                }
            })
        
        if time_diffs:
            stats['time_sync_stats'] = {
                'mean_diff_ms': statistics.mean(time_diffs),
                'max_diff_ms': max(time_diffs),
                'min_diff_ms': min(time_diffs)
            }
        
        return stats
    
    def print_results(self, stats: Dict):
        """
        打印测试结果
        
        Args:
            stats: 测试统计信息
        """
        print("\n" + "="*60)
        print("Binance 永续合约 API 延迟测试结果")
        print("="*60)
        
        print(f"总请求数: {stats['total_requests']}")
        print(f"成功请求: {stats['successful_requests']}")
        print(f"失败请求: {stats['failed_requests']}")
        print(f"成功率: {stats['success_rate']:.2f}%")
        
        if 'latency_stats' in stats:
            latency = stats['latency_stats']
            print(f"\n延迟统计 (毫秒):")
            print(f"  最小值: {latency['min']:.2f}")
            print(f"  最大值: {latency['max']:.2f}")
            print(f"  平均值: {latency['mean']:.2f}")
            print(f"  中位数: {latency['median']:.2f}")
            print(f"  标准差: {latency['std']:.2f}")
            print(f"  95分位: {latency['p95']:.2f}")
            print(f"  99分位: {latency['p99']:.2f}")
        
        if 'time_sync_stats' in stats:
            time_sync = stats['time_sync_stats']
            print(f"\n时间同步统计 (毫秒):")
            print(f"  平均时间差: {time_sync['mean_diff_ms']:.2f}")
            print(f"  最大时间差: {time_sync['max_diff_ms']:.2f}")
            print(f"  最小时间差: {time_sync['min_diff_ms']:.2f}")
        
        print("="*60)
    
    def save_results(self, stats: Dict, filename: str = None):
        """
        保存测试结果到文件
        
        Args:
            stats: 测试统计信息
            filename: 文件名（可选）
        """
        if filename is None:
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            filename = f"binance_latency_test_{timestamp}.json"
        
        # 添加测试时间戳
        stats['test_timestamp'] = datetime.now().isoformat()
        stats['api_endpoint'] = f"{self.base_url}/fapi/v1/time"
        
        with open(filename, 'w', encoding='utf-8') as f:
            json.dump(stats, f, indent=2, ensure_ascii=False)
        
        print(f"\n测试结果已保存到: {filename}")

def main():
    parser = argparse.ArgumentParser(description='Binance 永续合约 API 延迟测试')
    parser.add_argument('--count', type=int, default=100, help='测试请求次数 (默认: 100)')
    parser.add_argument('--interval', type=float, default=0.1, help='请求间隔秒数 (默认: 0.1)')
    parser.add_argument('--save', action='store_true', help='保存结果到文件')
    parser.add_argument('--filename', type=str, help='保存文件名')
    parser.add_argument('--url', type=str, default='https://fapi.binance.com', 
                       help='API 基础URL (默认: https://fapi.binance.com)')
    
    args = parser.parse_args()
    
    print("Binance 永续合约 API 延迟测试工具")
    print(f"API URL: {args.url}/fapi/v1/time")
    print(f"测试参数: {args.count} 次请求, 间隔 {args.interval} 秒")
    print("-" * 60)
    
    # 创建测试器
    tester = BinanceLatencyTester(args.url)
    
    # 先测试单次请求
    print("测试单次请求...")
    single_result = tester.test_single_request()
    if single_result['success']:
        print(f"✅ 单次请求成功: {single_result['latency_ms']:.2f}ms")
        print(f"   服务器时间: {single_result['server_time']}")
        print(f"   本地时间: {single_result['local_time']}")
        print(f"   时间差: {single_result['time_diff_ms']}ms")
    else:
        print(f"❌ 单次请求失败: {single_result.get('error', 'Unknown error')}")
        return
    
    print("\n" + "-" * 60)
    
    # 测试多次请求
    stats = tester.test_multiple_requests(args.count, args.interval)
    
    # 打印结果
    tester.print_results(stats)
    
    # 保存结果
    if args.save:
        tester.save_results(stats, args.filename)

if __name__ == "__main__":
    main() 