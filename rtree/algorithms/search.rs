use super::super::rectangle::Rectangle;
use super::super::node::{Node, Entry};
use super::super::rtree::RTree;
use geo::{Geometry, Intersects, Within};
use super::super::rtree::GeoItem;
use super::utils::geometry_to_bbox;

#[cfg(test)]
use crate::storage::geometry_utils::geometry_to_geojson;

/// 搜索操作相关算法
impl RTree {
    /// 搜索与查询几何体相交或完全包含在其中的所有条目
    /// within: true = 完全包含在 geometry 内部, false = 与 geometry 相交
    pub fn search(&self, geometry: &Geometry, limit: usize, within: bool) -> Vec<GeoItem> {
        let bbox = geometry_to_bbox(geometry);
        let mut results = Vec::new();
        
        if let Some(root) = self.root_ref() {
            self.search_recursive(root, &bbox.unwrap(), geometry, &mut results, limit, within);
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
    /// within: true = 完全包含在 geometry 内部, false = 与 geometry 相交
    fn search_recursive(&self, node: &Node, query: &Rectangle, geometry: &Geometry, results: &mut Vec<GeoItem>, limit: usize, within: bool) {
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
                            let matches = if within {
                                // Within 查询：entry_geometry 必须完全包含在 geometry 内部
                                entry_geometry.is_within(geometry)
                            } else {
                                // Intersects 查询：entry_geometry 与 geometry 相交
                                entry_geometry.intersects(geometry)
                            };
                            
                            if matches {
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
                        self.search_recursive(node, query, geometry, results, limit, within);
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

    /// 查找最近的 k 个对象（KNN 查询）
    /// 
    /// 使用 R-tree 的 KNN 算法，通过优先队列高效地查找距离查询点最近的 k 个对象。
    /// 
    /// # Arguments
    /// * `query_lon` - 查询点的经度
    /// * `query_lat` - 查询点的纬度
    /// * `k` - 返回最近的 k 个对象
    /// 
    /// # Returns
    /// 
    /// 返回一个元组数组 `Vec<(GeoItem, f64)>`，其中：
    /// - `GeoItem` - 查询到的地理对象
    /// - `f64` - 该对象到查询点的距离（米，使用 Haversine 公式计算）
    /// 
    /// 结果按距离升序排列（最近的在前）
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// let tree = RTree::new(10);
    /// // ... insert some data ...
    /// let results = tree.nearby(116.4, 39.9, 10); // 查找北京附近最近的10个对象
    /// for (item, distance) in results {
    ///     println!("Found {} at distance {} meters", item.id, distance);
    /// }
    /// ```
    pub fn nearby(&self, query_lon: f64, query_lat: f64, k: usize) -> Vec<(GeoItem, f64)> {
        use super::knn::knn_search;

        // 直接传递 geometry_map 和 geojson_map 的引用，避免复制整个数据集
        let knn_results = knn_search(
            self.get_root(),
            query_lon,
            query_lat,
            k,
            &self.geometry_map,
            &self.geojson_map,
        );

        // 转换结果为 (GeoItem, distance) 元组
        knn_results
            .into_iter()
            .map(|result| (result.item, result.distance))
            .collect()
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
        
        rtree.insert_geojson("1".to_string(), &geometry_to_geojson(&point1).to_string());
        rtree.insert_geojson("2".to_string(), &geometry_to_geojson(&point2).to_string());
        rtree.insert_geojson("3".to_string(), &geometry_to_geojson(&point3).to_string());
        
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
        let results = rtree.search(&query_polygon, 100, false);
        
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
        rtree.insert_geojson(i.to_string(), &geometry_to_geojson(&point).to_string());
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
    let results_no_limit = rtree.search(&query_polygon, 0, false);
    assert_eq!(results_no_limit.len(), 5);
    
    // 测试limit = 3 - 应该只返回3个结果
    let results_limit_3 = rtree.search(&query_polygon, 3, false);
    assert_eq!(results_limit_3.len(), 3);
    
    // 测试limit = 1 - 应该只返回1个结果
    let results_limit_1 = rtree.search(&query_polygon, 1, false);
    assert_eq!(results_limit_1.len(), 1);
    
    // 测试limit大于实际结果数 - 应该返回所有5个结果
    let results_limit_10 = rtree.search(&query_polygon, 10, false);
    assert_eq!(results_limit_10.len(), 5);
}

#[test]
fn test_search_limit_early_termination() {
    let mut rtree = RTree::new(3); // 小的分支因子，创建多层树
    
    // 插入10个点
    for i in 1..=10 {
        let point = Geometry::Point(Point::new(i as f64, i as f64));
        rtree.insert_geojson(format!("item_{}", i), &geometry_to_geojson(&point).to_string());
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
    let results_limit_2 = rtree.search(&query_polygon, 2, false);
    assert_eq!(results_limit_2.len(), 2);
    
    let results_limit_5 = rtree.search(&query_polygon, 5, false);
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
        
        rtree.insert_geojson("1".to_string(), &geometry_to_geojson(&poly1).to_string());
        rtree.insert_geojson("2".to_string(), &geometry_to_geojson(&poly2).to_string());
        rtree.insert_geojson("3".to_string(), &geometry_to_geojson(&poly3).to_string());
        
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
        
        let results = rtree.search(&query_poly, 100, false);
        
        // 应该找到poly1和poly2（相交），但不包括poly3（不相交）
        // 检查 id 是否存在
        assert!(results.iter().any(|item| item.id == "1"));
        assert!(results.iter().any(|item| item.id == "2"));
        assert!(!results.iter().any(|item| item.id == "3"));
        assert_eq!(results.len(), 2);
    }
}
