//! 简化的性能测试 - 用于初步验证
//! 避免复杂的删除操作，先测试插入和搜索性能

use rtree::{RTree, Rectangle};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::time::Instant;

fn main() {
    println!("🚀 R-tree 简化性能测试");
    println!("{}", "=".repeat(50));
    
    // 先测试小规模的操作
    test_small_scale();
    
    // 然后测试中等规模的操作
    test_medium_scale();
}

fn test_small_scale() {
    println!("\n📊 小规模测试 (1,000 条目)");
    println!("{}", "-".repeat(40));
    run_test_suite(1_000);
}

fn test_medium_scale() {
    println!("\n📊 中等规模测试 (10,000 条目)");
    println!("{}", "-".repeat(40));
    run_test_suite(10_000);
}

fn run_test_suite(size: usize) {
    // 1. 插入测试
    let (rtree, insert_time, test_data) = benchmark_insert(size);
    print_result("insert", size, insert_time);
    
    // 2. 单点查询测试  
    let search_time = benchmark_search_exact(&rtree, &test_data);
    print_result("search-exact", size, search_time);
    
    // 3. 区域查询测试
    let area_search_time = benchmark_search_area(&rtree, 1.0, 100);
    print_result("search-1%", 100, area_search_time);
    
    let area_search_time_5 = benchmark_search_area(&rtree, 5.0, 100);
    print_result("search-5%", 100, area_search_time_5);
    
    // 4. 简单删除测试（只删除少量条目避免触发复杂的下溢处理）
    let delete_time = benchmark_simple_delete(&test_data, size / 10); // 只删除10%
    print_result("delete-10%", size / 10, delete_time);
}

fn benchmark_insert(count: usize) -> (RTree, std::time::Duration, Vec<(Rectangle, i32)>) {
    let mut rng = StdRng::seed_from_u64(42);
    let mut test_data = Vec::with_capacity(count);
    
    // 生成测试数据
    for i in 0..count {
        let x = rng.gen_range(0.0..1000.0);
        let y = rng.gen_range(0.0..1000.0);
        let rect = Rectangle::new(x, y, x + 1.0, y + 1.0);
        test_data.push((rect, i as i32));
    }
    
    // 测试插入性能
    let start = Instant::now();
    let mut rtree = RTree::new(16);
    for (rect, data) in &test_data {
        rtree.insert(rect.clone(), *data);
    }
    let duration = start.elapsed();
    
    (rtree, duration, test_data)
}

fn benchmark_search_exact(rtree: &RTree, test_data: &[(Rectangle, i32)]) -> std::time::Duration {
    let start = Instant::now();
    let mut total_results = 0;
    
    for (rect, _) in test_data {
        let results = rtree.search(rect);
        total_results += results.len();
    }
    
    // 确保编译器不会优化掉我们的计算
    if total_results != test_data.len() {
        println!("Warning: unexpected result count: {}", total_results);
    }
    
    start.elapsed()
}

fn benchmark_search_area(rtree: &RTree, coverage_percent: f64, query_count: usize) -> std::time::Duration {
    let mut rng = StdRng::seed_from_u64(123);
    let query_size = (1000.0 * (coverage_percent / 100.0).sqrt()) as f64;
    
    let start = Instant::now();
    let mut total_results = 0;
    
    for _ in 0..query_count {
        let x = rng.gen_range(0.0..(1000.0 - query_size));
        let y = rng.gen_range(0.0..(1000.0 - query_size));
        let query = Rectangle::new(x, y, x + query_size, y + query_size);
        let results = rtree.search(&query);
        total_results += results.len();
    }
    
    // 防止编译器优化
    if total_results == 0 {
        println!("Warning: no results found in area search");
    }
    
    start.elapsed()
}

fn benchmark_simple_delete(test_data: &[(Rectangle, i32)], delete_count: usize) -> std::time::Duration {
    // 构建树
    let mut rtree = RTree::new(16);
    for (rect, data) in test_data {
        rtree.insert(rect.clone(), *data);
    }
    
    // 只删除前面的一些条目
    let start = Instant::now();
    let mut successful_deletes = 0;
    
    for i in 0..delete_count.min(test_data.len()) {
        let (rect, data) = &test_data[i];
        if rtree.delete(rect, *data) {
            successful_deletes += 1;
        }
    }
    
    let duration = start.elapsed();
    
    if successful_deletes != delete_count.min(test_data.len()) {
        println!("Warning: only {} out of {} deletes succeeded", 
            successful_deletes, delete_count.min(test_data.len()));
    }
    
    duration
}

fn print_result(operation: &str, ops: usize, duration: std::time::Duration) {
    let millis = duration.as_millis();
    let nanos = duration.as_nanos();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();
    let ns_per_op = if ops > 0 { nanos / ops as u128 } else { 0 };
    
    println!("{:<15} {:>8} ops in {}ms, {:>8.0}/sec, {} ns/op", 
        format!("{}:", operation), 
        format_number(ops),
        millis,
        ops_per_sec,
        ns_per_op
    );
}

fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
