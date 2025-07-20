use rtree::{RTree, Rectangle};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== R-tree持久化功能演示 ===\n");
    
    // 1. 创建并填充R-tree
    println!("1. 创建R-tree并插入数据...");
    let mut rtree = RTree::new(4);
    
    // 插入一些测试数据
    let data = vec![
        (Rectangle::new(0.0, 0.0, 1.0, 1.0), 1),
        (Rectangle::new(2.0, 2.0, 3.0, 3.0), 2),
        (Rectangle::new(5.0, 5.0, 6.0, 6.0), 3),
        (Rectangle::new(1.5, 1.5, 2.5, 2.5), 4),
        (Rectangle::new(3.5, 3.5, 4.5, 4.5), 5),
    ];
    
    for (rect, id) in data {
        rtree.insert(rect, id);
    }
    
    println!("   插入了 {} 个数据项", rtree.len());
    
    // 2. 导出为JSON格式（调试用）
    println!("\n2. 导出为JSON格式...");
    rtree.dump_to_file("example_data.json")?;
    
    let json_size = fs::metadata("example_data.json")?.len();
    println!("   JSON文件大小: {} bytes", json_size);
    
    // 查看JSON文件内容的前几行
    let json_content = fs::read_to_string("example_data.json")?;
    let lines: Vec<&str> = json_content.lines().take(10).collect();
    println!("   JSON文件内容预览:");
    for line in lines {
        println!("     {}", line);
    }
    if json_content.lines().count() > 10 {
        println!("     ... (更多内容)");
    }
    
    // 3. 导出为二进制格式（生产用）
    println!("\n3. 导出为二进制格式...");
    rtree.dump_to_file("example_data.bin")?;
    
    let bin_size = fs::metadata("example_data.bin")?.len();
    println!("   二进制文件大小: {} bytes", bin_size);
    println!("   文件大小比较: JSON {} bytes vs Binary {} bytes", json_size, bin_size);
    
    // 4. 从JSON文件加载
    println!("\n4. 从JSON文件加载R-tree...");
    let rtree_from_json = RTree::load_from_file("example_data.json")?;
    println!("   加载成功，数据项数量: {}", rtree_from_json.len());
    
    // 5. 从二进制文件加载
    println!("\n5. 从二进制文件加载R-tree...");
    let rtree_from_binary = RTree::load_from_file("example_data.bin")?;
    println!("   加载成功，数据项数量: {}", rtree_from_binary.len());
    
    // 6. 验证数据一致性
    println!("\n6. 验证数据一致性...");
    
    // 测试搜索功能
    let search_rect = Rectangle::new(0.5, 0.5, 2.5, 2.5);
    let original_results = rtree.search(&search_rect);
    let json_results = rtree_from_json.search(&search_rect);
    let binary_results = rtree_from_binary.search(&search_rect);
    
    println!("   搜索区域 [{:.1}, {:.1}, {:.1}, {:.1}]:", 
        search_rect.min[0], search_rect.min[1], search_rect.max[0], search_rect.max[1]);
    println!("   原始R-tree找到: {} 个结果", original_results.len());
    println!("   JSON加载R-tree找到: {} 个结果", json_results.len());
    println!("   二进制加载R-tree找到: {} 个结果", binary_results.len());
    
    // 验证结果一致性
    let consistent = original_results.len() == json_results.len() 
        && json_results.len() == binary_results.len()
        && rtree.len() == rtree_from_json.len()
        && rtree_from_json.len() == rtree_from_binary.len();
    
    if consistent {
        println!("   ✅ 数据一致性验证通过！");
    } else {
        println!("   ❌ 数据一致性验证失败！");
    }
    
    // 7. 性能比较（简单测试）
    println!("\n7. 性能比较...");
    
    use std::time::Instant;
    
    // JSON序列化性能
    let start = Instant::now();
    rtree.dump_to_file("perf_test.json")?;
    let json_write_time = start.elapsed();
    
    // 二进制序列化性能
    let start = Instant::now();
    rtree.dump_to_file("perf_test.bin")?;
    let binary_write_time = start.elapsed();
    
    // JSON反序列化性能
    let start = Instant::now();
    let _ = RTree::load_from_file("perf_test.json")?;
    let json_read_time = start.elapsed();
    
    // 二进制反序列化性能
    let start = Instant::now();
    let _ = RTree::load_from_file("perf_test.bin")?;
    let binary_read_time = start.elapsed();
    
    println!("   JSON写入时间: {:?}", json_write_time);
    println!("   二进制写入时间: {:?}", binary_write_time);
    println!("   JSON读取时间: {:?}", json_read_time);
    println!("   二进制读取时间: {:?}", binary_read_time);
    
    // 8. 清理临时文件
    println!("\n8. 清理临时文件...");
    let temp_files = ["example_data.json", "example_data.bin", "perf_test.json", "perf_test.bin"];
    for file in temp_files {
        if fs::remove_file(file).is_ok() {
            println!("   删除文件: {}", file);
        }
    }
    
    println!("\n=== 演示完成 ===");
    println!("\n使用建议:");
    println!("• 开发和调试时使用 .json 扩展名，可以直接查看文件内容");
    println!("• 生产环境使用 .bin 或 .rtree 扩展名，获得更好的性能和更小的文件");
    println!("• 支持的文件格式:");
    println!("  - .json -> JSON格式（可读性好）");
    println!("  - .bin/.rtree/其他 -> 二进制格式（高性能）");
    
    Ok(())
}
