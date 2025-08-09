use crate::rectangle::Rectangle;
use crate::node::{Node, Entry};
use crate::rtree::RTree;
use geo::{Geometry, Intersects};

/// 搜索操作相关算法
impl RTree {
    /// 搜索与查询矩形相交的所有条目 - 遵循论文Algorithm Search
    pub fn search(&self, query: &Rectangle, geometry: &Geometry) -> Vec<i32> {
        let mut results = Vec::new();
        
        if let Some(root) = self.root_ref() {
            self.search_recursive(root, query, geometry, &mut results);
        }
        
        results
    }

    /// 仅使用边界框进行搜索（用于测试和简单查询）
    /// todo : 现在只有大量的 test使用search_bbox，他们之前是使用 search 的，这个应该改为使用 search，并删除此方法。
    pub fn search_bbox(&self, query: &Rectangle) -> Vec<i32> {
        let mut results = Vec::new();
        
        if let Some(root) = self.root_ref() {
            self.search_recursive_bbox_only(root, query, &mut results);
        }
        
        results
    }

    /// 递归搜索 - 遵循论文Search算法
    fn search_recursive(&self, node: &Node, query: &Rectangle, geometry: &Geometry, results: &mut Vec<i32>) {
        // S1: 搜索子树
        for entry in &node.entries {
            if entry.mbr().intersects(query) {
                match entry {
                    Entry::Data { data, .. } => {
                        // 根据 Geometry 进行精确比较
                        if let Some(entry_geometry) = self.geometry_map.get(data) {
                            if entry_geometry.intersects(geometry) {
                                // S2: 添加数据到结果
                                results.push(*data);
                            }
                        }
                    }
                    Entry::Node { node, .. } => {
                        // 递归搜索子节点
                        self.search_recursive(node, query, geometry, results);
                    }
                }
            }
        }
    }

    /// 递归搜索 - 仅边界框过滤（用于测试）
    fn search_recursive_bbox_only(&self, node: &Node, query: &Rectangle, results: &mut Vec<i32>) {
        for entry in &node.entries {
            if entry.mbr().intersects(query) {
                match entry {
                    Entry::Data { data, .. } => {
                        results.push(*data);
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
        
        rtree.insert_geometry(1, point1.clone());
        rtree.insert_geometry(2, point2.clone());
        rtree.insert_geometry(3, point3.clone());
        
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
        
        // 计算查询几何体的边界框
        let query_bbox = Rectangle::new(0.0, 0.0, 15.0, 15.0);
        
        // 搜索相交的几何体
        let results = rtree.search(&query_bbox, &query_polygon);
        
        // 应该找到数据 1 和 2（在查询多边形内），但不包括 3
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        assert!(!results.contains(&3));
        assert_eq!(results.len(), 2);
    }

    #[test]
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
        
        rtree.insert_geometry(1, poly1);
        rtree.insert_geometry(2, poly2);
        rtree.insert_geometry(3, poly3);
        
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
        
        let query_bbox = Rectangle::new(8.0, 8.0, 12.0, 12.0);
        let results = rtree.search(&query_bbox, &query_poly);
        
        // 应该找到poly1和poly2（相交），但不包括poly3（不相交）
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        assert!(!results.contains(&3));
        assert_eq!(results.len(), 2);
    }
}
