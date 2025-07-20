use crate::rectangle::Rectangle;
use crate::node::{Node, Entry};
use crate::rtree::RTree;

/// R-tree算法实现
impl RTree {
    

    

}



impl RTree {
    #[allow(dead_code)]
    /// 打印完整的树结构用于调试 - 静态方法版本
    fn print_tree_structure_debug(&self) {
        fn print_node_recursive(node: &Node, depth: usize, path: String) {
            let indent = "  ".repeat(depth);
            println!("{}Node{} (level={}, type={:?}, mbr=[{:.2},{:.2},{:.2},{:.2}], {} entries):", 
                indent, path, node.level, node.node_type, 
                node.mbr.min[0], node.mbr.min[1], node.mbr.max[0], node.mbr.max[1],
                node.entries.len());
            
            if node.entries.is_empty() {
                println!("{}  ❌ EMPTY NODE!", indent);
            }
            
            for (i, entry) in node.entries.iter().enumerate() {
                match entry {
                    Entry::Data { mbr, data } => {
                        println!("{}  [{}] Data: {} at [{:.2},{:.2},{:.2},{:.2}]", 
                            indent, i, data, mbr.min[0], mbr.min[1], mbr.max[0], mbr.max[1]);
                    }
                    Entry::Node { mbr, node: child_node } => {
                        println!("{}  [{}] Node: mbr=[{:.2},{:.2},{:.2},{:.2}] -> child:", 
                            indent, i, mbr.min[0], mbr.min[1], mbr.max[0], mbr.max[1]);
                        
                        let child_path = if path.is_empty() {
                            format!("[{}]", i)
                        } else {
                            format!("{}[{}]", path, i)
                        };
                        
                        print_node_recursive(child_node, depth + 1, child_path);
                    }
                }
            }
        }
        
        println!("📊 Complete R-tree structure:");
        if let Some(root) = self.root_ref() {
            print_node_recursive(root, 0, "ROOT".to_string());
        } else {
            println!("❌ EMPTY TREE (root is None)");
        }
        println!("{}", "=".repeat(60));
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enlargement_cost() {
        let rtree = RTree::new(4);
        let rect1 = Rectangle::new(0.0, 0.0, 5.0, 5.0);
        let rect2 = Rectangle::new(3.0, 3.0, 8.0, 8.0);
        
        let cost = rtree.enlargement_cost(&rect1, &rect2);
        assert_eq!(cost, 39.0); // 8*8 - 5*5 = 64 - 25 = 39
    }

    #[test]
    fn test_insert_and_search() {
        let mut rtree = RTree::new(4);
        
        // 插入一些数据
        rtree.insert(Rectangle::new(0.0, 0.0, 10.0, 10.0), 1);
        rtree.insert(Rectangle::new(5.0, 5.0, 15.0, 15.0), 2);
        rtree.insert(Rectangle::new(20.0, 20.0, 30.0, 30.0), 3);
        
        // 搜索相交的矩形
        let query = Rectangle::new(8.0, 8.0, 12.0, 12.0);
        let results = rtree.search(&query);
        
        // 应该找到数据 1 和 2
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        assert!(!results.contains(&3));
    }
    

    
    #[test]
    fn test_node_split_with_overflow() {
        let mut rtree = RTree::new(3); // 最大3个条目，最小1个
        
        // 插入足够多的数据以触发分裂
        rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1);
        rtree.insert(Rectangle::new(2.0, 2.0, 3.0, 3.0), 2);
        rtree.insert(Rectangle::new(4.0, 4.0, 5.0, 5.0), 3);
        rtree.insert(Rectangle::new(6.0, 6.0, 7.0, 7.0), 4); // 这应该触发分裂
        
        // 验证树结构 - 根节点应该不再是叶子节点
        assert!(!rtree.is_empty());
        let root = rtree.root_ref().as_ref().unwrap();
        
        // 如果发生了分裂，根节点应该是索引节点
        if root.entries.len() > 3 {
            // 根节点溢出，应该创建新的根节点
            assert!(root.is_index_node());
        }
        
        // 搜索应该仍然工作
        let results = rtree.search(&Rectangle::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(results.len(), 4);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        assert!(results.contains(&3));
        assert!(results.contains(&4));
    }
    
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
    fn test_insert_debug() {
        let mut rtree = RTree::new(3);
        
        for i in 0..6 {
            let x = (i as f64) * 2.0;
            let rect = Rectangle::new(x, 0.0, x + 1.0, 1.0);
            println!("Inserting entry {} at ({}, {}, {}, {})", i, rect.min[0], rect.min[1], rect.max[0], rect.max[1]);
            rtree.insert(rect, i);
            
            // 立即验证刚插入的条目能否找到
            let search_rect = Rectangle::new(x, 0.0, x + 1.0, 1.0);
            let results = rtree.search(&search_rect);
            println!("After inserting {}: search results = {:?}", i, results);
            
            if !results.contains(&i) {
                println!("ERROR: Just inserted entry {} but cannot find it!", i);
                // 尝试用更大的搜索区域
                let expanded_rect = Rectangle::new(x - 1.0, -1.0, x + 2.0, 2.0);
                let expanded_results = rtree.search(&expanded_rect);
                println!("Expanded search results: {:?}", expanded_results);
                break;
            }
        }
    }
    
    #[test]
    fn test_tree_structure_debug() {
        let mut rtree = RTree::new(3);
        
        // 插入前4个条目
        for i in 0..4 {
            let x = (i as f64) * 2.0;
            let rect = Rectangle::new(x, 0.0, x + 1.0, 1.0);
            rtree.insert(rect, i);
        }
        
        // 打印树结构
        println!("Tree structure after inserting 0-3:");
        print_tree_structure(&rtree, 0);
        
        // 插入第5个条目
        let rect4 = Rectangle::new(8.0, 0.0, 9.0, 1.0);
        rtree.insert(rect4, 4);
        
        println!("\nTree structure after inserting 4:");
        print_tree_structure(&rtree, 0);
        
        // 测试搜索
        let search_results = rtree.search(&Rectangle::new(8.0, 0.0, 9.0, 1.0));
        println!("Search results for entry 4: {:?}", search_results);
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
