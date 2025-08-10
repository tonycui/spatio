use crate::node::{Node, Entry};
use crate::rtree::RTree;

/// R-tree调试功能实现
impl RTree {
    /// 打印完整的树结构用于调试
    /// 
    /// 这个函数会递归遍历整个树结构，打印每个节点的详细信息，
    /// 包括节点类型、层级、MBR边界、条目数量等，用于调试和可视化
    #[allow(dead_code)]
    pub fn print_tree_structure_debug(&self) {
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
        
        if let Some(root) = self.root_ref() {
            println!("=== R-tree Structure Debug ===");
            print_node_recursive(root, 0, String::new());
            println!("=== End Debug ===");
        } else {
            println!("=== R-tree Structure Debug ===");
            println!("Empty tree (no root)");
            println!("=== End Debug ===");
        }
    }
}

/// 独立的调试工具函数
/// 
/// 这些函数可以被测试代码使用，用于打印和分析R-tree结构
pub fn print_tree_structure(rtree: &RTree, depth: usize) {
    fn print_node(node: &Node, depth: usize) {
        let indent = "  ".repeat(depth);
        println!("{}Node (level={}, type={:?}, entries={})", 
            indent, node.level, node.node_type, node.entries.len());
        
        for (i, entry) in node.entries.iter().enumerate() {
            match entry {
                Entry::Data { mbr: _, data } => {
                    println!("{}  [{}] Data: {}", indent, i, data);
                }
                Entry::Node { mbr: _, node: child_node } => {
                    println!("{}  [{}] -> Child Node:", indent, i);
                    print_node(child_node, depth + 1);
                }
            }
        }
    }
    
    if let Some(root) = rtree.root_ref() {
        println!("Tree structure (max depth: {}):", depth);
        print_node(root, 0);
    } else {
        println!("Empty tree");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rectangle::Rectangle;

    #[test]
    fn test_debug_functions() {
        let mut rtree = RTree::new(4);
        
        // 测试空树的调试输出
        rtree.print_tree_structure_debug();
        print_tree_structure(&rtree, 3);
        
        // 插入一些数据
        rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), "1".to_string());
        rtree.insert(Rectangle::new(2.0, 2.0, 3.0, 3.0), "2".to_string());

        // 测试有数据的树的调试输出
        rtree.print_tree_structure_debug();
        print_tree_structure(&rtree, 3);
        
        // 这个测试主要确保调试函数不会崩溃
        assert!(!rtree.is_empty());
    }
}
