#!/usr/bin/env python3
"""
spatio 性能测试脚本（仅测试 spatio）
"""

import redis
import json
import random
import time
import statistics
from typing import List, Dict, Any

class SpatioBenchmark:
    def __init__(self):
        # 新加坡边界 (大约)
        self.singapore_bounds = {
            'min_lng': 103.6,
            'max_lng': 104.0,
            'min_lat': 1.2,
            'max_lat': 1.5
        }
        
        # Redis 连接
        self.spatio_client = redis.Redis(host='localhost', port=6379, decode_responses=True)
        self.collection_name = "benchmark_collection"
    
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
    
    def insert_data_spatio(self, data: List[Dict[str, Any]]):
        """向 spatio 插入数据"""
        print("向 spatio 插入数据...")
        start_time = time.time()
        
        for i, item in enumerate(data):
            if i % 10000 == 0:
                print(f"spatio 插入进度: {i}/{len(data)}")
            
            # spatio SET 命令格式: SET collection_name id geojson
            self.spatio_client.execute_command("SET", self.collection_name, item['id'], json.dumps(item['geometry']))
        
        end_time = time.time()
        print(f"spatio 插入完成，耗时: {end_time - start_time:.2f}s")
    
    def query_intersects_spatio(self, geometry: Dict[str, Any]) -> float:
        """spatio intersects 查询"""
        start_time = time.time()
        
        # spatio INTERSECTS 命令格式: INTERSECTS collection_name geometry
        self.spatio_client.execute_command("INTERSECTS", self.collection_name, json.dumps(geometry))
        
        end_time = time.time()
        return end_time - start_time
    
    def run_benchmark(self, data_count: int = 100000, query_count: int = 100):
        """运行性能测试"""
        print(f"开始 spatio 性能测试: {data_count} 条数据, {query_count} 次查询")
        
        # 生成测试数据
        test_data = self.generate_test_data(data_count)
        
        # 生成查询几何体
        print(f"生成 {query_count} 个查询几何体...")
        query_geometries = [self.generate_random_polygon_in_singapore() for _ in range(query_count)]
        
        # 插入数据
        self.insert_data_spatio(test_data)
        
        # print("等待 3 秒...")
        # time.sleep(3)
        
        # spatio 查询测试
        print("开始 spatio 查询测试...")
        spatio_times = []
        for i, geometry in enumerate(query_geometries):
            if i % 10 == 0:
                print(f"spatio 查询进度: {i}/{query_count}")
            
            query_time = self.query_intersects_spatio(geometry)
            spatio_times.append(query_time)
        
        # 输出结果
        self.print_results(spatio_times)
    
    def print_results(self, spatio_times: List[float]):
        """打印性能测试结果"""
        print("\n" + "="*60)
        print("spatio 性能测试结果")
        print("="*60)
        
        # 统计数据
        spatio_avg = statistics.mean(spatio_times) * 1000  # 转换为毫秒
        spatio_min = min(spatio_times) * 1000
        spatio_max = max(spatio_times) * 1000
        spatio_median = statistics.median(spatio_times) * 1000
        
        print(f"平均响应时间: {spatio_avg:.2f}ms")
        print(f"最小响应时间: {spatio_min:.2f}ms")
        print(f"最大响应时间: {spatio_max:.2f}ms")
        print(f"中位数响应时间: {spatio_median:.2f}ms")
        print(f"总查询数: {len(spatio_times)}")

if __name__ == "__main__":
    benchmark = SpatioBenchmark()
    benchmark.run_benchmark(data_count=100000, query_count=1000)
