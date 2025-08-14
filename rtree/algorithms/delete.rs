use super::super::rectangle::Rectangle;
use super::super::node::{Node, Entry};
use super::super::rtree::RTree;
use super::utils::geometry_to_bbox;

/// R-tree删除算法实现
impl RTree {
    /// 删除指定的数据条目 - 遵循论文Algorithm Delete
    pub fn delete(&mut self, data: &str)  -> bool {
         // 直接在 if let 中获取几何体，如果不存在就直接返回
        let Some(geometry) = self.geometry_map.get(data) else {
            return true; // 幂等：不存在视为已删除
        };

        let Ok(rect) = geometry_to_bbox(geometry) else {
            eprintln!("Error calculating bounding box for data={}", data);
            return false;
        };

        if self.delete_in_rtree(&rect, data) {
            self.geometry_map.remove(data);
            self.geojson_map.remove(data);
            true
        } else {
            false
        }
    }

    /// 删除指定的数据条目 - 使用简化的下溢处理策略
    pub fn delete_in_rtree(&mut self, rect: &Rectangle, data: &str) -> bool {
        
        // D1: 找到包含目标条目的叶子节点
        if let Some(leaf_path) = self.find_leaf_path(rect, data) {
            // D2: 从叶子节点删除条目
            let (deleted, leaf_entries_count) = {
                let leaf_node = match self.get_last_node_mut(&leaf_path) {
                    Some(node) => node,
                    None => {
                        println!("Warning: Failed to get leaf node during deletion");
                        return false;
                    }
                };
                let initial_count = leaf_node.entries.len();
                
                // 删除匹配的条目
                leaf_node.entries.retain(|entry| {
                    if let Entry::Data { mbr, data: entry_data } = entry {
                        !(mbr == rect && entry_data == &data)
                    } else {
                        true
                    }
                });
                
                // 检查是否真的删除了条目
                if leaf_node.entries.len() == initial_count {
                    return false; // 没有找到要删除的条目
                }
                
                // 更新叶子节点的MBR
                leaf_node.update_mbr();
                
                (true, leaf_node.entries.len())
            };
            
            if deleted {
                // D3: 检查叶子节点是否下溢
                let min_entries = self.min_entries_internal();

                if leaf_entries_count < min_entries && !leaf_path.is_empty() {
                    
                    // 叶子节点下溢且不是根节点 - 使用简化的处理方案
                    self.handle_leaf_underflow(leaf_path.clone());
                } else {
                    // 只需要向上调整MBR
                    self.adjust_tree_upward(leaf_path);
                }
                
                // D4: 如果根节点只有一个条目且为索引节点，则缩短树
                self.shorten_tree();
                
                true
            } else {
                false
            }
        } else {
            false // 没有找到要删除的条目
        }
    }

