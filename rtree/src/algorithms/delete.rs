use crate::rectangle::Rectangle;
use crate::node::{Node, Entry};
use crate::rtree::RTree;

/// R-tree删除算法实现
impl RTree {
    /// 删除指定的数据条目 - 使用简化的下溢处理策略
    pub fn delete(&mut self, rect: &Rectangle, data: i32) -> bool {
        
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
    pub(crate) fn find_leaf_path(&self, rect: &Rectangle, data: i32) -> Option<Vec<usize>> {
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
    fn find_leaf_recursive(&self, node: &Node, rect: &Rectangle, data: i32, path: &mut Vec<usize>) -> bool {
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
                    entries.push((mbr.clone(), *data));
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
    use super::*;

    #[test]
    fn test_delete() {
        let mut rtree = RTree::new(4);
        
        // 插入数据
        rtree.insert(Rectangle::new(0.0, 0.0, 10.0, 10.0), 1);
        rtree.insert(Rectangle::new(5.0, 5.0, 15.0, 15.0), 2);
        rtree.insert(Rectangle::new(20.0, 20.0, 30.0, 30.0), 3);
        
        // 删除一个条目
        let deleted = rtree.delete(&Rectangle::new(5.0, 5.0, 15.0, 15.0), 2);
        assert!(deleted);
        
        // 尝试删除不存在的条目
        let deleted_again = rtree.delete(&Rectangle::new(5.0, 5.0, 15.0, 15.0), 2);
        assert!(!deleted_again);
        
        // 验证树结构 - 对于只有3个条目的小树，根节点可能仍然是叶子节点
        let root = rtree.root_ref().as_ref().unwrap();
        assert_eq!(root.entries.len(), 2);
        
        // 验证剩余条目仍然存在
        let results1 = rtree.search(&Rectangle::new(0.0, 0.0, 10.0, 10.0));
        assert!(results1.contains(&1));
        
        let results3 = rtree.search(&Rectangle::new(20.0, 20.0, 30.0, 30.0));
        assert!(results3.contains(&3));
        
        // 验证删除的条目不存在
        let results2 = rtree.search(&Rectangle::new(5.0, 5.0, 15.0, 15.0));
        assert!(!results2.contains(&2));
    }
    
    #[test]
    fn test_delete_operations() {
        let mut rtree = RTree::new(4);
        
        // 插入一些数据
        rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1);
        rtree.insert(Rectangle::new(2.0, 2.0, 3.0, 3.0), 2);
        rtree.insert(Rectangle::new(4.0, 4.0, 5.0, 5.0), 3);
        rtree.insert(Rectangle::new(6.0, 6.0, 7.0, 7.0), 4);
        
        // 验证初始状态
        assert_eq!(rtree.len(), 4);
        
        // 删除一个存在的条目
        assert!(rtree.delete(&Rectangle::new(2.0, 2.0, 3.0, 3.0), 2));
        assert_eq!(rtree.len(), 3);
        
        // 验证删除后搜索不到该条目
        let results = rtree.search(&Rectangle::new(2.0, 2.0, 3.0, 3.0));
        assert!(!results.contains(&2));
        
        // 验证其他条目仍然存在
        let all_results = rtree.search(&Rectangle::new(0.0, 0.0, 10.0, 10.0));
        assert!(all_results.contains(&1));
        assert!(all_results.contains(&3));
        assert!(all_results.contains(&4));
        assert!(!all_results.contains(&2));
        
        // 尝试删除不存在的条目
        assert!(!rtree.delete(&Rectangle::new(10.0, 10.0, 11.0, 11.0), 5));
        assert_eq!(rtree.len(), 3);
        
        // 删除所有剩余条目
        assert!(rtree.delete(&Rectangle::new(0.0, 0.0, 1.0, 1.0), 1));
        assert!(rtree.delete(&Rectangle::new(4.0, 4.0, 5.0, 5.0), 3));
        assert!(rtree.delete(&Rectangle::new(6.0, 6.0, 7.0, 7.0), 4));
        
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
            rtree.insert(Rectangle::new(x, 0.0, x + 1.0, 1.0), i);
        }
        
        println!("Initial tree structure:");
        print_tree_structure(&rtree, 0);
        
        // 验证所有条目都在
        for i in 0..10 {
            let x = (i as f64) * 2.0;
            let results = rtree.search(&Rectangle::new(x - 0.1, -0.1, x + 1.1, 1.1));
            println!("Before deletion - Entry {}: found = {}", i, results.contains(&i));
        }
        
        // 删除前5个条目
        for i in 0..5 {
            let x = (i as f64) * 2.0;
            println!("\nDeleting entry {}", i);
            let deleted = rtree.delete(&Rectangle::new(x, 0.0, x + 1.0, 1.0), i);
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
            let x = (i as f64) * 2.0;
            let results = rtree.search(&Rectangle::new(x - 0.1, -0.1, x + 1.1, 1.1));
            println!("After deletion - Entry {}: found = {}", i, results.contains(&i));
        }
    }
    
    #[test]
    fn test_delete_with_underflow() {
        let mut rtree = RTree::new(4); // min_entries = 2, max_entries = 4
        
        // 插入足够多的数据以创建有意义的树结构
        let data_points = vec![
            (Rectangle::new(0.0, 0.0, 1.0, 1.0), 1),
            (Rectangle::new(1.0, 0.0, 2.0, 1.0), 2),
            (Rectangle::new(2.0, 0.0, 3.0, 1.0), 3),
            (Rectangle::new(10.0, 0.0, 11.0, 1.0), 4),
            (Rectangle::new(11.0, 0.0, 12.0, 1.0), 5),
        ];
        
        for (rect, data) in &data_points {
            rtree.insert(rect.clone(), *data);
        }
        
        let initial_len = rtree.len();
        
        // 删除一些条目，可能触发下溢处理
        assert!(rtree.delete(&Rectangle::new(1.0, 0.0, 2.0, 1.0), 2));
        assert!(rtree.delete(&Rectangle::new(2.0, 0.0, 3.0, 1.0), 3));
        
        // 验证删除后的树状态
        assert_eq!(rtree.len(), initial_len - 2);
        
        // 验证剩余条目仍然可以找到
        let remaining_data = vec![1, 4, 5];
        for &data in &remaining_data {
            let found = data_points.iter()
                .find(|(_, d)| *d == data)
                .map(|(rect, _)| rtree.search(rect).contains(&data))
                .unwrap_or(false);
            assert!(found, "Entry {} should still be findable after deletions", data);
        }
        
        // 验证删除的条目不存在
        let deleted_results_2 = rtree.search(&Rectangle::new(1.0, 0.0, 2.0, 1.0));
        let deleted_results_3 = rtree.search(&Rectangle::new(2.0, 0.0, 3.0, 1.0));
        assert!(!deleted_results_2.contains(&2));
        assert!(!deleted_results_3.contains(&3));
    }
    
    #[test]
    fn test_simplified_underflow_handling() {
        let mut rtree = RTree::new(3); // min_entries = 1, max_entries = 3
        
        // 创建一个简单的测试场景验证简化的下溢处理
        let data_points = vec![
            (Rectangle::new(0.0, 0.0, 1.0, 1.0), 1),
            (Rectangle::new(0.5, 0.5, 1.5, 1.5), 2),  // 与1重叠
            (Rectangle::new(10.0, 0.0, 11.0, 1.0), 10),
            (Rectangle::new(10.5, 0.5, 11.5, 1.5), 11), // 与10重叠
        ];
        
        for (rect, data) in &data_points {
            rtree.insert(rect.clone(), *data);
        }
        
        // 验证插入后所有条目都存在
        for (rect, data) in &data_points {
            let results = rtree.search(rect);
            assert!(results.contains(data));
        }
        
        // 删除一个条目可能导致叶子节点下溢
        let deleted = rtree.delete(&Rectangle::new(0.5, 0.5, 1.5, 1.5), 2);
        assert!(deleted);
        
        // 验证重新插入的正确性：剩余条目应该仍然能找到
        for (rect, data) in &data_points {
            if *data == 2 {
                // 被删除的条目应该找不到
                let results = rtree.search(rect);
                assert!(!results.contains(data));
            } else {
                // 其他条目应该仍然能找到（即使可能被重新插入了）
                let results = rtree.search(rect);
                assert!(results.contains(data), "Entry {} should still be found after underflow handling", data);
            }
        }
        
        assert_eq!(rtree.len(), 3);
    }
    
    #[test]
    fn test_reinsert_correctness() {
        let mut rtree = RTree::new(3); // min_entries = 1, max_entries = 3
        
        // 创建一个特定的树结构，测试重新插入的正确性
        let original_data = vec![
            (Rectangle::new(0.0, 0.0, 1.0, 1.0), 1),
            (Rectangle::new(0.5, 0.5, 1.5, 1.5), 2),  // 与1重叠
            (Rectangle::new(10.0, 0.0, 11.0, 1.0), 10),
            (Rectangle::new(10.5, 0.5, 11.5, 1.5), 11), // 与10重叠
            (Rectangle::new(20.0, 0.0, 21.0, 1.0), 20),
            (Rectangle::new(20.5, 0.5, 21.5, 1.5), 21), // 与20重叠
        ];
        
        for (rect, data) in &original_data {
            rtree.insert(rect.clone(), *data);
        }
        
        // 记录插入前每个条目的搜索结果
        for (rect, data) in &original_data {
            let results = rtree.search(rect);
            assert!(results.contains(data));
        }
        
        // 删除一个可能导致节点重组的条目
        let deleted = rtree.delete(&Rectangle::new(0.5, 0.5, 1.5, 1.5), 2);
        assert!(deleted);
        
        // 验证重新插入的正确性
        for (rect, data) in &original_data {
            if *data == 2 {
                // 被删除的条目应该找不到
                let results = rtree.search(rect);
                assert!(!results.contains(data), "Deleted entry {} should not be found", data);
            } else {
                // 其他条目应该仍然能找到
                let results = rtree.search(rect);
                assert!(results.contains(data), "Entry {} should still be found after underflow handling", data);
            }
        }
        
        // 额外验证：使用扩大的搜索区域确保没有条目丢失
        let wide_search = rtree.search(&Rectangle::new(-1.0, -1.0, 30.0, 3.0));
        let expected_remaining = vec![1, 10, 11, 20, 21];
        for &expected in &expected_remaining {
            assert!(wide_search.contains(&expected), "Entry {} should be in wide search results", expected);
        }
        assert!(!wide_search.contains(&2), "Deleted entry 2 should not be in wide search results");
        
        assert_eq!(rtree.len(), 5);
    }
    
    #[test]
    fn test_mbr_update_after_deletion() {
        let mut rtree = RTree::new(3);
        
        // 构建一个简单的树，测试删除后的MBR更新
        rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1);
        rtree.insert(Rectangle::new(1.0, 0.0, 2.0, 1.0), 2);
        rtree.insert(Rectangle::new(2.0, 0.0, 3.0, 1.0), 3);
        rtree.insert(Rectangle::new(10.0, 10.0, 11.0, 11.0), 4); // 远离的点
        
        // 删除一个条目，验证简化的下溢处理正确工作
        let deleted = rtree.delete(&Rectangle::new(1.0, 0.0, 2.0, 1.0), 2);
        assert!(deleted);
        
        // 验证删除后树的完整性
        assert_eq!(rtree.len(), 3);
        
        // 验证剩余条目仍然可以找到
        let results1 = rtree.search(&Rectangle::new(0.0, 0.0, 1.0, 1.0));
        let results3 = rtree.search(&Rectangle::new(2.0, 0.0, 3.0, 1.0));
        let results4 = rtree.search(&Rectangle::new(10.0, 10.0, 11.0, 11.0));
        
        assert!(results1.contains(&1), "Entry 1 should still exist");
        assert!(results3.contains(&3), "Entry 3 should still exist");
        assert!(results4.contains(&4), "Entry 4 should still exist");
        
        // 验证删除的条目不存在
        let results2 = rtree.search(&Rectangle::new(1.0, 0.0, 2.0, 1.0));
        assert!(!results2.contains(&2), "Entry 2 should not exist");
        
        // 验证树结构仍然有效（能搜索到所有剩余条目）
        let all_results = rtree.search(&Rectangle::new(-1.0, -1.0, 20.0, 20.0));
        assert_eq!(all_results.len(), 3);
        assert!(all_results.contains(&1));
        assert!(all_results.contains(&3));
        assert!(all_results.contains(&4));
        assert!(!all_results.contains(&2));
    }
    
    #[test]
    fn test_edge_cases() {
        // 测试边界情况：删除导致根节点下溢等情况
        let mut rtree = RTree::new(3);
        
        // 只插入少量数据
        rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1);
        rtree.insert(Rectangle::new(2.0, 0.0, 3.0, 1.0), 2);
        
        // 删除一个条目
        let deleted = rtree.delete(&Rectangle::new(0.0, 0.0, 1.0, 1.0), 1);
        assert!(deleted);
        
        // 验证树仍然有效
        assert_eq!(rtree.len(), 1);
        let results = rtree.search(&Rectangle::new(2.0, 0.0, 3.0, 1.0));
        assert!(results.contains(&2));
        
        // 删除最后一个条目
        let deleted_last = rtree.delete(&Rectangle::new(2.0, 0.0, 3.0, 1.0), 2);
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
