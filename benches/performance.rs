//! R-tree 性能基准测试
//! 
//! 参考 tidwall/rtree.rs 的测试方法，但适配我们的实现特点
//! 当前测试规模：100,000 条目
//! 未来扩展：1,000,000 条目

use criterion::{criterion_group, criterion_main, Criterion};
use rtree::{RTree, Rectangle};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

const BENCHMARK_SIZE: usize = 100_000;  // 当前基准测试大小
#[allow(dead_code)]
const FUTURE_SIZE: usize = 1_000_000;   // 未来目标大小

/// 性能测试配置
struct BenchConfig {
    size: usize,
    max_entries: usize,
    seed: u64,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            size: BENCHMARK_SIZE,
            max_entries: 16,
            seed: 42,
        }
    }
}

/// 生成测试数据
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

/// 生成查询矩形
fn generate_query_rects(count: usize, coverage_percent: f64, seed: u64) -> Vec<Rectangle> {
    let mut rng = StdRng::seed_from_u64(seed + 1000);
    let mut queries = Vec::with_capacity(count);
    
    // 根据覆盖率计算查询矩形的大小
    let query_size = (1000.0 * (coverage_percent / 100.0).sqrt()) as f64;
    
    for _ in 0..count {
        let x = rng.gen_range(0.0..(1000.0 - query_size));
        let y = rng.gen_range(0.0..(1000.0 - query_size));
        queries.push(Rectangle::new(x, y, x + query_size, y + query_size));
    }
    
    queries
}

/// 插入性能测试
fn bench_insert(c: &mut Criterion) {
    let config = BenchConfig::default();
    let test_data = generate_test_data(config.size, config.seed);
    
    c.bench_function("insert", |b| {
        b.iter(|| {
            let mut rtree = RTree::new(config.max_entries);
            for (rect, data) in &test_data {
                rtree.insert(rect.clone(), *data);
            }
            rtree
        });
    });
}

/// 单点查询性能测试
fn bench_search_item(c: &mut Criterion) {
    let config = BenchConfig::default();
    let test_data = generate_test_data(config.size, config.seed);
    
    // 构建测试树
    let mut rtree = RTree::new(config.max_entries);
    for (rect, data) in &test_data {
        rtree.insert(rect.clone(), *data);
    }
    
    c.bench_function("search_item", |b| {
        b.iter(|| {
            let mut total_results = 0;
            for (rect, _) in &test_data {
                let results = rtree.search(rect);
                total_results += results.len();
            }
            total_results
        });
    });
}

/// 区域查询性能测试
fn bench_search_area(c: &mut Criterion) {
    let config = BenchConfig::default();
    let test_data = generate_test_data(config.size, config.seed);
    
    // 构建测试树
    let mut rtree = RTree::new(config.max_entries);
    for (rect, data) in &test_data {
        rtree.insert(rect.clone(), *data);
    }
    
    let test_cases = vec![
        ("search_1%", 1.0),
        ("search_5%", 5.0),
        ("search_10%", 10.0),
    ];
    
    for (name, coverage) in test_cases {
        let queries = generate_query_rects(10_000, coverage, config.seed);
        
        c.bench_function(name, |b| {
            b.iter(|| {
                let mut total_results = 0;
                for query in &queries {
                    let results = rtree.search(query);
                    total_results += results.len();
                }
                total_results
            });
        });
    }
}

/// 删除性能测试
fn bench_remove(c: &mut Criterion) {
    let config = BenchConfig::default();
    let test_data = generate_test_data(config.size, config.seed);
    
    c.bench_function("remove_half", |b| {
        b.iter_batched(
            || {
                // Setup: 构建完整的树
                let mut rtree = RTree::new(config.max_entries);
                for (rect, data) in &test_data {
                    rtree.insert(rect.clone(), *data);
                }
                rtree
            },
            |mut rtree| {
                // Benchmark: 删除一半数据
                let half_size = config.size / 2;
                for i in 0..half_size {
                    let (rect, data) = &test_data[i];
                    rtree.delete(rect, *data);
                }
                rtree
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// 重新插入性能测试
fn bench_reinsert(c: &mut Criterion) {
    let config = BenchConfig::default();
    let test_data = generate_test_data(config.size, config.seed);
    
    c.bench_function("reinsert_half", |b| {
        b.iter_batched(
            || {
                // Setup: 构建一半数据的树
                let mut rtree = RTree::new(config.max_entries);
                let half_size = config.size / 2;
                for i in half_size..config.size {
                    let (rect, data) = &test_data[i];
                    rtree.insert(rect.clone(), *data);
                }
                rtree
            },
            |mut rtree| {
                // Benchmark: 插入另一半数据
                let half_size = config.size / 2;
                for i in 0..half_size {
                    let (rect, data) = &test_data[i];
                    rtree.insert(rect.clone(), *data);
                }
                rtree
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// 删除全部数据性能测试
fn bench_remove_all(c: &mut Criterion) {
    let config = BenchConfig::default();
    let test_data = generate_test_data(config.size, config.seed);
    
    c.bench_function("remove_all", |b| {
        b.iter_batched(
            || {
                // Setup: 构建完整的树
                let mut rtree = RTree::new(config.max_entries);
                for (rect, data) in &test_data {
                    rtree.insert(rect.clone(), *data);
                }
                rtree
            },
            |mut rtree| {
                // Benchmark: 删除所有数据
                for (rect, data) in &test_data {
                    rtree.delete(rect, *data);
                }
                rtree
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_insert,
    bench_search_item,
    bench_search_area,
    bench_remove,
    bench_reinsert,
    bench_remove_all
);
criterion_main!(benches);
