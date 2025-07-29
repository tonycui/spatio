use rtree::{AsyncConcurrentRTree, Rectangle};
use std::sync::Arc;
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌳 异步并发 R-tree 演示");
    println!("============================");
    
    // 创建异步并发 R-tree
    let rtree = Arc::new(AsyncConcurrentRTree::with_timeout(
        4, 
        Duration::from_secs(5)
    ));
    
    println!("✅ 创建异步并发 R-tree (max_entries=4, timeout=5s)");
    
    // 演示 1：基础异步操作
    println!("\n📍 演示 1: 基础异步操作");
    let rect1 = Rectangle::new(0.0, 0.0, 1.0, 1.0);
    rtree.insert(rect1, 1).await?;
    println!("   插入矩形 (0,0,1,1) with data=1");
    
    let rect2 = Rectangle::new(2.0, 2.0, 3.0, 3.0);
    rtree.insert(rect2, 2).await?;
    println!("   插入矩形 (2,2,3,3) with data=2");
    
    let count = rtree.len().await?;
    println!("   当前 R-tree 包含 {} 个项目", count);
    
    // 演示 2：异步搜索
    println!("\n🔍 演示 2: 异步搜索");
    let search_area = Rectangle::new(-0.5, -0.5, 1.5, 1.5);
    let results = rtree.search(&search_area).await?;
    println!("   搜索区域 (-0.5,-0.5,1.5,1.5)");
    println!("   找到 {} 个相交的项目: {:?}", results.len(), results);
    
    // 演示 3：并发插入
    println!("\n⚡ 演示 3: 并发插入 (100 个项目)");
    let start = std::time::Instant::now();
    
    let mut tasks = Vec::new();
    for i in 0..100 {
        let rtree_clone = Arc::clone(&rtree);
        let task = tokio::spawn(async move {
            let x = (i % 10) as f64;
            let y = (i / 10) as f64;
            let rect = Rectangle::new(x, y, x + 0.8, y + 0.8);
            rtree_clone.insert(rect, i + 100).await
        });
        tasks.push(task);
    }
    
    // 等待所有插入完成
    for task in tasks {
        task.await??;
    }
    
    let duration = start.elapsed();
    let final_count = rtree.len().await?;
    println!("   并发插入完成，耗时: {:?}", duration);
    println!("   最终项目数量: {}", final_count);
    
    // 演示 4：并发读取
    println!("\n📖 演示 4: 并发读取");
    let start = std::time::Instant::now();
    
    let mut read_tasks = Vec::new();
    for i in 0..50 {
        let rtree_clone = Arc::clone(&rtree);
        let task = tokio::spawn(async move {
            let x = (i % 10) as f64;
            let y = (i / 10) as f64;
            let search_rect = Rectangle::new(x - 0.5, y - 0.5, x + 1.5, y + 1.5);
            let results = rtree_clone.search(&search_rect).await.unwrap();
            results.len()
        });
        read_tasks.push(task);
    }
    
    // 收集所有搜索结果
    let mut total_found = 0;
    for task in read_tasks {
        let found = task.await?;
        total_found += found;
    }
    
    let duration = start.elapsed();
    println!("   50 次并发搜索完成，耗时: {:?}", duration);
    println!("   总共找到 {} 个匹配项", total_found);
    
    // 演示 5：混合读写操作
    println!("\n🔄 演示 5: 混合读写操作");
    let start = std::time::Instant::now();
    
    let mut mixed_tasks = Vec::new();
    
    // 读任务
    for i in 0..20 {
        let rtree_clone = Arc::clone(&rtree);
        let task = tokio::spawn(async move {
            for j in 0..5 {
                let x = ((i + j) % 10) as f64;
                let y = ((i + j) / 10) as f64;
                let search_rect = Rectangle::new(x, y, x + 1.0, y + 1.0);
                let _results = rtree_clone.search(&search_rect).await.unwrap();
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
        mixed_tasks.push(task);
    }
    
    // 写任务
    for i in 0..10 {
        let rtree_clone = Arc::clone(&rtree);
        let task = tokio::spawn(async move {
            for j in 0..3 {
                let data = 1000 + i * 10 + j;
                let x = (data % 15) as f64;
                let y = (data / 15) as f64;
                let rect = Rectangle::new(x, y, x + 0.7, y + 0.7);
                rtree_clone.insert(rect, data).await.unwrap();
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        });
        mixed_tasks.push(task);
    }
    
    // 等待所有混合任务完成
    for task in mixed_tasks {
        task.await?;
    }
    
    let duration = start.elapsed();
    let final_count = rtree.len().await?;
    println!("   混合读写操作完成，耗时: {:?}", duration);
    println!("   最终项目数量: {}", final_count);
    
    // 演示 6：超时控制
    println!("\n⏱️  演示 6: 超时控制");
    let short_timeout = Duration::from_nanos(1); // 极短超时，用于演示
    let search_rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
    
    match rtree.search_with_timeout(&search_rect, short_timeout).await {
        Ok(results) => println!("   快速搜索成功，找到 {} 个项目", results.len()),
        Err(e) => println!("   预期的超时错误: {}", e),
    }
    
    // 使用正常超时进行搜索
    let normal_timeout = Duration::from_secs(1);
    match rtree.search_with_timeout(&search_rect, normal_timeout).await {
        Ok(results) => println!("   正常搜索成功，找到 {} 个项目", results.len()),
        Err(e) => println!("   搜索错误: {}", e),
    }
    
    // 最终统计
    println!("\n📊 最终统计");
    println!("   R-tree 是否为空: {}", rtree.is_empty().await?);
    println!("   总项目数量: {}", rtree.len().await?);
    println!("   默认超时时间: {:?}", rtree.default_timeout());
    
    println!("\n🎉 异步并发 R-tree 演示完成！");
    
    Ok(())
}
