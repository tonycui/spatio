//! 快速性能测试 - 用于开发过程中的快速验证
//! 
//! 输出格式类似 tidwall/rtree.rs，便于对比

use rtree::{RTree, Rectangle};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::time::Instant;

const QUICK_TEST_SIZE: usize = 10_000;
const FULL_TEST_SIZE: usize = 100_000;

fn main() {

    println!("🚀 R-tree 快速性能测试");
    println!("{}", "=".repeat(50));
    
    run_quick_benchmarks();
    
    println!("\n🔥 完整性能测试");
    println!("{}", "=".repeat(50));
    
    run_full_benchmarks();
}

fn run_quick_benchmarks() {
    let config = BenchConfig {
        size: QUICK_TEST_SIZE,
        max_entries: 100,
        seed: 42,
    };
    
    run_benchmark_suite("Quick Test", &config);
}

fn run_full_benchmarks() {
    let config = BenchConfig {
        size: FULL_TEST_SIZE,
        max_entries: 100,
        seed: 42,
    };
    
    run_benchmark_suite("Full Test", &config);
}
#[derive(Debug)]
struct BenchConfig {
    size: usize,
    max_entries: usize,
    seed: u64,
}

fn run_benchmark_suite(suite_name: &str, config: &BenchConfig) {
    println!("\n📊 {} ({} 条目)", suite_name, config.size);
    println!("{}", "-".repeat(40));
    println!("BenchConfig: {:#?}", config);
    
    // 1. 插入测试
    let (rtree, insert_time, test_data) = benchmark_insert(config);
    print_result("insert", config.size, insert_time);
    
    // 2. 单点查询测试
    let search_item_time = benchmark_search_item(&rtree, &test_data);
    print_result("search-item", config.size, search_item_time);
    
    // 3. 区域查询测试
    let search_1_time = benchmark_search_area(&rtree, 1.0, 1000, config.seed);
    print_result("search-1%", 1000, search_1_time);
    
    let search_5_time = benchmark_search_area(&rtree, 5.0, 1000, config.seed);
    print_result("search-5%", 1000, search_5_time);
    
    let search_10_time = benchmark_search_area(&rtree, 10.0, 1000, config.seed);
    print_result("search-10%", 1000, search_10_time);
    
    // 4. 删除测试
    let remove_half_time = benchmark_remove_half(&test_data, config);
    print_result("remove-half", config.size / 2, remove_half_time);
    
    // 5. 重新插入测试
    let reinsert_half_time = benchmark_reinsert_half(&test_data, config);
    print_result("reinsert-half", config.size / 2, reinsert_half_time);
    
    // 6. 删除全部测试
    let remove_all_time = benchmark_remove_all(&test_data, config);
    print_result("remove-all", config.size, remove_all_time);
}

fn benchmark_insert(config: &BenchConfig) -> (RTree, std::time::Duration, Vec<(Rectangle, i32)>) {
    let test_data = generate_test_data(config.size, config.seed);
    
    let start = Instant::now();
    let mut rtree = RTree::new(config.max_entries);
    for (rect, data) in &test_data {
        rtree.insert(rect.clone(), *data);
    }
    let duration = start.elapsed();
    
    (rtree, duration, test_data)
}

fn benchmark_search_item(rtree: &RTree, test_data: &[(Rectangle, i32)]) -> std::time::Duration {
    let start = Instant::now();
    for (rect, _) in test_data {
        let _results = rtree.search(rect);
    }
    start.elapsed()
}

fn benchmark_search_area(rtree: &RTree, coverage_percent: f64, query_count: usize, seed: u64) -> std::time::Duration {
    let queries = generate_query_rects(query_count, coverage_percent, seed);
    
    let start = Instant::now();
    for query in &queries {
        let _results = rtree.search(query);
    }
    start.elapsed()
}

fn benchmark_remove_half(test_data: &[(Rectangle, i32)], config: &BenchConfig) -> std::time::Duration {
    let mut rtree = RTree::new(config.max_entries);
    for (rect, data) in test_data {
        rtree.insert(rect.clone(), *data);
    }
    
    let half_size = config.size / 2;
    let start = Instant::now();
    for i in 0..half_size {
        let (rect, data) = &test_data[i];
        rtree.delete(*data);
    }
    start.elapsed()
}

fn benchmark_reinsert_half(test_data: &[(Rectangle, i32)], config: &BenchConfig) -> std::time::Duration {
    let mut rtree = RTree::new(config.max_entries);
    let half_size = config.size / 2;
    
    // 先插入后一半
    for i in half_size..config.size {
        let (rect, data) = &test_data[i];
        rtree.insert(rect.clone(), *data);
    }
    
    // 测试插入前一半的时间
    let start = Instant::now();
    for i in 0..half_size {
        let (rect, data) = &test_data[i];
        rtree.insert(rect.clone(), *data);
    }
    start.elapsed()
}

fn benchmark_remove_all(test_data: &[(Rectangle, i32)], config: &BenchConfig) -> std::time::Duration {
    let mut rtree = RTree::new(config.max_entries);
    for (rect, data) in test_data {
        rtree.insert(rect.clone(), *data);
    }
    
    let start = Instant::now();
    for (rect, data) in test_data {
        rtree.delete(*data);
    }
    start.elapsed()
}

fn generate_test_data(count: usize, seed: u64) -> Vec<(Rectangle, i32)> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut data = Vec::with_capacity(count);
    
    for i in 0..count {
        let x = rng.gen_range(0.0..1000.0);
        let y = rng.gen_range(0.0..1000.0);
        let rect = Rectangle::new(x, y, x + 1.0, y + 1.0);
        data.push((rect, i as i32));
    }
    
    data
}

fn generate_query_rects(count: usize, coverage_percent: f64, seed: u64) -> Vec<Rectangle> {
    let mut rng = StdRng::seed_from_u64(seed + 1000);
    let mut queries = Vec::with_capacity(count);
    
    let query_size = (1000.0 * (coverage_percent / 100.0).sqrt()) as f64;
    
    for _ in 0..count {
        let x = rng.gen_range(0.0..(1000.0 - query_size));
        let y = rng.gen_range(0.0..(1000.0 - query_size));
        queries.push(Rectangle::new(x, y, x + query_size, y + query_size));
    }
    
    queries
}

fn print_result(operation: &str, ops: usize, duration: std::time::Duration) {
    let millis = duration.as_millis();
    let nanos = duration.as_nanos();
    let ops_per_sec = ops as f64 / duration.as_secs_f64();
    let ns_per_op = nanos / ops as u128;
    
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
