#!/usr/bin/env python3
"""
geo42 vs tile38 intersects 性能对比脚本（多线程版本）
"""

import redis
import json
import random
import time
import statistics
from typing import List, Dict, Any
from concurrent.futures import ThreadPoolExecutor, as_completed
import threading

class GeoConcurrentBenchmark:
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
    
    def get_geo42_connection(self):
        """获取线程本地的 geo42 连接"""
        if not hasattr(self._local_connections, 'geo42_client'):
            self._local_connections.geo42_client = redis.Redis(
                host='localhost', 
                port=6379, 
                decode_responses=True,
                socket_connect_timeout=5,
                socket_timeout=5,
                retry_on_timeout=True,
                health_check_interval=30
            )
        return self._local_connections.geo42_client
    
    def get_tile38_connection(self):
        """获取线程本地的 tile38 连接"""
        if not hasattr(self._local_connections, 'tile38_client'):
            self._local_connections.tile38_client = redis.Redis(
                host='localhost', 
                port=9851, 
                decode_responses=True,
                socket_connect_timeout=5,
                socket_timeout=5,
                retry_on_timeout=True,
                health_check_interval=30
            )
        return self._local_connections.tile38_client

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
    
    def insert_single_item_geo42(self, item: Dict[str, Any]) -> bool:
        """向 geo42 插入单个数据项"""
        try:
            client = self.get_geo42_connection()
            client.execute_command("SET", self.collection_name, item['id'], json.dumps(item['geometry']))
            return True
        except Exception as e:
            print(f"geo42 插入失败: {e}")
            return False
    
    def insert_single_item_tile38(self, item: Dict[str, Any]) -> bool:
        """向 tile38 插入单个数据项"""
        try:
            client = self.get_tile38_connection()
            client.execute_command("SET", self.collection_name, item['id'], "OBJECT", json.dumps(item['geometry']))
            return True
        except Exception as e:
            print(f"tile38 插入失败: {e}")
            return False
    
    def insert_data_geo42_concurrent(self, data: List[Dict[str, Any]], max_workers: int = 100):
        """并发插入数据到 geo42"""
        print(f"开始并发插入数据到 geo42，并发数: {max_workers}")
        start_time = time.time()
        
        success_count = 0
        total_count = len(data)
        
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = [executor.submit(self.insert_single_item_geo42, item) for item in data]
            
            for i, future in enumerate(as_completed(futures)):
                if i % 5000 == 0:
                    print(f"geo42 插入进度: {i}/{total_count} ({i/total_count*100:.1f}%)")
                
                try:
                    if future.result():
                        success_count += 1
                except Exception as e:
                    print(f"geo42 任务执行失败: {e}")
        
        end_time = time.time()
        duration = end_time - start_time
        
        print(f"geo42 并发插入完成:")
        print(f"  总数: {total_count}")
        print(f"  成功: {success_count}")
        print(f"  失败: {total_count - success_count}")
        print(f"  耗时: {duration:.2f}s")
        print(f"  吞吐量: {success_count/duration:.2f} ops/s")
    
    def insert_data_tile38_concurrent(self, data: List[Dict[str, Any]], max_workers: int = 100):
        """并发插入数据到 tile38"""
        print(f"开始并发插入数据到 tile38，并发数: {max_workers}")
        start_time = time.time()
        
        success_count = 0
        total_count = len(data)
        
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = [executor.submit(self.insert_single_item_tile38, item) for item in data]
            
            for i, future in enumerate(as_completed(futures)):
                if i % 5000 == 0:
                    print(f"tile38 插入进度: {i}/{total_count} ({i/total_count*100:.1f}%)")
                
                try:
                    if future.result():
                        success_count += 1
                except Exception as e:
                    print(f"tile38 任务执行失败: {e}")
        
        end_time = time.time()
        duration = end_time - start_time
        
        print(f"tile38 并发插入完成:")
        print(f"  总数: {total_count}")
        print(f"  成功: {success_count}")
        print(f"  失败: {total_count - success_count}")
        print(f"  耗时: {duration:.2f}s")
        print(f"  吞吐量: {success_count/duration:.2f} ops/s")
    
    def query_single_intersects_geo42(self, geometry: Dict[str, Any]) -> Dict[str, Any]:
        """执行单个 geo42 intersects 查询"""
        start_time = time.time()
        try:
            client = self.get_geo42_connection()
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
    
    def query_single_intersects_tile38(self, geometry: Dict[str, Any]) -> Dict[str, Any]:
        """执行单个 tile38 intersects 查询"""
        start_time = time.time()
        try:
            client = self.get_tile38_connection()
            result = client.execute_command("INTERSECTS", self.collection_name, "LIMIT", "100000", "OBJECT", json.dumps(geometry))
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
    
    def query_intersects_geo42_concurrent(self, geometries: List[Dict[str, Any]], max_workers: int = 100) -> List[float]:
        """并发查询 geo42 intersects"""
        print(f"开始并发查询 geo42，并发数: {max_workers}")
        
        query_times = []
        success_count = 0
        total_count = len(geometries)
        
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = [executor.submit(self.query_single_intersects_geo42, geom) for geom in geometries]
            
            for i, future in enumerate(as_completed(futures)):
                if i % 100 == 0:
                    print(f"geo42 查询进度: {i}/{total_count} ({i/total_count*100:.1f}%)")
                
                try:
                    result = future.result()
                    if result['success']:
                        query_times.append(result['duration'])
                        success_count += 1
                    else:
                        print(f"geo42 查询失败: {result['error']}")
                except Exception as e:
                    print(f"geo42 查询任务失败: {e}")
        
        print(f"geo42 查询测试完成: 成功 {success_count}/{total_count}")
        return query_times
    
    def query_intersects_tile38_concurrent(self, geometries: List[Dict[str, Any]], max_workers: int = 100) -> List[float]:
        """并发查询 tile38 intersects"""
        print(f"开始并发查询 tile38，并发数: {max_workers}")
        
        query_times = []
        success_count = 0
        total_count = len(geometries)
        
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = [executor.submit(self.query_single_intersects_tile38, geom) for geom in geometries]
            
            for i, future in enumerate(as_completed(futures)):
                if i % 100 == 0:
                    print(f"tile38 查询进度: {i}/{total_count} ({i/total_count*100:.1f}%)")
                
                try:
                    result = future.result()
                    if result['success']:
                        query_times.append(result['duration'])
                        success_count += 1
                    else:
                        print(f"tile38 查询失败: {result['error']}")
                except Exception as e:
                    print(f"tile38 查询任务失败: {e}")
        
        print(f"tile38 查询测试完成: 成功 {success_count}/{total_count}")
        return query_times
    
    def run_benchmark(self, data_count: int = 50000, query_count: int = 5000, max_workers: int = 100):
        """运行高并发性能对比测试"""
        print(f"开始 geo42 vs tile38 高并发性能对比测试:")
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
        self.insert_data_geo42_concurrent(test_data, max_workers)
        self.insert_data_tile38_concurrent(test_data, max_workers)
        
        print("等待 3 秒让系统稳定...")
        time.sleep(3)
        
        # 并发查询测试
        print("\n开始查询性能测试...")
        
        # geo42 查询测试
        start_time = time.time()
        geo42_times = self.query_intersects_geo42_concurrent(query_geometries, max_workers)
        geo42_total_time = time.time() - start_time
        
        # tile38 查询测试
        start_time = time.time()
        tile38_times = self.query_intersects_tile38_concurrent(query_geometries, max_workers)
        tile38_total_time = time.time() - start_time
        
        # 输出结果
        self.print_results(geo42_times, tile38_times, geo42_total_time, tile38_total_time)
    
    def print_results(self, geo42_times: List[float], tile38_times: List[float], geo42_total_time: float, tile38_total_time: float):
        """打印性能测试结果"""
        print("\n" + "="*60)
        print("geo42 vs tile38 高并发性能对比结果")
        print("="*60)
        
        if not geo42_times or not tile38_times:
            print("查询结果不完整，无法进行对比")
            return
        
        # 统计数据
        geo42_avg = statistics.mean(geo42_times) * 1000  # 转换为毫秒
        geo42_min = min(geo42_times) * 1000
        geo42_max = max(geo42_times) * 1000
        geo42_median = statistics.median(geo42_times) * 1000
        geo42_p95 = sorted(geo42_times)[int(len(geo42_times) * 0.95)] * 1000
        geo42_qps = len(geo42_times) / geo42_total_time
        
        tile38_avg = statistics.mean(tile38_times) * 1000
        tile38_min = min(tile38_times) * 1000
        tile38_max = max(tile38_times) * 1000
        tile38_median = statistics.median(tile38_times) * 1000
        tile38_p95 = sorted(tile38_times)[int(len(tile38_times) * 0.95)] * 1000
        tile38_qps = len(tile38_times) / tile38_total_time
        
        print(f"{'指标':<15} {'geo42':<15} {'tile38':<15} {'对比':<15}")
        print("-" * 60)
        print(f"{'查询成功数':<15} {len(geo42_times):<15} {len(tile38_times):<15}")
        print(f"{'QPS':<15} {geo42_qps:<15.2f} {tile38_qps:<15.2f} {geo42_qps/tile38_qps:<15.2f}x")
        print(f"{'平均延迟(ms)':<15} {geo42_avg:<15.2f} {tile38_avg:<15.2f} {tile38_avg/geo42_avg:<15.2f}x")
        print(f"{'中位数(ms)':<15} {geo42_median:<15.2f} {tile38_median:<15.2f} {tile38_median/geo42_median:<15.2f}x")
        print(f"{'P95延迟(ms)':<15} {geo42_p95:<15.2f} {tile38_p95:<15.2f} {tile38_p95/geo42_p95:<15.2f}x")
        print(f"{'最小延迟(ms)':<15} {geo42_min:<15.2f} {tile38_min:<15.2f}")
        print(f"{'最大延迟(ms)':<15} {geo42_max:<15.2f} {tile38_max:<15.2f}")
        
        print("\n" + "="*60)
        print("总结:")
        if geo42_qps > tile38_qps:
            print(f"🚀 geo42 QPS 比 tile38 高 {geo42_qps/tile38_qps:.2f}x")
        else:
            print(f"📊 tile38 QPS 比 geo42 高 {tile38_qps/geo42_qps:.2f}x")
            
        if geo42_avg < tile38_avg:
            print(f"⚡ geo42 平均延迟比 tile38 低 {tile38_avg/geo42_avg:.2f}x")
        else:
            print(f"⏱️ tile38 平均延迟比 geo42 低 {geo42_avg/tile38_avg:.2f}x")

if __name__ == "__main__":
    benchmark = GeoConcurrentBenchmark()
    
    # 可以调整参数进行不同规模的测试
    # 建议先用较小的并发数测试，确认系统稳定后再提升
    benchmark.run_benchmark(
        data_count=100000,    # 10万条数据
        query_count=10000,    # 1万次查询
        max_workers=100      # 100 并发（先用较小值测试）
    )
