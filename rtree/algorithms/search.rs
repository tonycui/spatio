use super::super::rectangle::Rectangle;
use super::super::node::{Node, Entry};
use super::super::rtree::RTree;
use geo::{Geometry, Intersects};
use super::super::rtree::GeoItem;
use super::utils::geometry_to_bbox;

/// 搜索操作相关算法
impl RTree {
    /// 搜索与查询矩形相交的所有条目 - 遵循论文Algorithm Search
    pub fn search(&self, geometry: &Geometry, limit: usize) -> Vec<GeoItem> {
        let bbox = geometry_to_bbox(geometry);
        let mut results = Vec::new();
        
        if let Some(root) = self.root_ref() {
            self.search_recursive(root, &bbox.unwrap(), geometry, &mut results, limit);
        }
        
        results
    }

    /// 仅使用边界框进行搜索（用于测试和简单查询）
    pub fn search_bbox(&self, query: &Rectangle) -> Vec<String> {
        let mut results = Vec::new();
        
        if let Some(root) = self.root_ref() {
            self.search_recursive_bbox_only(root, query, &mut results);
        }
        
        results
    }

    /// 递归搜索 - 遵循论文Search算法
    fn search_recursive(&self, node: &Node, query: &Rectangle, geometry: &Geometry, results: &mut Vec<GeoItem>, limit: usize) {
        // limit == 0 表示无限制，其他值表示有限制
        if limit > 0 && results.len() >= limit {
            return;
        }
        
        // S1: 搜索子树
        for entry in &node.entries {
            if entry.mbr().intersects(query) {
                match entry {
                    Entry::Data { data, .. } => {
                        // 根据 Geometry 进行精确比较
                        if let Some(entry_geometry) = self.geometry_map.get(data) {
                            if entry_geometry.intersects(geometry) {
                                // S2: 添加数据到结果
                                results.push(GeoItem {
                                    id: data.clone(),
                                    geometry: entry_geometry.clone(),
                                    geojson: self.geojson_map.get(data).cloned().unwrap_or_default(),
                                });
                                if limit > 0 && results.len() >= limit {
                                    return;
                                }
                            }
                        }
                    }
                    Entry::Node { node, .. } => {
                        // 递归搜索子节点
                        self.search_recursive(node, query, geometry, results, limit);
                        if limit > 0 && results.len() >= limit {
                            return;
                        }
                    }
                }
            }
        }
    }

