use rtree::{ConcurrentRTree, Rectangle};
use geo::{Point, Geometry};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    println!("=== R-tree并发功能演示 ===\n");
    
    // 1. 基础并发操作演示
    println!("1. 基础并发操作演示");
    basic_concurrent_demo();
    
    // 2. 高并发读取测试
    println!("\n2. 高并发读取测试");
    concurrent_read_demo();
    
    // 3. 并发写入测试
    println!("\n3. 并发写入测试");
    concurrent_write_demo();
    
    // 4. 混合读写测试
    println!("\n4. 混合读写测试");
    mixed_operations_demo();
    
    // 5. 性能对比
    println!("\n5. 单线程 vs 多线程性能对比");
    performance_comparison();
    
    println!("\n=== 演示完成 ===");
}

fn basic_concurrent_demo() {
    let rtree = ConcurrentRTree::new(4);
    
    // 预填充一些数据
    println!("   预填充数据...");
    for i in 0..10 {
        let point = Geometry::Point(Point::new(i as f64 + 0.5, i as f64 + 0.5));
        rtree.insert(point, i).unwrap();
    }
    println!("   初始数据量: {}", rtree.len().unwrap());
    
    // 创建多个线程进行各种操作
    let handles: Vec<_> = (0..3).map(|thread_id| {
        let rtree_clone = rtree.clone(); // 使用clone而不是Arc::clone
        thread::spawn(move || {
            match thread_id {
                0 => {
                    // 线程0: 查询操作
                    let search_rect = Rectangle::new(2.5, 2.5, 5.5, 5.5);
                    let results = rtree_clone.search(&search_rect).unwrap();
                    println!("   线程{}(查询): 找到 {} 个结果", thread_id, results.len());
                }
                1 => {
                    // 线程1: 插入操作
                    let new_point = Geometry::Point(Point::new(20.5, 20.5));
                    rtree_clone.insert(new_point, 100).unwrap();
                    println!("   线程{}(插入): 插入成功", thread_id);
                }
                2 => {
                    // 线程2: 删除操作
                    thread::sleep(Duration::from_millis(10)); // 稍等一下
                    let deleted = rtree_clone.delete(0).unwrap();
                    println!("   线程{}(删除): 删除{}", thread_id, if deleted { "成功" } else { "失败" });
                }
                _ => unreachable!()
            }
        })
    }).collect();
    
    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    
    println!("   最终数据量: {}", rtree.len().unwrap());
}

fn concurrent_read_demo() {
    let rtree = ConcurrentRTree::new(8);
    
    // 填充测试数据
    println!("   填充1000个数据项...");
    for i in 0..1000 {
        let x = (i % 100) as f64;
        let y = (i / 100) as f64;
        let point = Geometry::Point(Point::new(x + 0.5, y + 0.5));
        rtree.insert(point, i).unwrap();
    }
    
    println!("   启动10个并发读取线程...");
    let start = Instant::now();
    
    let handles: Vec<_> = (0..10).map(|thread_id| {
        let rtree_clone = rtree.clone(); // 使用clone
        thread::spawn(move || {
            let mut total_results = 0;
            
            // 每个线程执行100次查询
            for i in 0..100 {
                let x = ((thread_id * 100 + i) % 50) as f64;
                let y = ((thread_id * 100 + i) / 50) as f64;
                let search_rect = Rectangle::new(x, y, x + 10.0, y + 10.0);
                let results = rtree_clone.search(&search_rect).unwrap();
                total_results += results.len();
            }
            
            println!("   线程{}: 完成100次查询，总计找到{}个结果", thread_id, total_results);
            total_results
        })
    }).collect();
    
    // 等待所有线程并统计结果
    let mut grand_total = 0;
    for handle in handles {
        grand_total += handle.join().unwrap();
    }
    
    let elapsed = start.elapsed();
    println!("   并发读取完成: 10个线程, 1000次查询, 找到{}个结果, 耗时{:?}", 
        grand_total, elapsed);
    println!("   平均QPS: {:.0}", 1000.0 / elapsed.as_secs_f64());
}

fn concurrent_write_demo() {
    let rtree = ConcurrentRTree::new(8);
    
    println!("   启动5个并发写入线程...");
    let start = Instant::now();
    
    let handles: Vec<_> = (0..5).map(|thread_id| {
        let rtree_clone = rtree.clone();
        thread::spawn(move || {
            // 每个线程插入100个项目
            for i in 0..100 {
                let data_id = thread_id * 1000 + i;
                let x = (data_id % 50) as f64;
                let y = (data_id / 50) as f64;
                let point = Geometry::Point(Point::new(x + 0.5, y + 0.5));
                rtree_clone.insert(point, data_id).unwrap();
                
                // 偶尔暂停一下，模拟真实场景
                if i % 20 == 0 {
                    thread::sleep(Duration::from_micros(100));
                }
            }
            
            println!("   线程{}: 完成100次插入", thread_id);
        })
    }).collect();
    
    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    
    let elapsed = start.elapsed();
    let final_count = rtree.len().unwrap();
    println!("   并发写入完成: 插入{}个项目, 耗时{:?}", final_count, elapsed);
    println!("   平均插入速率: {:.0} items/sec", final_count as f64 / elapsed.as_secs_f64());
}

