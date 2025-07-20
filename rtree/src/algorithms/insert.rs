use crate::rectangle::Rectangle;
use crate::node::{Node, Entry};
use crate::rtree::RTree;

/// 插入操作相关算法
impl RTree {
    /// 插入新的数据条目 - 遵循论文Algorithm Insert
    pub fn insert(&mut self, rect: Rectangle, data: i32) {
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

    #[test]
    fn test_insert_basic() {
        let mut rtree = RTree::new(4);
        
        // 测试插入到空树
        assert!(rtree.is_empty());
        rtree.insert(Rectangle::new(0.0, 0.0, 10.0, 10.0), 1);
        assert_eq!(rtree.len(), 1);
        assert!(!rtree.is_empty());
        
        // 测试插入多个条目
        rtree.insert(Rectangle::new(5.0, 5.0, 15.0, 15.0), 2);
        rtree.insert(Rectangle::new(20.0, 20.0, 30.0, 30.0), 3);
        assert_eq!(rtree.len(), 3);
    }

    #[test]
    fn test_choose_leaf_path() {
        let mut rtree = RTree::new(3); // 小的max_entries以便测试分裂
        
        // 插入足够多的数据以创建多层树结构
        for i in 0..6 {
            let x = (i as f64) * 2.0;
            rtree.insert(Rectangle::new(x, 0.0, x + 1.0, 1.0), i);
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
            Entry::Data { mbr: Rectangle::new(0.0, 0.0, 5.0, 5.0), data: 1 },
            Entry::Data { mbr: Rectangle::new(10.0, 10.0, 15.0, 15.0), data: 2 },
            Entry::Data { mbr: Rectangle::new(20.0, 20.0, 25.0, 25.0), data: 3 },
        ];
        
        // 测试选择最合适的子树
        let test_rect = Rectangle::new(2.0, 2.0, 3.0, 3.0);
        let best_index = rtree.choose_subtree(&entries, &test_rect);
        
        // 应该选择第一个条目，因为它与测试矩形重叠
        assert_eq!(best_index, 0);
    }
}