    /// 递归搜索 - 仅边界框过滤（用于测试）
    fn search_recursive_bbox_only(&self, node: &Node, query: &Rectangle, results: &mut Vec<String>) {
        for entry in &node.entries {
            if entry.mbr().intersects(query) {
                match entry {
                    Entry::Data { data, .. } => {
                        results.push(data.clone());
                    }
                    Entry::Node { node, .. } => {
                        self.search_recursive_bbox_only(node, query, results);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Point, Polygon, Coord};

    #[test]
    fn test_search_with_geometry() {
        let mut rtree = RTree::new(4);
        
        // 插入一些几何体数据
        let point1 = Geometry::Point(Point::new(5.0, 5.0));
        let point2 = Geometry::Point(Point::new(10.0, 10.0));
        let point3 = Geometry::Point(Point::new(25.0, 25.0));
        
        rtree.insert_geometry("1".to_string(), point1.clone());
        rtree.insert_geometry("2".to_string(), point2.clone());
        rtree.insert_geometry("3".to_string(), point3.clone());
        
        // 创建查询几何体 - 一个包含点1和点2的多边形
        let query_polygon = Geometry::Polygon(Polygon::new(
            vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 15.0, y: 0.0 },
                Coord { x: 15.0, y: 15.0 },
                Coord { x: 0.0, y: 15.0 },
                Coord { x: 0.0, y: 0.0 }
            ].into(),
            vec![]
        ));
        
        // 搜索相交的几何体
        let results = rtree.search( &query_polygon,100);
        
        // 应该找到数据 1 和 2（在查询多边形内），但不包括 3
        // 检查 id 是否存在
    assert!(results.iter().any(|item| item.id == "1"));
    assert!(results.iter().any(|item| item.id == "2"));
    assert!(!results.iter().any(|item| item.id == "3"));
    assert_eq!(results.len(), 2);
}

#[test]
fn test_search_with_limit() {
    let mut rtree = RTree::new(4);
    
    // 插入5个点，都在查询范围内
    for i in 1..=5 {
        let point = Geometry::Point(Point::new(i as f64, i as f64));
        rtree.insert_geometry(i.to_string(), point);
    }
    
    // 创建一个包含所有点的查询几何体
    let query_polygon = Geometry::Polygon(Polygon::new(
        vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 6.0, y: 0.0 },
            Coord { x: 6.0, y: 6.0 },
            Coord { x: 0.0, y: 6.0 },
            Coord { x: 0.0, y: 0.0 }
        ].into(),
        vec![]
    ));
    
    // 测试无限制情况 - 应该返回所有5个结果
    let results_no_limit = rtree.search(&query_polygon, 0);
    assert_eq!(results_no_limit.len(), 5);
    
    // 测试limit = 3 - 应该只返回3个结果
    let results_limit_3 = rtree.search(&query_polygon, 3);
    assert_eq!(results_limit_3.len(), 3);
    
    // 测试limit = 1 - 应该只返回1个结果
    let results_limit_1 = rtree.search(&query_polygon, 1);
    assert_eq!(results_limit_1.len(), 1);
    
    // 测试limit大于实际结果数 - 应该返回所有5个结果
    let results_limit_10 = rtree.search(&query_polygon, 10);
    assert_eq!(results_limit_10.len(), 5);
}

#[test]
fn test_search_limit_early_termination() {
    let mut rtree = RTree::new(3); // 小的分支因子，创建多层树
    
    // 插入10个点
    for i in 1..=10 {
        let point = Geometry::Point(Point::new(i as f64, i as f64));
        rtree.insert_geometry(format!("item_{}", i), point);
    }
    
    // 创建查询几何体覆盖所有点
    let query_polygon = Geometry::Polygon(Polygon::new(
        vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 11.0, y: 0.0 },
            Coord { x: 11.0, y: 11.0 },
            Coord { x: 0.0, y: 11.0 },
            Coord { x: 0.0, y: 0.0 }
        ].into(),
        vec![]
    ));
    
    // 测试小的limit值，验证早期终止
    let results_limit_2 = rtree.search(&query_polygon, 2);
    assert_eq!(results_limit_2.len(), 2);
    
    let results_limit_5 = rtree.search(&query_polygon, 5);
    assert_eq!(results_limit_5.len(), 5);
    
    // 验证返回的结果都是有效的
    for result in &results_limit_5 {
        assert!(result.id.starts_with("item_"));
        assert!(!result.geojson.is_empty());
    }
}    #[test]
    fn test_search_polygon_intersection() {
        let mut rtree = RTree::new(4);
        
        // 插入一些多边形
        let poly1 = Geometry::Polygon(Polygon::new(
            vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 10.0, y: 0.0 },
                Coord { x: 10.0, y: 10.0 },
                Coord { x: 0.0, y: 10.0 },
                Coord { x: 0.0, y: 0.0 }
            ].into(),
            vec![]
        ));
        
        let poly2 = Geometry::Polygon(Polygon::new(
            vec![
                Coord { x: 5.0, y: 5.0 },
                Coord { x: 15.0, y: 5.0 },
                Coord { x: 15.0, y: 15.0 },
                Coord { x: 5.0, y: 15.0 },
                Coord { x: 5.0, y: 5.0 }
            ].into(),
            vec![]
        ));
        
        let poly3 = Geometry::Polygon(Polygon::new(
            vec![
                Coord { x: 20.0, y: 20.0 },
                Coord { x: 30.0, y: 20.0 },
                Coord { x: 30.0, y: 30.0 },
                Coord { x: 20.0, y: 30.0 },
                Coord { x: 20.0, y: 20.0 }
            ].into(),
            vec![]
        ));
        
        rtree.insert_geometry("1".to_string(), poly1);
        rtree.insert_geometry("2".to_string(), poly2);
        rtree.insert_geometry("3".to_string(), poly3);
        
        // 查询多边形：与poly1和poly2相交，但与poly3不相交
        let query_poly = Geometry::Polygon(Polygon::new(
            vec![
                Coord { x: 8.0, y: 8.0 },
                Coord { x: 12.0, y: 8.0 },
                Coord { x: 12.0, y: 12.0 },
                Coord { x: 8.0, y: 12.0 },
                Coord { x: 8.0, y: 8.0 }
            ].into(),
            vec![]
        ));
        
        let results = rtree.search( &query_poly,100);
        
        // 应该找到poly1和poly2（相交），但不包括poly3（不相交）
        // 检查 id 是否存在
        assert!(results.iter().any(|item| item.id == "1"));
        assert!(results.iter().any(|item| item.id == "2"));
        assert!(!results.iter().any(|item| item.id == "3"));
        assert_eq!(results.len(), 2);
    }
}
