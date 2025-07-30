#!/usr/bin/env python3
"""
专门用于 profiling geo42 的测试脚本
"""

import redis
import json
import random
import time

def generate_random_polygon():
    """生成新加坡范围内的随机多边形"""
    min_lng = random.uniform(103.6, 103.9)
    min_lat = random.uniform(1.2, 1.4)
    width = random.uniform(0.01, 0.05)
    height = random.uniform(0.01, 0.05)
    
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

def main():
    client = redis.Redis(host='localhost', port=6379, decode_responses=True)
    collection_name = "profile_test"
    
    print("插入 10k 条数据...")
    # 插入较少数据但足够测试
    for i in range(10000):
        if i % 1000 == 0:
            print(f"进度: {i}")
        geometry = generate_random_polygon()
        client.execute_command("SET", collection_name, f"item_{i}", json.dumps(geometry))
    
    print("开始 intersects 查询测试...")
    # 执行大量查询来触发性能瓶颈
    for i in range(1000):  # 1000次查询应该足够看出瓶颈
        if i % 100 == 0:
            print(f"查询进度: {i}")
        query_geometry = generate_random_polygon()
        client.execute_command("INTERSECTS", collection_name, json.dumps(query_geometry))
    
    print("测试完成!")

if __name__ == "__main__":
    main()