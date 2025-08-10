use geo::{Geometry};

use crate::algorithms::utils::geometry_to_bbox;
use crate::rectangle::Rectangle;
use crate::node::{Node, Entry};
use crate::rtree::RTree;
use geojson::Value;

/// 插入操作相关算法
impl RTree {
    /// 插入新的数据条目 - 遵循论文Algorithm Insert
    pub fn insert_geometry(&mut self, data: String, geometry: Geometry) {
        
        let rect;
        match geometry_to_bbox(&geometry) {
            Ok(bbox) => rect = bbox,
            Err(e) => {
                eprintln!("Error calculating bounding box: {}", e);
                return;
            }
        }

        self.insert(rect, data.clone());
        // 将几何体转换为GeoJSON格式并存储
        let geojson_value: Value = Value::from(&geometry);
        self.geometry_map.insert(data.clone(), geometry);
        self.geojson_map.insert(data.clone(), geojson_value.to_string());

    }
    /// 插入新的数据条目 - 遵循论文Algorithm Insert
    pub fn insert(&mut self, rect: Rectangle, data: String) {
        // I1: 如果根节点不存在，创建根节点
        if self.root_ref().is_none() {
            let mut root = Node::new_leaf_node();
            root.add_entry(Entry::Data { mbr: rect, data });
            *self.root_mut() = Some(Box::new(root));
            return;
        }

        // I2: 选择叶子节点
        let leaf_path = self.choose_leaf_path(&rect);
        
        // I3: 添加记录到叶子节点
        let max_entries = self.max_entries_internal();
        let leaf_node = match self.get_last_node_mut(&leaf_path) {
            Some(node) => node,
            None => {
                // 如果无法获取叶子节点，说明路径有问题，这是一个严重的错误
                panic!("Failed to get leaf node during insertion");
            }
        };
        leaf_node.add_entry(Entry::Data { mbr: rect, data });
        
        // I4: 检查是否需要分裂并调整树
        if leaf_node.entries.len() > max_entries {
            self.handle_overflow(leaf_path);
        } else {
            // 只需要更新MBR
            self.adjust_tree_upward(leaf_path);
        }
    }

    /// 选择叶子节点路径 - 遵循论文ChooseLeaf算法
    fn choose_leaf_path(&self, rect: &Rectangle) -> Vec<usize> {
        let mut path = Vec::new();
        let mut current = self.root_ref().as_ref().unwrap();
        
        // CL1: 初始化，从根节点开始
        // CL2: 叶子检查
        while !current.is_leaf_node() {
            
            // CL3: 选择子树 - 选择扩大面积最小的条目
            let best_index = self.choose_subtree(&current.entries, rect);
            path.push(best_index);
            
            // CL4: 下降到子节点
            if let Some(Entry::Node { node, .. }) = current.entries.get(best_index) {
                current = node;
            }
        }
        
        path
    }

    /// 选择子树 - 计算扩大面积最小的条目
    fn choose_subtree(&self, entries: &[Entry], rect: &Rectangle) -> usize {
        let mut best_index = 0;
        let mut min_enlargement = f64::INFINITY;
        let mut min_area = f64::INFINITY;
        
        for (i, entry) in entries.iter().enumerate() {
            let mbr = entry.mbr();
            let enlargement = mbr.enlargement(rect);
            let area = mbr.area();
            
            // 选择扩大面积最小的，如果相同则选择面积最小的
            if enlargement < min_enlargement || 
               (enlargement == min_enlargement && area < min_area) {
                min_enlargement = enlargement;
                min_area = area;
                best_index = i;
            }
        }
        
        best_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Point, Polygon, Coord};

    #[test]
    fn test_insert_basic() {
        let mut rtree = RTree::new(4);
        
        // 测试插入到空树
        assert!(rtree.is_empty());
        rtree.insert(Rectangle::new(0.0, 0.0, 10.0, 10.0), "1".to_string());
        assert_eq!(rtree.len(), 1);
        assert!(!rtree.is_empty());
        
        // 测试插入多个条目
        rtree.insert(Rectangle::new(5.0, 5.0, 15.0, 15.0), "2".to_string());
        rtree.insert(Rectangle::new(20.0, 20.0, 30.0, 30.0), "3".to_string());
        assert_eq!(rtree.len(), 3);
    }