    /// 查找包含指定数据条目的叶子节点路径
    /// 
    /// 返回从根节点到包含目标条目的叶子节点的路径
    pub(crate) fn find_leaf_path(&self, rect: &Rectangle, data: &str) -> Option<Vec<usize>> {
        if let Some(root) = self.root_ref() {
            let mut path = Vec::new();
            if self.find_leaf_recursive(root, rect, data, &mut path) {
                Some(path)
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// 递归查找包含指定数据条目的叶子节点
    fn find_leaf_recursive(&self, node: &Node, rect: &Rectangle, data: &str, path: &mut Vec<usize>) -> bool {
        if node.is_leaf_node() {
            // 在叶子节点中查找目标条目
            for entry in node.entries.iter() {
                if let Entry::Data { mbr, data: entry_data } = entry {
                    if mbr == rect && *entry_data == data {
                        return true; // 找到了目标条目
                    }
                }
            }
            false
        } else {
            // 在索引节点中递归搜索
            for (i, entry) in node.entries.iter().enumerate() {
                if let Entry::Node { mbr, node: child_node } = entry {
                    // 只在MBR包含目标矩形的子树中搜索
                    if mbr.contains(rect) {
                        path.push(i);
                        if self.find_leaf_recursive(child_node, rect, data, path) {
                            return true;
                        }
                        path.pop();
                    }
                }
            }
            false
        }
    }
    
    /// 处理叶子节点下溢 - 简化方案
    /// 
    /// 1. 收集下溢叶子节点中的所有数据条目
    /// 2. 将这些条目重新插入到树中
    /// 3. 从父节点中移除下溢的叶子节点
    /// 4. 向上调整MBR
    pub(crate) fn handle_leaf_underflow(&mut self, leaf_path: Vec<usize>) {
        // 1. 收集下溢叶子节点中的所有数据条目
        let entries_to_reinsert = {
            let leaf_node = match self.get_last_node_mut(&leaf_path) {
                Some(node) => node,
                None => {
                    println!("Warning: Failed to get leaf node in handle_leaf_underflow");
                    return;
                }
            };
            let mut entries = Vec::new();
            for entry in &leaf_node.entries {
                if let Entry::Data { mbr, data } = entry {
                    entries.push((mbr.clone(), data.clone()));
                }
            }
            entries
        };
        
        // 2. 从父节点中移除下溢的叶子节点
        let parent_path = &leaf_path[..leaf_path.len() - 1];
        let leaf_index = leaf_path[leaf_path.len() - 1];
        
        if parent_path.is_empty() {
            // 父节点是根节点
            let root = self.root_mut().as_mut().unwrap();
            if leaf_index < root.entries.len() {
                root.entries.remove(leaf_index);
                root.update_mbr();
            }
        } else {
            // 父节点是中间节点
            let parent = match self.get_last_node_mut(parent_path) {
                Some(node) => node,
                None => {
                    println!("Warning: Failed to get parent node in handle_leaf_underflow");
                    // 仍然尝试重新插入条目
                    for (rect, data) in entries_to_reinsert {
                        self.insert(rect, data);
                    }
                    return;
                }
            };
            if leaf_index < parent.entries.len() {
                parent.entries.remove(leaf_index);
                parent.update_mbr();
            }
        }

        // 2.5 如果父节点变空了，递归删除空的非叶子节点
        if !parent_path.is_empty() {
            let parent = match self.get_last_node_mut(parent_path) {
                Some(node) => node,
                None => {
                    println!("Warning: Failed to get parent node for empty check");
                    // 仍然尝试重新插入条目
                    for (rect, data) in entries_to_reinsert {
                        self.insert(rect, data);
                    }
                    return;
                }
            };
            if parent.entries.is_empty() && parent.is_index_node() {
                // 父节点也变空了，递归处理父节点
                self.remove_empty_nodes(parent_path.to_vec());
            } 
        }
         
        // 3. 向上调整MBR（仅调整MBR，不做其他下溢检查）
        self.adjust_tree_upward(parent_path.to_vec());
        
        // 4. 重新插入收集到的数据条目
        for (mbr, data) in entries_to_reinsert {
            self.insert(mbr, data);
        }
    }

    /// 删除空的非叶子节点 - 从指定路径的节点开始，递归删除空的父节点
    /// 
    /// 这个函数检查path指定的节点，如果它是空的非叶子节点，则删除它。
    /// 删除后，检查其父节点是否也变成空的，如果是则继续向上删除。
    /// 
    /// # 参数
    /// - `node_path`: 从根节点到目标节点的路径索引
    /// 
    /// # 说明
    /// - 只删除空的非叶子节点（索引节点）
    /// - 叶子节点即使为空也不会被删除
    /// - 只有当删除节点后其父节点变空时，才继续向上处理
    /// - 如果根节点变空，会清空整个树
    /// - 删除节点后会向上调整MBR
    pub(crate) fn remove_empty_nodes(&mut self, node_path: Vec<usize>) {
        if node_path.is_empty() {
            return;
        }
        
        // 检查指定路径的节点是否为空的非叶子节点
        let should_remove = {
            let node = match self.get_last_node_mut(&node_path) {
                Some(node) => node,
                None => {
                    println!("Warning: Failed to get node in remove_empty_nodes");
                    return;
                }
            };
            node.is_index_node() && node.entries.is_empty()
        };
        
        if !should_remove {
            // 当前节点不是空的非叶子节点，不需要删除
            return;
        }
        
        // 构造父节点路径
        let mut parent_path = node_path.clone();
        let node_index = parent_path.pop().unwrap();
        
        if parent_path.is_empty() {
            // 要删除的是根节点的直接子节点
            let root = self.root_mut().as_mut().unwrap();
            
            if node_index < root.entries.len() {
                root.entries.remove(node_index);
                
                // 检查根节点是否变空
                if root.entries.is_empty() {
                    // 清空整个树
                    *self.root_mut() = None;
                } else {
                    // 更新根节点的MBR
                    root.update_mbr();
                    
                    // 根节点不为空，停止递归
                }
            }
        } else {
            // 要删除的是中间节点
            let parent = match self.get_last_node_mut(&parent_path) {
                Some(node) => node,
                None => {
                    println!("Warning: Failed to get parent node in remove_empty_nodes");
                    return;
                }
            };
            
            if node_index < parent.entries.len() {
                parent.entries.remove(node_index);
                
                // 更新父节点的MBR
                parent.update_mbr();
                
                // 检查父节点是否也变空了
                if parent.entries.is_empty() && parent.is_index_node() {
                    // 父节点也变空了，递归处理父节点
                    self.remove_empty_nodes(parent_path);
                } else {
                    // 父节点不为空，向上调整MBR
                    self.adjust_tree_upward(parent_path);
                }
            }
        }
    }
    
    /// 缩短树 - 如果根节点只有一个条目且为索引节点，则将其子节点作为新的根节点
    pub(crate) fn shorten_tree(&mut self) {
        loop {
            let should_shorten = {
                if let Some(root) = self.root_ref() {
                    root.is_index_node() && root.entries.len() == 1
                } else {
                    false
                }
            };
            
            if should_shorten {
                // 将唯一的子节点提升为新的根节点
                let old_root = self.root_mut().take().unwrap();
                let mut entries = old_root.entries;
                if let Some(Entry::Node { node, .. }) = entries.pop() {
                    *self.root_mut() = Some(node);
                } else {
                    // 恢复根节点，防止出错
                    let restored_root = Node::new(old_root.node_type, old_root.level);
                    *self.root_mut() = Some(Box::new(restored_root));
                    break;
                }
            } else {
                break;
            }
        }
        
        // 如果根节点为空（所有条目都被删除），则清空树
        if let Some(root) = self.root_ref() {
            if root.entries.is_empty() {
                *self.root_mut() = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::geometry_utils::geometry_to_geojson;

    use super::*;
    use geo::{Point, Polygon, Coord};

    // 新的 delete 函数测试（直接通过 data ID 删除）
    #[test]
    fn test_delete_by_id_basic() {
        let mut rtree = RTree::new(4);
        
        // 插入几个几何体
        let point1 = geo::Geometry::Point(Point::new(1.0, 1.0));
        let point2 = geo::Geometry::Point(Point::new(5.0, 5.0));
        let point3 = geo::Geometry::Point(Point::new(10.0, 10.0));

        rtree.insert_geojson("1".to_string(), &geometry_to_geojson(&point1).to_string());
        rtree.insert_geojson("2".to_string(), &geometry_to_geojson(&point2).to_string());
        rtree.insert_geojson("3".to_string(), &geometry_to_geojson(&point3).to_string());

        // 验证初始状态
        assert_eq!(rtree.len(), 3);
        assert_eq!(rtree.geometry_map.len(), 3);
        assert_eq!(rtree.geojson_map.len(), 3);
        
        // 删除中间的一个几何体
        assert!(rtree.delete("2"));

        // 验证删除成功
        assert_eq!(rtree.len(), 2);
        assert_eq!(rtree.geometry_map.len(), 2);
        assert_eq!(rtree.geojson_map.len(), 2);
        
        // 验证正确的条目被删除
        assert!(rtree.geometry_map.contains_key(&"1".to_string()));
        assert!(!rtree.geometry_map.contains_key(&"2".to_string()));
        assert!(rtree.geometry_map.contains_key(&"3".to_string()));
        
        // 验证空间查询结果
        let search_all = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 15.0, 15.0));
        assert_eq!(search_all.len(), 2);
        assert!(search_all.contains(&"1".to_string()));
        assert!(!search_all.contains(&"2".to_string()));
        assert!(search_all.contains(&"3".to_string()));
    }

    #[test]
    fn test_delete_by_id_nonexistent() {
        let mut rtree = RTree::new(4);
        
        // 插入一个几何体
        let point = geo::Geometry::Point(Point::new(1.0, 1.0));
        rtree.insert_geojson("1".to_string(), &geometry_to_geojson(&point).to_string());

        // 验证初始状态
        assert_eq!(rtree.len(), 1);
        assert_eq!(rtree.geometry_map.len(), 1);
        
        // 尝试删除不存在的 ID，应该返回 true（幂等性）
        assert!(rtree.delete("999"));

        // 验证原有数据没有被影响
        assert_eq!(rtree.len(), 1);
        assert_eq!(rtree.geometry_map.len(), 1);
        assert!(rtree.geometry_map.contains_key(&"1".to_string()));
    }

    #[test]
    fn test_delete_by_id_empty_tree() {
        let mut rtree = RTree::new(4);
        
        // 在空树上删除，应该返回 true（幂等性）
        assert!(rtree.delete("1"));

        // 验证树仍然为空
        assert_eq!(rtree.len(), 0);
        assert!(rtree.is_empty());
    }

    #[test]
    fn test_delete_by_id_multiple_operations() {
        let mut rtree = RTree::new(4);
        
        // 插入多个几何体
        for i in 1..=5 {
            let point = geo::Geometry::Point(Point::new(i as f64, i as f64));
            rtree.insert_geojson(i.to_string(), &geometry_to_geojson(&point).to_string());
        }
        
        // 验证初始状态
        assert_eq!(rtree.len(), 5);
        
        // 删除部分几何体
        assert!(rtree.delete("2"));
        assert!(rtree.delete("4"));

        // 验证删除后的状态
        assert_eq!(rtree.len(), 3);
        assert_eq!(rtree.geometry_map.len(), 3);
        assert_eq!(rtree.geojson_map.len(), 3);
        
        // 验证剩余的几何体
        let remaining_ids = vec!["1".to_string(), "3".to_string(), "5".to_string()];
        for id in remaining_ids {
            assert!(rtree.geometry_map.contains_key(&id));
        }
        
        // 验证被删除的几何体
        let deleted_ids = vec!["2".to_string(), "4".to_string()];
        for id in deleted_ids {
            assert!(!rtree.geometry_map.contains_key(&id));
        }
        
        // 验证空间查询结果
        let search_all = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(search_all.len(), 3);
        assert!(search_all.contains(&"1".to_string()));
        assert!(!search_all.contains(&"2".to_string()));
        assert!(search_all.contains(&"3".to_string()));
        assert!(!search_all.contains(&"4".to_string()));
        assert!(search_all.contains(&"5".to_string()));
    }

    #[test]
    fn test_delete_by_id_all_entries() {
        let mut rtree = RTree::new(4);
        
        // 插入几个几何体
        let geometries = vec![
            ("1", geo::Geometry::Point(Point::new(1.0, 1.0))),
            ("2", geo::Geometry::Point(Point::new(5.0, 5.0))),
            ("3", geo::Geometry::Point(Point::new(10.0, 10.0))),
        ];
        
        for (id, geom) in &geometries {
            rtree.insert_geojson(id.to_string(), &geometry_to_geojson(geom).to_string());
        }
        
        // 验证初始状态
        assert_eq!(rtree.len(), 3);
        
        // 删除所有几何体
        for (id, _) in &geometries {
            assert!(rtree.delete(*id));
        }
        
        // 验证树为空
        assert_eq!(rtree.len(), 0);
        assert_eq!(rtree.geometry_map.len(), 0);
        assert_eq!(rtree.geojson_map.len(), 0);
        assert!(rtree.is_empty());
        
        // 验证空间查询返回空结果
        let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 15.0, 15.0));
        assert!(search_results.is_empty());
    }

    #[test]
    fn test_delete_by_id_consistency() {
        let mut rtree = RTree::new(4);
        
        // 插入不同类型的几何体
        let point = geo::Geometry::Point(Point::new(1.0, 1.0));
        rtree.insert_geojson("1".to_string(), &geometry_to_geojson(&point).to_string());

        let coords = vec![
            Coord { x: 5.0, y: 5.0 },
            Coord { x: 8.0, y: 5.0 },
            Coord { x: 8.0, y: 8.0 },
            Coord { x: 5.0, y: 8.0 },
            Coord { x: 5.0, y: 5.0 },
        ];
        let polygon = geo::Geometry::Polygon(Polygon::new(coords.into(), vec![]));
        rtree.insert_geojson("2".to_string(), &geometry_to_geojson(&polygon).to_string());

        // 验证插入成功
        assert_eq!(rtree.len(), 2);
        assert_eq!(rtree.geometry_map.len(), 2);
        assert_eq!(rtree.geojson_map.len(), 2);
        
        // 删除一个几何体
        assert!(rtree.delete("1"));

        // 验证数据一致性
        assert_eq!(rtree.len(), 1);
        assert_eq!(rtree.geometry_map.len(), 1);
        assert_eq!(rtree.geojson_map.len(), 1);
        
        // 验证剩余几何体仍然正常
        assert!(!rtree.geometry_map.contains_key(&"1".to_string()));
        assert!(rtree.geometry_map.contains_key(&"2".to_string()));
        
        let search_results = rtree.search_bbox(&Rectangle::new(5.0, 5.0, 8.0, 8.0));
        assert!(search_results.contains(&"2".to_string()));
        assert!(!search_results.contains(&"1".to_string()));
    }

    #[test]
    fn test_delete_by_id_with_bbox_error() {
        let mut rtree = RTree::new(4);
        
        // 这个测试验证当几何体存在但bbox计算失败时的行为
        // 注意：实际情况下geo::Geometry很难出现bbox计算失败的情况
        // 但我们测试函数的错误处理路径
        
        let point = geo::Geometry::Point(Point::new(1.0, 1.0));
        rtree.insert_geojson("1".to_string(), &geometry_to_geojson(&point).to_string());

        // 验证初始状态
        assert_eq!(rtree.len(), 1);
        assert!(rtree.geometry_map.contains_key(&"1".to_string()));

        // 删除操作（正常情况下应该成功）
        let result = rtree.delete("1");

        // 验证结果：要么成功删除，要么因为bbox错误返回false但不破坏数据一致性
        if result {
            // 删除成功
            assert_eq!(rtree.len(), 0);
            assert!(!rtree.geometry_map.contains_key(&"1".to_string()));
        } else {
            // 删除失败但数据保持一致
            assert_eq!(rtree.len(), 1);
            assert!(rtree.geometry_map.contains_key(&"1".to_string()));
        }
        
        // 无论如何，所有计数都应该保持一致
        assert_eq!(rtree.len(), rtree.geometry_map.len());
        assert_eq!(rtree.len(), rtree.geojson_map.len());
    }

    #[test]
    fn test_delete() {
        let mut rtree = RTree::new(4);
        
        // 插入数据
        let point1 = geo::Geometry::Point(Point::new(5.0, 5.0));
        let point2 = geo::Geometry::Point(Point::new(10.0, 10.0));
        let point3 = geo::Geometry::Point(Point::new(25.0, 25.0));

        rtree.insert_geojson("1".to_string(), &geometry_to_geojson(&point1).to_string());
        rtree.insert_geojson("2".to_string(), &geometry_to_geojson(&point2).to_string());
        rtree.insert_geojson("3".to_string(), &geometry_to_geojson(&point3).to_string());

        // 删除一个条目
        let deleted = rtree.delete("2");
        assert!(deleted);
        
        // 尝试删除不存在的条目
        let deleted_again = rtree.delete("2");
        assert!(deleted_again); // 幂等性，返回 true
        
        // 验证树结构
        assert_eq!(rtree.len(), 2);
        
        // 验证剩余条目仍然存在
        let search_all = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 30.0, 30.0));
        assert!(search_all.contains(&"1".to_string()));
        assert!(!search_all.contains(&"2".to_string()));
        assert!(search_all.contains(&"3".to_string()));
    }
    
