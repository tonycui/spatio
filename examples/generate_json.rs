use rtree::{RTree, Rectangle};
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("正在生成R-tree JSON数据用于可视化...");

    // 创建几个不同的测试场景
    generate_simple_tree()?;
    generate_complex_tree()?;
    generate_clustered_data()?;
    generate_uniform_data()?; // 新增均匀分布数据生成

    println!("JSON数据已生成完成！");
    println!("请使用以下文件进行可视化测试：");
    println!("1. visualization/example_data/simple_tree.json - 20个数据点，网格分布");
    println!("2. visualization/example_data/complex_tree.json - 24个数据点，多层复杂树");
    println!("3. visualization/example_data/clustered_data.json - 28个数据点，4个聚集区域");
    println!("4. visualization/example_data/uniform_data.json - 20个数据点，均匀分布");

    Ok(())
}

/// 生成简单的单层树
fn generate_simple_tree() -> Result<(), Box<dyn std::error::Error>> {
    let mut rtree = RTree::new(6); // 增大分支因子以容纳更多数据
    
    // 插入20个矩形数据，分布在不同区域
    let data_points = vec![
        // 第一行 (y=10-30)
        (10.0, 10.0, 20.0, 20.0, 1),
        (25.0, 15.0, 35.0, 25.0, 2),
        (40.0, 10.0, 50.0, 20.0, 3),
        (55.0, 15.0, 65.0, 25.0, 4),
        (70.0, 10.0, 80.0, 20.0, 5),
        
        // 第二行 (y=35-55)
        (15.0, 35.0, 25.0, 45.0, 6),
        (30.0, 40.0, 40.0, 50.0, 7),
        (45.0, 35.0, 55.0, 45.0, 8),
        (60.0, 40.0, 70.0, 50.0, 9),
        (75.0, 35.0, 85.0, 45.0, 10),
        
        // 第三行 (y=60-80)
        (20.0, 60.0, 30.0, 70.0, 11),
        (35.0, 65.0, 45.0, 75.0, 12),
        (50.0, 60.0, 60.0, 70.0, 13),
        (65.0, 65.0, 75.0, 75.0, 14),
        (80.0, 60.0, 90.0, 70.0, 15),
        
        // 分散的几个点
        (5.0, 5.0, 12.0, 12.0, 16),
        (95.0, 25.0, 105.0, 35.0, 17),
        (25.0, 85.0, 35.0, 95.0, 18),
        (85.0, 85.0, 95.0, 95.0, 19),
        (50.0, 50.0, 58.0, 58.0, 20),
    ];
    
    for (x1, y1, x2, y2, data) in data_points {
        rtree.insert(Rectangle::new(x1, y1, x2, y2), data);
    }

    let json = rtree.export_to_json()?;
    let mut file = File::create("visualization/example_data/simple_tree.json")?;
    file.write_all(json.as_bytes())?;
    
    println!("✓ 生成 simple_tree.json (20个数据点)");
    Ok(())
}

/// 生成多层复杂树
fn generate_complex_tree() -> Result<(), Box<dyn std::error::Error>> {
    let mut rtree = RTree::new(4); // 较小的分支因子创建多层结构
    
    // 生成24个数据点，分布在4个主要区域
    let mut data_id = 1;
    
    // 区域1：左下角 (0-40, 0-40)
    let region1_points = vec![
        (5.0, 5.0), (15.0, 8.0), (25.0, 12.0), (35.0, 7.0),
        (8.0, 20.0), (18.0, 25.0), (28.0, 30.0), (38.0, 22.0),
    ];
    
    for (x, y) in region1_points {
        rtree.insert(Rectangle::new(x, y, x + 6.0, y + 6.0), data_id);
        data_id += 1;
    }
    
    // 区域2：右下角 (60-100, 0-40)
    let region2_points = vec![
        (65.0, 5.0), (75.0, 8.0), (85.0, 12.0), (95.0, 7.0),
        (68.0, 20.0), (78.0, 25.0), (88.0, 30.0), (98.0, 22.0),
    ];
    
    for (x, y) in region2_points {
        rtree.insert(Rectangle::new(x, y, x + 6.0, y + 6.0), data_id);
        data_id += 1;
    }
    
    // 区域3：左上角 (0-40, 60-100)
    let region3_points = vec![
        (5.0, 65.0), (15.0, 68.0), (25.0, 72.0), (35.0, 67.0),
    ];
    
    for (x, y) in region3_points {
        rtree.insert(Rectangle::new(x, y, x + 6.0, y + 6.0), data_id);
        data_id += 1;
    }
    
    // 区域4：右上角 (60-100, 60-100)
    let region4_points = vec![
        (65.0, 65.0), (75.0, 68.0), (85.0, 72.0), (95.0, 67.0),
    ];
    
    for (x, y) in region4_points {
        rtree.insert(Rectangle::new(x, y, x + 6.0, y + 6.0), data_id);
        data_id += 1;
    }

    let json = rtree.export_to_json()?;
    let mut file = File::create("visualization/example_data/complex_tree.json")?;
    file.write_all(json.as_bytes())?;
    
    println!("✓ 生成 complex_tree.json (24个数据点，多层结构)");
    Ok(())
}

