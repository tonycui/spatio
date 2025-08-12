#!/usr/bin/env python3
"""
spatio 高并发性能测试脚本（多线程版本）
"""

import redis
import json
import random
import time
import statistics
from typing import List, Dict, Any
from concurrent.futures import ThreadPoolExecutor, as_completed
import threading

class SpatioConcurrentBenchmark:
    def __init__(self):
        # 新加坡边界 (大约)
        self.singapore_bounds = {
            'min_lng': 103.6,
            'max_lng': 104.0,
            'min_lat': 1.2,
            'max_lat': 1.5
        }
        
        self.collection_name = "benchmark_collection"
        
        # 线程本地存储，为每个线程创建独立的连接
        self._local_connections = threading.local()
    
    def get_connection(self):
        """获取线程本地的 Redis 连接"""
        if not hasattr(self._local_connections, 'client'):
            self._local_connections.client = redis.Redis(
                host='localhost', 
                port=6379, 
                decode_responses=True,
                socket_connect_timeout=5,
                socket_timeout=5,
                retry_on_timeout=True,
                health_check_interval=30
            )
        return self._local_connections.client

    def generate_random_polygon_in_singapore(self) -> Dict[str, Any]:
        """在新加坡范围内生成随机多边形"""
        bounds = self.singapore_bounds
        
        # 生成一个小的随机矩形
        width = random.uniform(0.01, 0.05)  # 经度宽度
        height = random.uniform(0.01, 0.05)  # 纬度高度
        
        min_lng = random.uniform(bounds['min_lng'], bounds['max_lng'] - width)
        min_lat = random.uniform(bounds['min_lat'], bounds['max_lat'] - height)
        max_lng = min_lng + width
        max_lat = min_lat + height
        
        return {
            "type": "Polygon",
            "coordinates": [[
                [min_lng, min_lat],
                [max_lng, min_lat],
                [max_lng, max_lat],
                [min_lng, max_lat],
                [min_lng, min_lat]
            ]]
        }
    
    def generate_test_data(self, count: int) -> List[Dict[str, Any]]:
        """生成测试数据"""
        print(f"生成 {count} 条测试数据...")
        data = []
        for i in range(count):
            if i % 10000 == 0:
                print(f"进度: {i}/{count}")
            
            geometry = self.generate_random_polygon_in_singapore()
            item = {
                "id": f"item_{i}",
                "geometry": geometry
            }
            data.append(item)
        return data
    
    def insert_single_item(self, item: Dict[str, Any]) -> bool:
        """插入单个数据项"""
        try:
            client = self.get_connection()
            client.execute_command("SET", self.collection_name, item['id'], json.dumps(item['geometry']))
            return True
        except Exception as e:
            print(f"插入失败: {e}")
            return False
    
    def insert_data_spatio_concurrent(self, data: List[Dict[str, Any]], max_workers: int = 500):
        """并发插入数据到 spatio"""
        print(f"开始并发插入数据，并发数: {max_workers}")
        start_time = time.time()
        
        success_count = 0
        total_count = len(data)
        
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            # 提交所有任务
            futures = [executor.submit(self.insert_single_item, item) for item in data]
            
            # 收集结果
            for i, future in enumerate(as_completed(futures)):
                if i % 5000 == 0:
                    print(f"插入进度: {i}/{total_count} ({i/total_count*100:.1f}%)")
                
                try:
                    if future.result():
                        success_count += 1
                except Exception as e:
                    print(f"任务执行失败: {e}")
        
        end_time = time.time()
        duration = end_time - start_time
        
        print(f"并发插入完成:")
        print(f"  总数: {total_count}")
        print(f"  成功: {success_count}")
        print(f"  失败: {total_count - success_count}")
        print(f"  耗时: {duration:.2f}s")
        print(f"  吞吐量: {success_count/duration:.2f} ops/s")
    
    def query_single_intersects(self, geometry: Dict[str, Any]) -> Dict[str, Any]:
        """执行单个 intersects 查询"""
        start_time = time.time()
        try:
            client = self.get_connection()
            result = client.execute_command("INTERSECTS", self.collection_name, json.dumps(geometry))
            end_time = time.time()
            return {
                'success': True,
                'duration': end_time - start_time,
                'result': result
            }
        except Exception as e:
            end_time = time.time()
            return {
                'success': False,
                'duration': end_time - start_time,
                'error': str(e)
            }
    
    def query_intersects_spatio_concurrent(self, geometries: List[Dict[str, Any]], max_workers: int = 500) -> List[float]:
        """并发查询 intersects"""
        print(f"开始并发查询测试，并发数: {max_workers}")
        
        query_times = []
        success_count = 0
        total_count = len(geometries)
        
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            # 提交所有查询任务
            futures = [executor.submit(self.query_single_intersects, geom) for geom in geometries]
            
            # 收集结果
            for i, future in enumerate(as_completed(futures)):
                if i % 100 == 0:
                    print(f"查询进度: {i}/{total_count} ({i/total_count*100:.1f}%)")
                
                try:
                    result = future.result()
                    if result['success']:
                        query_times.append(result['duration'])
                        success_count += 1
                    else:
                        print(f"查询失败: {result['error']}")
                except Exception as e:
                    print(f"查询任务失败: {e}")
        
        print(f"查询测试完成:")
        print(f"  总数: {total_count}")
        print(f"  成功: {success_count}")
        print(f"  失败: {total_count - success_count}")
        
        return query_times
    
    def run_benchmark(self, data_count: int = 50000, query_count: int = 5000, max_workers: int = 100):
        """运行高并发性能测试"""
        print(f"开始 spatio 高并发性能测试:")
        print(f"  数据量: {data_count}")
        print(f"  查询数: {query_count}")
        print(f"  并发数: {max_workers}")
        print("="*60)
        
        # 生成测试数据
        test_data = self.generate_test_data(data_count)
        
        # 生成查询几何体
        print(f"生成 {query_count} 个查询几何体...")
        query_geometries = [self.generate_random_polygon_in_singapore() for _ in range(query_count)]
        
        # 并发插入数据
        self.insert_data_spatio_concurrent(test_data, max_workers)
        
        print("等待 2 秒让系统稳定...")
        time.sleep(2)
        
        # 并发查询测试
        start_time = time.time()
        spatio_times = self.query_intersects_spatio_concurrent(query_geometries, max_workers)
        end_time = time.time()
        
        total_query_time = end_time - start_time
        
        # 输出结果
        self.print_results(spatio_times, total_query_time)
    
    def print_results(self, spatio_times: List[float], total_time: float):
        """打印性能测试结果"""
        print("\n" + "="*60)
        print("spatio 高并发性能测试结果")
        print("="*60)
        
        if not spatio_times:
            print("没有成功的查询结果")
            return
        
        # 统计数据
        spatio_avg = statistics.mean(spatio_times) * 1000  # 转换为毫秒
        spatio_min = min(spatio_times) * 1000
        spatio_max = max(spatio_times) * 1000
        spatio_median = statistics.median(spatio_times) * 1000
        
        # 计算百分位数
        sorted_times = sorted(spatio_times)
        p95 = sorted_times[int(len(sorted_times) * 0.95)] * 1000
        p99 = sorted_times[int(len(sorted_times) * 0.99)] * 1000
        
        # 计算 QPS
        total_queries = len(spatio_times)
        qps = total_queries / total_time
        
        print(f"总查询数: {total_queries}")
        print(f"总耗时: {total_time:.2f}s")
        print(f"QPS (每秒查询数): {qps:.2f}")
        print(f"平均响应时间: {spatio_avg:.2f}ms")
        print(f"最小响应时间: {spatio_min:.2f}ms")
        print(f"最大响应时间: {spatio_max:.2f}ms")
        print(f"中位数响应时间: {spatio_median:.2f}ms")
        print(f"95% 响应时间: {p95:.2f}ms")
        print(f"99% 响应时间: {p99:.2f}ms")

if __name__ == "__main__":
    benchmark = SpatioConcurrentBenchmark()
    
    # 可以调整参数进行不同规模的测试
    # 建议先用较小的并发数测试，确认系统稳定后再提升
    benchmark.run_benchmark(
        data_count=10000,    # 1万条数据
        query_count=2000,    # 2千次查询
        max_workers=100      # 100 并发（先用较小值测试）
    )