fn mixed_operations_demo() {
    let rtree = ConcurrentRTree::new(8);
    
    // 预填充一些基础数据
    for i in 0..50 {
        let point = Geometry::Point(Point::new(i as f64 + 0.5, i as f64 + 0.5));
        rtree.insert(point, i).unwrap();
    }
    
    println!("   启动混合操作: 3个读线程 + 2个写线程...");
    let start = Instant::now();
    
    let handles: Vec<_> = (0..5).map(|thread_id| {
        let rtree_clone = rtree.clone();
        thread::spawn(move || {
            match thread_id {
                0 | 1 | 2 => {
                    // 读线程
                    let mut read_count = 0;
                    for i in 0..200 {
                        let x = (i % 30) as f64;
                        let y = (i / 30) as f64;
                        let search_rect = Rectangle::new(x, y, x + 5.0, y + 5.0);
                        let results = rtree_clone.search(&search_rect).unwrap();
                        read_count += results.len();
                        
                        // 模拟处理时间
                        thread::sleep(Duration::from_micros(50));
                    }
                    println!("   读线程{}: 完成200次查询，找到{}个结果", thread_id, read_count);
                }
                3 | 4 => {
                    // 写线程
                    let mut write_count = 0;
                    for i in 0..50 {
                        let data_id = thread_id * 1000 + i;
                        let x = 100.0 + (data_id % 20) as f64;
                        let y = (data_id / 20) as f64;
                        let point = Geometry::Point(Point::new(x + 0.5, y + 0.5));
                        rtree_clone.insert(point, data_id).unwrap();
                        write_count += 1;
                        
                        // 写操作比读操作慢一些
                        thread::sleep(Duration::from_millis(2));
                    }
                    println!("   写线程{}: 完成{}次插入", thread_id, write_count);
                }
                _ => unreachable!()
            }
        })
    }).collect();
    
    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    
    let elapsed = start.elapsed();
    let final_count = rtree.len().unwrap();
    println!("   混合操作完成: 最终数据量{}, 总耗时{:?}", final_count, elapsed);
}

fn performance_comparison() {
    let data_count = 10000;
    
    // 单线程性能测试
    println!("   单线程插入{}项...", data_count);
    let start = Instant::now();
    let single_tree = ConcurrentRTree::new(8);
    
    for i in 0..data_count {
        let x = (i % 100) as f64;
        let y = (i / 100) as f64;
        let point = Geometry::Point(Point::new(x + 0.5, y + 0.5));
        single_tree.insert(point, i).unwrap();
    }
    
    let single_elapsed = start.elapsed();
    println!("   单线程完成，耗时{:?}", single_elapsed);
    
    // 多线程性能测试
    println!("   多线程插入{}项(4个线程)...", data_count);
    let multi_tree = ConcurrentRTree::new(8);
    let start = Instant::now();
    
    let handles: Vec<_> = (0..4).map(|thread_id| {
        let rtree_clone = multi_tree.clone();
        let items_per_thread = data_count / 4;
        
        thread::spawn(move || {
            let start_id = thread_id * items_per_thread;
            let end_id = if thread_id == 3 { 
                data_count // 最后一个线程处理剩余的项目
            } else { 
                start_id + items_per_thread 
            };
            
            for i in start_id..end_id {
                let x = (i % 100) as f64;
                let y = (i / 100) as f64;
                let point = Geometry::Point(Point::new(x + 0.5, y + 0.5));
                rtree_clone.insert(point, i).unwrap();
            }
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let multi_elapsed = start.elapsed();
    println!("   多线程完成，耗时{:?}", multi_elapsed);
    
    // 性能分析
    let speedup = single_elapsed.as_secs_f64() / multi_elapsed.as_secs_f64();
    println!("   性能对比:");
    println!("     单线程速率: {:.0} items/sec", data_count as f64 / single_elapsed.as_secs_f64());
    println!("     多线程速率: {:.0} items/sec", data_count as f64 / multi_elapsed.as_secs_f64());
    println!("     加速比: {:.2}x", speedup);
    
    // 验证数据完整性
    assert_eq!(single_tree.len().unwrap(), data_count as usize);
    assert_eq!(multi_tree.len().unwrap(), data_count as usize);
    println!("   ✅ 数据完整性验证通过");
}