/// 生成聚集分布数据
fn generate_clustered_data() -> Result<(), Box<dyn std::error::Error>> {
    let mut rtree = RTree::new(5);
    let mut data_id = 1;
    
    // 创建4个主要聚集区域，每个区域6个点
    let clusters = vec![
        (25.0, 25.0),   // 左下聚集
        (75.0, 25.0),   // 右下聚集
        (25.0, 75.0),   // 左上聚集
        (75.0, 75.0),   // 右上聚集
    ];
    
    for (center_x, center_y) in clusters {
        // 在每个聚集中心周围添加6个数据点
        let cluster_points = vec![
            (center_x - 8.0, center_y - 8.0),
            (center_x + 8.0, center_y - 8.0),
            (center_x - 8.0, center_y + 8.0),
            (center_x + 8.0, center_y + 8.0),
            (center_x - 4.0, center_y),
            (center_x + 4.0, center_y),
        ];
        
        for (x, y) in cluster_points {
            rtree.insert(Rectangle::new(x, y, x + 4.0, y + 4.0), data_id);
            data_id += 1;
        }
    }
    
    // 添加一些分散的点作为"噪声"
    let scattered_points = vec![
        (10.0, 50.0),   // 左中
        (50.0, 10.0),   // 下中
        (90.0, 50.0),   // 右中
        (50.0, 90.0),   // 上中
    ];
    
    for (x, y) in scattered_points {
        rtree.insert(Rectangle::new(x, y, x + 4.0, y + 4.0), data_id);
        data_id += 1;
    }

    let json = rtree.export_to_json()?;
    let mut file = File::create("visualization/example_data/clustered_data.json")?;
    file.write_all(json.as_bytes())?;
    
    println!("✓ 生成 clustered_data.json (28个数据点，4个聚集区域)");
    Ok(())
}

/// 生成均匀分布的20个数据点
fn generate_uniform_data() -> Result<(), Box<dyn std::error::Error>> {
    let mut rtree = RTree::new(5);
    
    // 创建20个均匀分布的数据点
    let uniform_points = vec![
        // 第一行 (y=10-20)
        (10.0, 10.0, 18.0, 18.0, 1),
        (30.0, 12.0, 38.0, 20.0, 2),
        (50.0, 10.0, 58.0, 18.0, 3),
        (70.0, 12.0, 78.0, 20.0, 4),
        (90.0, 10.0, 98.0, 18.0, 5),
        
        // 第二行 (y=30-40)
        (15.0, 30.0, 23.0, 38.0, 6),
        (35.0, 32.0, 43.0, 40.0, 7),
        (55.0, 30.0, 63.0, 38.0, 8),
        (75.0, 32.0, 83.0, 40.0, 9),
        (95.0, 30.0, 103.0, 38.0, 10),
        
        // 第三行 (y=50-60)
        (20.0, 50.0, 28.0, 58.0, 11),
        (40.0, 52.0, 48.0, 60.0, 12),
        (60.0, 50.0, 68.0, 58.0, 13),
        (80.0, 52.0, 88.0, 60.0, 14),
        (100.0, 50.0, 108.0, 58.0, 15),
        
        // 第四行 (y=70-80)
        (25.0, 70.0, 33.0, 78.0, 16),
        (45.0, 72.0, 53.0, 80.0, 17),
        (65.0, 70.0, 73.0, 78.0, 18),
        (85.0, 72.0, 93.0, 80.0, 19),
        (105.0, 70.0, 113.0, 78.0, 20),
    ];
    
    for (x1, y1, x2, y2, data) in uniform_points {
        rtree.insert(Rectangle::new(x1, y1, x2, y2), data);
    }

    let json = rtree.export_to_json()?;
    let mut file = File::create("visualization/example_data/uniform_data.json")?;
    file.write_all(json.as_bytes())?;
    
    println!("✓ 生成 uniform_data.json (20个数据点，均匀分布)");
    Ok(())
}