    #[test]
    fn test_insert_geometry_point() {
        let mut rtree = RTree::new(4);
        
        // 创建一个点几何体
        let point = Geometry::Point(Point::new(5.0, 10.0));
        let data_id = "42";
        
        // 插入几何体 - 不再需要手动传递 rect
        rtree.insert_geometry(data_id.to_string(), point.clone());
        
        // 验证空间索引中包含该数据
        assert_eq!(rtree.len(), 1);
        
        // 验证 geometry_map 中存储了几何体
        assert!(rtree.geometry_map.contains_key(data_id));
        let stored_geometry = rtree.geometry_map.get(data_id).unwrap();
        match stored_geometry {
            Geometry::Point(p) => {
                assert_eq!(p.x(), 5.0);
                assert_eq!(p.y(), 10.0);
            }
            _ => panic!("Expected Point geometry"),
        }
        
        // 验证 geojson_map 中存储了 GeoJSON 字符串
        assert!(rtree.geojson_map.contains_key(data_id));
        let geojson_str = rtree.geojson_map.get(data_id).unwrap();
        assert!(geojson_str.contains("Point"));
        assert!(geojson_str.contains("5"));
        assert!(geojson_str.contains("10"));
    }

    #[test]
    fn test_insert_geometry_polygon() {
        let mut rtree = RTree::new(4);
        
        // 创建一个多边形几何体
        let coords = vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 4.0, y: 0.0 },
            Coord { x: 4.0, y: 4.0 },
            Coord { x: 0.0, y: 4.0 },
            Coord { x: 0.0, y: 0.0 },
        ];
        let polygon = Geometry::Polygon(Polygon::new(coords.into(), vec![]));
        let data_id = "123".to_string();
        
        // 插入几何体 - 不再需要手动传递 rect
        rtree.insert_geometry(data_id.clone(), polygon.clone());
        
        // 验证空间索引中包含该数据
        assert_eq!(rtree.len(), 1);
        
        // 验证 geometry_map 中存储了几何体
        assert!(rtree.geometry_map.contains_key(&data_id));
        let stored_geometry = rtree.geometry_map.get(&data_id).unwrap();
        match stored_geometry {
            Geometry::Polygon(p) => {
                assert_eq!(p.exterior().0.len(), 5); // 5个点（首尾相同）
            }
            _ => panic!("Expected Polygon geometry"),
        }
        
        // 验证 geojson_map 中存储了 GeoJSON 字符串
        assert!(rtree.geojson_map.contains_key(&data_id));
        let geojson_str = rtree.geojson_map.get(&data_id).unwrap();
        assert!(geojson_str.contains("Polygon"));
    }

    #[test]
    fn test_insert_multiple_geometries() {
        let mut rtree = RTree::new(4);
        
        // 插入多个不同类型的几何体
        let point = Geometry::Point(Point::new(1.0, 1.0));
        rtree.insert_geometry("1".to_string(), point);

        let coords = vec![
            Coord { x: 5.0, y: 5.0 },
            Coord { x: 8.0, y: 5.0 },
            Coord { x: 8.0, y: 8.0 },
            Coord { x: 5.0, y: 8.0 },
            Coord { x: 5.0, y: 5.0 },
        ];
        let polygon = Geometry::Polygon(Polygon::new(coords.into(), vec![]));
        rtree.insert_geometry("2".to_string(), polygon);
        
        // 验证两个几何体都被正确存储
        assert_eq!(rtree.len(), 2);
        assert_eq!(rtree.geometry_map.len(), 2);
        assert_eq!(rtree.geojson_map.len(), 2);
        
        // 验证每个几何体的类型
        match rtree.geometry_map.get("1").unwrap() {
            Geometry::Point(_) => {},
            _ => panic!("Expected Point for ID 1"),
        }

        match rtree.geometry_map.get("2").unwrap() {
            Geometry::Polygon(_) => {},
            _ => panic!("Expected Polygon for ID 2"),
        }
        
        // 验证 GeoJSON 字符串包含正确的类型标识
        assert!(rtree.geojson_map.get("1").unwrap().contains("Point"));
        assert!(rtree.geojson_map.get("2").unwrap().contains("Polygon"));
    }

    #[test]
    fn test_insert_geometry_consistency() {
        let mut rtree = RTree::new(4);
        
        // 测试 insert_geometry 调用了 insert 方法
        let point = Geometry::Point(Point::new(3.0, 7.0));
        let data_id = "999".to_string();

        let initial_len = rtree.len();
        rtree.insert_geometry(data_id.clone(), point);

        // 验证空间索引被更新（len 增加）
        assert_eq!(rtree.len(), initial_len + 1);
        
        // 验证数据映射被更新
        assert!(rtree.geometry_map.contains_key(&data_id));
        assert!(rtree.geojson_map.contains_key(&data_id));
        
        // 验证空间查询能找到该数据 - 使用点的边界框
        let search_rect = Rectangle::new(3.0, 7.0, 3.0, 7.0);
        let search_results = rtree.search_bbox(&search_rect);
        assert!(search_results.contains(&data_id));
    }

    #[test]
    fn test_insert_geometry_bbox_calculation() {
        let mut rtree = RTree::new(4);
        
        // 测试几何体边界框自动计算
        let coords = vec![
            Coord { x: 1.0, y: 1.0 },
            Coord { x: 5.0, y: 1.0 },
            Coord { x: 5.0, y: 4.0 },
            Coord { x: 1.0, y: 4.0 },
            Coord { x: 1.0, y: 1.0 },
        ];
        let polygon = Geometry::Polygon(Polygon::new(coords.into(), vec![]));
        let data_id = "555".to_string();

        rtree.insert_geometry(data_id.clone(), polygon);

        // 验证能够通过计算出的边界框范围进行空间查询
        let search_rect = Rectangle::new(0.5, 0.5, 5.5, 4.5); // 包含整个多边形
        let results = rtree.search_bbox(&search_rect);
        assert!(results.contains(&data_id));
        
        // 验证不包含多边形的查询范围不会找到该数据
        let no_overlap_rect = Rectangle::new(10.0, 10.0, 15.0, 15.0);
        let no_results = rtree.search_bbox(&no_overlap_rect);
        assert!(!no_results.contains(&data_id));
        
        // 验证部分重叠的查询范围能找到该数据
        let partial_overlap_rect = Rectangle::new(2.0, 2.0, 3.0, 3.0); // 部分重叠
        let partial_results = rtree.search_bbox(&partial_overlap_rect);
        assert!(partial_results.contains(&data_id));
    }

    #[test]
    fn test_choose_leaf_path() {
        let mut rtree = RTree::new(3); // 小的max_entries以便测试分裂
        
        // 插入足够多的数据以创建多层树结构
        for i in 0..6 {
            let x = (i as f64) * 2.0;
            rtree.insert(Rectangle::new(x, 0.0, x + 1.0, 1.0), i.to_string());
        }
        
        // 测试选择叶子路径
        let rect = Rectangle::new(0.5, 0.5, 1.5, 1.5);
        if let Some(root) = rtree.root_ref() {
            if !root.is_leaf_node() {
                let path = rtree.choose_leaf_path(&rect);
                assert!(!path.is_empty());
            }
        }
    }

    #[test]
    fn test_choose_subtree() {
        let rtree = RTree::new(4);
        
        // 创建一些测试条目
        let entries = vec![
            Entry::Data { mbr: Rectangle::new(0.0, 0.0, 5.0, 5.0), data: "1".to_string() },
            Entry::Data { mbr: Rectangle::new(10.0, 10.0, 15.0, 15.0), data: "2".to_string() },
            Entry::Data { mbr: Rectangle::new(20.0, 20.0, 25.0, 25.0), data: "3".to_string() },
        ];
        
        // 测试选择最合适的子树
        let test_rect = Rectangle::new(2.0, 2.0, 3.0, 3.0);
        let best_index = rtree.choose_subtree(&entries, &test_rect);
        
        // 应该选择第一个条目，因为它与测试矩形重叠
        assert_eq!(best_index, 0);
    }
}