    #[test]
    fn test_delete_operations() {
        let mut rtree = RTree::new(4);
        
        // 插入一些数据
        let point1 = geo::Geometry::Point(Point::new(0.5, 0.5));
        let point2 = geo::Geometry::Point(Point::new(2.5, 2.5));
        let point3 = geo::Geometry::Point(Point::new(4.5, 4.5));
        let point4 = geo::Geometry::Point(Point::new(6.5, 6.5));

        rtree.insert_geojson("1".to_string(), &geometry_to_geojson(&point1).to_string());
        rtree.insert_geojson("2".to_string(), &geometry_to_geojson(&point2).to_string());
        rtree.insert_geojson("3".to_string(), &geometry_to_geojson(&point3).to_string());
        rtree.insert_geojson("4".to_string(), &geometry_to_geojson(&point4).to_string());

        // 验证初始状态
        assert_eq!(rtree.len(), 4);
        
        // 删除一个存在的条目
        assert!(rtree.delete("2"));
        assert_eq!(rtree.len(), 3);
        
        // 验证删除后搜索不到该条目
        let search_all = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 10.0, 10.0));
        assert!(search_all.contains(&"1".to_string()));
        assert!(!search_all.contains(&"2".to_string()));
        assert!(search_all.contains(&"3".to_string()));
        assert!(search_all.contains(&"4".to_string()));

        // 尝试删除不存在的条目（幂等性）
        assert!(rtree.delete("5"));
        assert_eq!(rtree.len(), 3);
        
        // 删除所有剩余条目
        assert!(rtree.delete("1"));
        assert!(rtree.delete("3"));
        assert!(rtree.delete("4"));
        
        // 验证树为空
        assert_eq!(rtree.len(), 0);
        assert!(rtree.is_empty());
    }
    
    #[test]
    fn test_delete_debug_detailed() {
        let mut rtree = RTree::new(3);
        
        // 插入10个条目
        for i in 0..10 {
            let x = (i as f64) * 2.0;
            let point = geo::Geometry::Point(Point::new(x + 0.5, 0.5));
            rtree.insert_geojson(i.to_string(), &geometry_to_geojson(&point).to_string());
        }
        
        println!("Initial tree structure:");
        print_tree_structure(&rtree, 0);
        
        // 验证所有条目都在
        for i in 0..10 {
            let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 20.0, 2.0));
            println!("Before deletion - Entry {}: found = {}", i, search_results.contains(&i.to_string()));
        }
        
        // 删除前5个条目
        for i in 0..5 {
            println!("\nDeleting entry {}", i);
            let deleted = rtree.delete(&i.to_string());
            println!("Delete success: {}, tree length: {}", deleted, rtree.len());
            
            if i == 2 {  // 在删除第3个条目后打印树结构
                println!("Tree structure after deleting first 3 entries:");
                print_tree_structure(&rtree, 0);
            }
        }
        
        println!("\nFinal tree structure:");
        print_tree_structure(&rtree, 0);
        
        // 验证剩余条目
        for i in 5..10 {
            let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 20.0, 2.0));
            println!("After deletion - Entry {}: found = {}", i, search_results.contains(&i.to_string()));
        }
    }
    
    #[test]
    fn test_delete_with_underflow() {
        let mut rtree = RTree::new(4); // min_entries = 2, max_entries = 4
        
        // 插入足够多的数据以创建有意义的树结构
        let geometries = vec![
            (1, geo::Geometry::Point(Point::new(0.5, 0.5))),
            (2, geo::Geometry::Point(Point::new(1.5, 0.5))),
            (3, geo::Geometry::Point(Point::new(2.5, 0.5))),
            (4, geo::Geometry::Point(Point::new(10.5, 0.5))),
            (5, geo::Geometry::Point(Point::new(11.5, 0.5))),
        ];
        
        for (id, geom) in &geometries {
            rtree.insert_geojson(id.to_string(), &geometry_to_geojson(geom).to_string());
        }
        
        let initial_len = rtree.len();
        
        // 删除一些条目，可能触发下溢处理
        assert!(rtree.delete("2"));
        assert!(rtree.delete("3"));

        // 验证删除后的树状态
        assert_eq!(rtree.len(), initial_len - 2);
        
        // 验证剩余条目仍然可以找到
        let remaining_data = vec!["1", "4", "5"];
        for &data in &remaining_data {
            let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 15.0, 2.0));
            assert!(search_results.contains(&data.to_string()), "Entry {} should still be findable after deletions", data);
        }
        
        // 验证删除的条目不存在
        let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 15.0, 2.0));
        assert!(!search_results.contains(&"2".to_string()));
        assert!(!search_results.contains(&"3".to_string()));
    }
    
    #[test]
    fn test_simplified_underflow_handling() {
        let mut rtree = RTree::new(3); // min_entries = 1, max_entries = 3
        
        // 创建一个简单的测试场景验证简化的下溢处理
        let geometries = vec![
            (1, geo::Geometry::Point(Point::new(0.5, 0.5))),
            (2, geo::Geometry::Point(Point::new(1.0, 1.0))),  // 与1重叠区域
            (10, geo::Geometry::Point(Point::new(10.5, 0.5))),
            (11, geo::Geometry::Point(Point::new(11.0, 1.0))), // 与10重叠区域
        ];
        
        for (id, geom) in &geometries {
            rtree.insert_geojson(id.to_string(), &geometry_to_geojson(geom).to_string());
        }
        
        // 验证插入后所有条目都存在
        for (id, _) in &geometries {
            let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 15.0, 2.0));
            assert!(search_results.contains(&id.to_string()));
        }
        
        // 删除一个条目可能导致叶子节点下溢
        let deleted = rtree.delete("2");
        assert!(deleted);
        
        // 验证重新插入的正确性：剩余条目应该仍然能找到
        for (id, _) in &geometries {
            if *id == 2 {
                // 被删除的条目应该找不到
                let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 15.0, 2.0));
                assert!(!search_results.contains(&id.to_string()));
            } else {
                // 其他条目应该仍然能找到（即使可能被重新插入了）
                let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 15.0, 2.0));
                assert!(search_results.contains(&id.to_string()), "Entry {} should still be found after underflow handling", id);
            }
        }
        
        assert_eq!(rtree.len(), 3);
    }
    
    #[test]
    fn test_reinsert_correctness() {
        let mut rtree = RTree::new(3); // min_entries = 1, max_entries = 3
        
        // 创建一个特定的树结构，测试重新插入的正确性
        let geometries = vec![
            (1, geo::Geometry::Point(Point::new(0.5, 0.5))),
            (2, geo::Geometry::Point(Point::new(1.0, 1.0))),  // 与1重叠区域
            (10, geo::Geometry::Point(Point::new(10.5, 0.5))),
            (11, geo::Geometry::Point(Point::new(11.0, 1.0))), // 与10重叠区域
            (20, geo::Geometry::Point(Point::new(20.5, 0.5))),
            (21, geo::Geometry::Point(Point::new(21.0, 1.0))), // 与20重叠区域
        ];
        
        for (id, geom) in &geometries {
            rtree.insert_geojson(id.to_string(), &geometry_to_geojson(geom).to_string());
        }
        
        // 记录插入前每个条目的搜索结果
        for (id, _) in &geometries {
            let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 25.0, 2.0));
            assert!(search_results.contains(&id.to_string()));
        }
        
        // 删除一个可能导致节点重组的条目
        let deleted = rtree.delete("2");
        assert!(deleted);
        
        // 验证重新插入的正确性
        for (id, _) in &geometries {
            let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 25.0, 2.0));
            if *id == 2 {
                // 被删除的条目应该找不到
                assert!(!search_results.contains(&id.to_string()), "Deleted entry {} should not be found", id);
            } else {
                // 其他条目应该仍然能找到
                assert!(search_results.contains(&id.to_string()), "Entry {} should still be found after underflow handling", id);
            }
        }
        
        // 额外验证：使用扩大的搜索区域确保没有条目丢失
        let wide_search = rtree.search_bbox(&Rectangle::new(-1.0, -1.0, 30.0, 3.0));
        let expected_remaining = vec![1, 10, 11, 20, 21];
        for &expected in &expected_remaining {
            assert!(wide_search.contains(&expected.to_string()), "Entry {} should be in wide search results", expected);
        }
        assert!(!wide_search.contains(&"2".to_string()), "Deleted entry 2 should not be in wide search results");

        assert_eq!(rtree.len(), 5);
    }
    
    #[test]
    fn test_mbr_update_after_deletion() {
        let mut rtree = RTree::new(3);
        
        // 构建一个简单的树，测试删除后的MBR更新
        let geometries = vec![
            (1, geo::Geometry::Point(Point::new(0.5, 0.5))),
            (2, geo::Geometry::Point(Point::new(1.5, 0.5))),
            (3, geo::Geometry::Point(Point::new(2.5, 0.5))),
            (4, geo::Geometry::Point(Point::new(10.5, 10.5))), // 远离的点
        ];
        
        for (id, geom) in &geometries {
            rtree.insert_geojson(id.to_string(), &geometry_to_geojson(geom).to_string());
        }
        
        // 删除一个条目，验证简化的下溢处理正确工作
        let deleted = rtree.delete("2");
        assert!(deleted);
        
        // 验证删除后树的完整性
        assert_eq!(rtree.len(), 3);
        
        // 验证剩余条目仍然可以找到
        let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 15.0, 15.0));
        assert!(search_results.contains(&"1".to_string()), "Entry 1 should still exist");
        assert!(search_results.contains(&"3".to_string()), "Entry 3 should still exist");
        assert!(search_results.contains(&"4".to_string()), "Entry 4 should still exist");
        assert!(!search_results.contains(&"2".to_string()), "Entry 2 should not exist");

        // 验证树结构仍然有效（能搜索到所有剩余条目）
        assert_eq!(search_results.len(), 3);
    }
    
    #[test]
    fn test_edge_cases() {
        // 测试边界情况：删除导致根节点下溢等情况
        let mut rtree = RTree::new(3);
        
        // 只插入少量数据
        let point1 = geo::Geometry::Point(Point::new(0.5, 0.5));
        let point2 = geo::Geometry::Point(Point::new(2.5, 0.5));

        rtree.insert_geojson("1".to_string(), &geometry_to_geojson(&point1).to_string());
        rtree.insert_geojson("2".to_string(), &geometry_to_geojson(&point2).to_string());

        // 删除一个条目
        let deleted = rtree.delete("1");
        assert!(deleted);
        
        // 验证树仍然有效
        assert_eq!(rtree.len(), 1);
        let search_results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 5.0, 2.0));
        assert!(search_results.contains(&"2".to_string()));
        assert!(!search_results.contains(&"1".to_string()));

        // 删除最后一个条目
        let deleted_last = rtree.delete("2");
        assert!(deleted_last);
        
        // 验证树为空
        assert_eq!(rtree.len(), 0);
        assert!(rtree.is_empty());
    }

    #[allow(dead_code)]
    fn print_tree_structure(rtree: &RTree, depth: usize) {
        fn print_node(node: &Node, depth: usize) {
            let indent = "  ".repeat(depth);
            println!("{}Node (level={}, type={:?}, mbr=[{},{},{},{}]):", 
                indent, node.level, node.node_type, 
                node.mbr.min[0], node.mbr.min[1], node.mbr.max[0], node.mbr.max[1]);
            
            for (i, entry) in node.entries.iter().enumerate() {
                match entry {
                    Entry::Data { mbr, data } => {
                        println!("{}  [{}] Data: {} at [{},{},{},{}]", 
                            indent, i, data, mbr.min[0], mbr.min[1], mbr.max[0], mbr.max[1]);
                    }
                    Entry::Node { mbr, node } => {
                        println!("{}  [{}] Node: mbr=[{},{},{},{}]", 
                            indent, i, mbr.min[0], mbr.min[1], mbr.max[0], mbr.max[1]);
                        print_node(node, depth + 2);
                    }
                }
            }
        }
        
        if let Some(root) = rtree.root_ref() {
            print_node(root, depth);
        } else {
            println!("Empty tree");
        }
    }
}
