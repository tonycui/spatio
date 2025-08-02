#!/usr/bin/env python3
"""
geo42 vs tile38 intersects 性能对比脚本
"""

import redis
import json
import random
import time
import statistics
from typing import List, Dict, Any

class GeoBenchmark:
    def __init__(self):
        # 新加坡边界 (大约)
        self.singapore_bounds = {
            'min_lng': 103.6,
            'max_lng': 104.0,
            'min_lat': 1.2,
            'max_lat': 1.5
        }
        
        # Redis 连接
        self.geo42_client = redis.Redis(host='localhost', port=6379, decode_responses=True)
        self.tile38_client = redis.Redis(host='localhost', port=9851, decode_responses=True)
        
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
    
    def insert_data_geo42(self, data: List[Dict[str, Any]]):
        """向 geo42 插入数据"""
        print("向 geo42 插入数据...")
        start_time = time.time()
        
        for i, item in enumerate(data):
            if i % 10000 == 0:
                print(f"geo42 插入进度: {i}/{len(data)}")
            
            # geo42 SET 命令格式: SET collection_name id geojson
            self.geo42_client.execute_command("SET", self.collection_name, item['id'], json.dumps(item['geometry']))
        
        end_time = time.time()
        print(f"geo42 插入完成，耗时: {end_time - start_time:.2f}s")
    
    def insert_data_tile38(self, data: List[Dict[str, Any]]):
        """向 tile38 插入数据"""
        print("向 tile38 插入数据...")
        start_time = time.time()
        
        for i, item in enumerate(data):
            if i % 10000 == 0:
                print(f"tile38 插入进度: {i}/{len(data)}")
            
            # tile38 SET 命令格式: SET collection_name id OBJECT geojson
            self.tile38_client.execute_command("SET", self.collection_name, item['id'], "OBJECT", json.dumps(item['geometry']))
        
        end_time = time.time()
        print(f"tile38 插入完成，耗时: {end_time - start_time:.2f}s")
    
    def query_intersects_geo42(self, geometry: Dict[str, Any]) -> float:
        """geo42 intersects 查询"""
        start_time = time.time()
        
        # geo42 INTERSECTS 命令格式: INTERSECTS collection_name geometry
        self.geo42_client.execute_command("INTERSECTS", self.collection_name, json.dumps(geometry))
        
        end_time = time.time()
        return end_time - start_time
    
    def query_intersects_tile38(self, geometry: Dict[str, Any]) -> float:
        """tile38 intersects 查询"""
        start_time = time.time()
        
        # tile38 INTERSECTS 命令格式: INTERSECTS collection_name LIMIT 100000 OBJECT geojson  
        self.tile38_client.execute_command("INTERSECTS", self.collection_name, "LIMIT", "100000", "OBJECT", json.dumps(geometry))
        
        end_time = time.time()
        return end_time - start_time
    
    def run_benchmark(self, data_count: int = 100000, query_count: int = 100):
        """运行性能测试"""
        print(f"开始性能测试: {data_count} 条数据, {query_count} 次查询")
        
        # 生成测试数据
        test_data = self.generate_test_data(data_count)
        
        # 生成查询几何体
        print(f"生成 {query_count} 个查询几何体...")
        query_geometries = [self.generate_random_polygon_in_singapore() for _ in range(query_count)]
        
        # 插入数据
        self.insert_data_geo42(test_data)
        self.insert_data_tile38(test_data)
        
        print("等待 3 秒...")
        time.sleep(3)
        
        # geo42 查询测试
        print("开始 geo42 查询测试...")
        geo42_times = []
        for i, geometry in enumerate(query_geometries):
            if i % 10 == 0:
                print(f"geo42 查询进度: {i}/{query_count}")
            
            query_time = self.query_intersects_geo42(geometry)
            geo42_times.append(query_time)
        
        # tile38 查询测试
        print("开始 tile38 查询测试...")
        tile38_times = []
        for i, geometry in enumerate(query_geometries):
            if i % 10 == 0:
                print(f"tile38 查询进度: {i}/{query_count}")
            
            query_time = self.query_intersects_tile38(geometry)
            tile38_times.append(query_time)
        
        # 输出结果
        self.print_results(geo42_times, tile38_times)
    
    def print_results(self, geo42_times: List[float], tile38_times: List[float]):
        """打印性能测试结果"""
        print("\n" + "="*60)
        print("性能测试结果")
        print("="*60)
        
        # 统计数据
        geo42_avg = statistics.mean(geo42_times) * 1000  # 转换为毫秒
        geo42_min = min(geo42_times) * 1000
        geo42_max = max(geo42_times) * 1000
        
        tile38_avg = statistics.mean(tile38_times) * 1000
        tile38_min = min(tile38_times) * 1000
        tile38_max = max(tile38_times) * 1000
        
        print(f"geo42  - 平均: {geo42_avg:.2f}ms, 最小: {geo42_min:.2f}ms, 最大: {geo42_max:.2f}ms")
        print(f"tile38 - 平均: {tile38_avg:.2f}ms, 最小: {tile38_min:.2f}ms, 最大: {tile38_max:.2f}ms")
        
        # 性能对比
        if geo42_avg < tile38_avg:
            speedup = tile38_avg / geo42_avg
            print(f"\ngeo42 比 tile38 快 {speedup:.2f}x")
        else:
            speedup = geo42_avg / tile38_avg
            print(f"\ntile38 比 geo42 快 {speedup:.2f}x")

if __name__ == "__main__":
    benchmark = GeoBenchmark()
    benchmark.run_benchmark(data_count=100000, query_count=10000)
