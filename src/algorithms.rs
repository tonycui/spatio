use crate::rectangle::Rectangle;
use crate::node::{Node, Entry};
use crate::rtree::RTree;

/// R-tree算法实现
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
        let leaf_node = self.get_node_mut(&leaf_path);
        leaf_node.add_entry(Entry::Data { mbr: rect, data });
        
        // I4: 检查是否需要分裂并调整树
        if leaf_node.entries.len() > max_entries {
            self.handle_overflow(leaf_path);
        } else {
            // 只需要更新MBR
            self.adjust_tree_upward(leaf_path);
        }
    }

    /// 搜索与查询矩形相交的所有条目 - 遵循论文Algorithm Search
    pub fn search(&self, query: &Rectangle) -> Vec<i32> {
        let mut results = Vec::new();
        
        if let Some(root) = self.root_ref() {
            self.search_recursive(root, query, &mut results);
        }
        
        results
    }

    /// 删除指定的数据条目
    pub fn delete(&mut self, _rect: &Rectangle, _data: i32) -> bool {
        // 后续实现
        todo!("Delete algorithm implementation")
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
            } else {
                panic!("Expected child entry");
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

    /// 递归搜索 - 遵循论文Search算法
    fn search_recursive(&self, node: &Node, query: &Rectangle, results: &mut Vec<i32>) {
        // S1: 搜索子树
        for entry in &node.entries {
            if entry.mbr().intersects(query) {
                match entry {
                    Entry::Data { data, .. } => {
                        // S2: 搜索叶子节点
                        results.push(*data);
                    }
                    Entry::Node { node, .. } => {
                        // 递归搜索子节点
                        self.search_recursive(node, query, results);
                    }
                }
            }
        }
    }

    /// 处理节点溢出 - 分裂节点
    fn handle_overflow(&mut self, path: Vec<usize>) {
        let max_entries = self.max_entries_internal();
        
        // 简单的处理：如果根节点溢出，创建新根
        if path.is_empty() {
            // 根节点溢出
            let old_root = self.root_mut().take().unwrap();
            let split_node = Self::simple_split(*old_root);
            
            let mut new_root = Node::new_index_node(split_node.0.level + 1);
            new_root.add_entry(Entry::Node {
                mbr: split_node.0.mbr.clone(),
                node: split_node.0,
            });
            new_root.add_entry(Entry::Node {
                mbr: split_node.1.mbr.clone(),
                node: split_node.1,
            });
            
            *self.root_mut() = Some(Box::new(new_root));
        } else {
            // 叶子节点溢出 - 暂时使用简单分裂
            let node = self.get_node_mut(&path);
            if node.entries.len() > max_entries {
                // 简单分裂：取一半条目
                let mid = node.entries.len() / 2;
                let split_entries = node.entries.split_off(mid);
                
                // 更新原节点
                node.update_mbr();
                
                // 创建新节点
                let mut new_node = Node::new(node.node_type.clone(), node.level);
                new_node.entries = split_entries;
                new_node.update_mbr();
                
                // 这里需要更复杂的处理来将新节点添加到父节点
                // 暂时简化处理
            }
        }
        
        // 更新路径上所有节点的MBR
        self.adjust_tree_upward(path);
    }

    /// 简单分裂节点
    fn simple_split(mut node: Node) -> (Box<Node>, Box<Node>) {
        let mid = node.entries.len() / 2;
        let split_entries = node.entries.split_off(mid);
        
        node.update_mbr();
        
        let mut new_node = Node::new(node.node_type.clone(), node.level);
        new_node.entries = split_entries;
        new_node.update_mbr();
        
        (Box::new(node), Box::new(new_node))
    }

    /// 向上调整树 - 更新MBR
    fn adjust_tree_upward(&mut self, path: Vec<usize>) {
        let mut current_path = path;
        
        while !current_path.is_empty() {
            let node = self.get_node_mut(&current_path);
            node.update_mbr();
            current_path.pop();
        }
        
        // 更新根节点
        if let Some(root) = self.root_mut() {
            root.update_mbr();
        }
    }

    /// 根据路径获取节点的可变引用
    fn get_node_mut(&mut self, path: &[usize]) -> &mut Node {
        let mut current = self.root_mut().as_mut().unwrap();
        
        for &index in path {
            if let Some(Entry::Node { node, .. }) = current.entries.get_mut(index) {
                current = node;
            } else {
                panic!("Invalid path");
            }
        }
        
        current
    }
}

/// 节点分裂算法 - 后续实现完整的二次分裂
impl RTree {
    /// 分裂节点（简单版本）
    fn split_node(&self, _node: &mut Node) -> Box<Node> {
        // 后续实现完整的二次分裂算法
        todo!("Full quadratic split algorithm implementation")
    }

    /// 选择种子算法
    fn pick_seeds(&self, _entries: &[Entry]) -> (usize, usize) {
        // 后续实现
        todo!("Pick seeds algorithm implementation")
    }

    /// 选择下一个条目分配算法
    fn pick_next(&self, _remaining: &[Entry], _group1: &[Entry], _group2: &[Entry]) -> (usize, usize, Entry) {
        // 后续实现
        todo!("Pick next algorithm implementation")
    }
}

/// 树结构调整算法
impl RTree {
    /// 计算扩大成本
    fn enlargement_cost(&self, mbr: &Rectangle, rect: &Rectangle) -> f64 {
        mbr.enlargement(rect)
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
}
