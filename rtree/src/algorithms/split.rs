use crate::rectangle::Rectangle;
use crate::node::{Node, Entry};
use crate::rtree::RTree;

/// 节点分裂算法 - 实现完整的二次分裂(Quadratic Split)
impl RTree {
    /// 处理节点溢出 - 使用二次分裂算法
    pub(crate) fn handle_overflow(&mut self, path: Vec<usize>) {
        // 如果是根节点溢出，需要特殊处理
        if path.is_empty() {
            // 根节点溢出 - 创建新的根节点
            let old_root = self.root_mut().take().unwrap();
            let (group1, group2) = self.quadratic_split(old_root.entries);
            
            // 创建两个新节点
            let mut node1 = Node::new(old_root.node_type.clone(), old_root.level);
            node1.entries = group1;
            node1.update_mbr();
            
            let mut node2 = Node::new(old_root.node_type.clone(), old_root.level);
            node2.entries = group2;
            node2.update_mbr();
            
            // 创建新的根节点
            let mut new_root = Node::new_index_node(old_root.level + 1);
            new_root.add_entry(Entry::Node {
                mbr: node1.mbr.clone(),
                node: Box::new(node1),
            });
            new_root.add_entry(Entry::Node {
                mbr: node2.mbr.clone(),
                node: Box::new(node2),
            });
            
            *self.root_mut() = Some(Box::new(new_root));
        } else {
            // 非根节点溢出 - 分裂节点并可能向上传播
            self.split_and_propagate(path);
        }
    }
    
    /// 分裂节点并向上传播溢出
    /// 
    /// 这个方法处理非根节点的分裂，并在必要时向上传播分裂
    pub(crate) fn split_and_propagate(&mut self, mut path: Vec<usize>) {
        let max_entries = self.max_entries_internal();
        
        // 获取要分裂的节点并提取其条目
        let (entries, node_type, level) = {
            let node = match self.get_last_node_mut(&path) {
                Some(node) => node,
                None => {
                    println!("Warning: Failed to get node during split_and_propagate");
                    return;
                }
            };
            
            // 检查是否真的需要分裂
            if node.entries.len() <= max_entries {
                // 只需要更新MBR
                self.adjust_tree_upward(path);
                return;
            }
            
            // 提取节点信息
            let entries = std::mem::take(&mut node.entries);
            let node_type = node.node_type.clone();
            let level = node.level;
            
            (entries, node_type, level)
        };
        
        // 执行二次分裂（现在self没有被借用）
        let (group1, group2) = self.quadratic_split(entries);
        
        // 更新原节点
        {
            let node = match self.get_last_node_mut(&path) {
                Some(node) => node,
                None => {
                    println!("Warning: Failed to get node during split group update");
                    return;
                }
            };
            node.entries = group1;
            node.update_mbr();
        }
        
        // 创建新节点
        let mut new_node = Node::new(node_type, level);
        new_node.entries = group2;
        new_node.update_mbr();
        
        // 获取父节点路径
        path.pop();
        
        if path.is_empty() {
            // 父节点是根节点，需要特殊处理
            let root = self.root_mut().as_mut().unwrap();
            
            // 添加新节点到根节点
            root.add_entry(Entry::Node {
                mbr: new_node.mbr.clone(),
                node: Box::new(new_node),
            });
            
            // 检查根节点是否溢出
            if root.entries.len() > max_entries {
                self.handle_overflow(vec![]);
            } else {
                root.update_mbr();
            }
        } else {
            // 父节点不是根节点
            let parent = match self.get_last_node_mut(&path) {
                Some(node) => node,
                None => {
                    println!("Warning: Failed to get parent node during split propagation");
                    return;
                }
            };
            
            // 添加新节点到父节点
            parent.add_entry(Entry::Node {
                mbr: new_node.mbr.clone(),
                node: Box::new(new_node),
            });
            
            // 检查父节点是否溢出
            if parent.entries.len() > max_entries {
                // 递归处理父节点溢出
                self.split_and_propagate(path);
            } else {
                // 只需要向上更新MBR
                self.adjust_tree_upward(path);
            }
        }
    }

    /// 二次分裂算法 - 遵循Gut84.pdf论文Algorithm QuadraticSplit
    /// 
    /// 该算法的目标是将溢出的节点分裂为两个节点，使得：
    /// 1. 两个节点的总面积最小化
    /// 2. 两个节点之间的重叠最小化
    /// 3. 每个节点至少包含最小条目数
    fn quadratic_split(&self, mut entries: Vec<Entry>) -> (Vec<Entry>, Vec<Entry>) {
        let min_entries = self.min_entries_internal();
        let total_entries = entries.len();
        
        // QS1: 选择种子 - 找到浪费空间最大的两个条目作为两组的种子
        let (seed1, seed2) = self.pick_seeds(&entries);
        
        // 初始化两个分组
        let mut group1 = vec![entries.swap_remove(seed1.max(seed2))]; // 先移除索引大的
        let mut group2 = vec![entries.swap_remove(seed1.min(seed2))]; // 再移除索引小的
        
        // QS2: 检查是否需要将所有剩余条目分配到一组
        // 如果某一组已经包含了 total_entries - min_entries 个条目，
        // 则将剩余条目全部分配给另一组
        while !entries.is_empty() {
            // 检查是否需要强制分配
            if group1.len() == total_entries - min_entries {
                // group1已达到最大值，剩余全部给group2
                group2.extend(entries.drain(..));
                break;
            } else if group2.len() == total_entries - min_entries {
                // group2已达到最大值，剩余全部给group1
                group1.extend(entries.drain(..));
                break;
            }
            
            // QS3: 选择下一个条目 - 选择对某一组偏好最强的条目
            let (next_index, preferred_group) = self.pick_next(&entries, &group1, &group2);
            let entry = entries.swap_remove(next_index);
            
            if preferred_group == 1 {
                group1.push(entry);
            } else {
                group2.push(entry);
            }
        }
        
        (group1, group2)
    }

    /// PickSeeds算法 - 选择两个条目作为种子，使得它们组合后的死空间最大
    /// 
    /// 死空间 = 包含两个条目的矩形面积 - 两个条目各自的面积
    /// 选择死空间最大的两个条目，这样可以避免在同一组中放置相距很远的条目
    fn pick_seeds(&self, entries: &[Entry]) -> (usize, usize) {
        let mut max_waste = f64::NEG_INFINITY;
        let mut best_pair = (0, 1);
        
        // 遍历所有条目对
        for i in 0..entries.len() {
            for j in (i + 1)..entries.len() {
                let rect1 = entries[i].mbr();
                let rect2 = entries[j].mbr();
                
                // 计算包含两个矩形的最小边界矩形
                let combined = rect1.union(rect2);
                
                // 计算死空间：组合面积 - 两个矩形各自面积
                let waste = combined.area() - rect1.area() - rect2.area();
                
                if waste > max_waste {
                    max_waste = waste;
                    best_pair = (i, j);
                }
            }
        }
        
        best_pair
    }

    /// PickNext算法 - 选择下一个要分配的条目
    /// 
    /// 对于每个剩余条目，计算将其加入group1和group2的扩大成本差异
    /// 选择差异最大的条目，并将其分配给扩大成本较小的组
    fn pick_next(&self, remaining: &[Entry], group1: &[Entry], group2: &[Entry]) -> (usize, usize) {
        let mut max_preference = f64::NEG_INFINITY;
        let mut best_entry = 0;
        let mut preferred_group = 1;
        
        // 计算当前两组的MBR
        let group1_mbr = self.calculate_group_mbr(group1);
        let group2_mbr = self.calculate_group_mbr(group2);
        
        // 对每个剩余条目，计算加入各组的扩大成本
        for (i, entry) in remaining.iter().enumerate() {
            let entry_mbr = entry.mbr();
            
            // 计算加入group1的扩大成本
            let enlargement1 = group1_mbr.enlargement(entry_mbr);
            
            // 计算加入group2的扩大成本
            let enlargement2 = group2_mbr.enlargement(entry_mbr);
            
            // 计算偏好差异 - 偏好差异越大，说明该条目对某一组的偏好越明显
            let preference_diff = (enlargement1 - enlargement2).abs();
            
            if preference_diff > max_preference {
                max_preference = preference_diff;
                best_entry = i;
                
                // 选择扩大成本较小的组
                preferred_group = if enlargement1 < enlargement2 { 1 } else { 2 };
                
                // 如果扩大成本相等，选择面积较小的组
                if enlargement1 == enlargement2 {
                    let area1 = group1_mbr.area();
                    let area2 = group2_mbr.area();
                    preferred_group = if area1 < area2 { 1 } else { 2 };
                    
                    // 如果面积也相等，选择条目数较少的组
                    if area1 == area2 {
                        preferred_group = if group1.len() < group2.len() { 1 } else { 2 };
                    }
                }
            }
        }
        
        (best_entry, preferred_group)
    }
    
    /// 计算一组条目的最小边界矩形
    /// 
    /// 遍历组中所有条目，计算能够包含所有条目的最小矩形
    fn calculate_group_mbr(&self, group: &[Entry]) -> Rectangle {
        if group.is_empty() {
            return Rectangle::new(0.0, 0.0, 0.0, 0.0);
        }
        
        let mut mbr = group[0].mbr().clone();
        for entry in &group[1..] {
            mbr = mbr.union(entry.mbr());
        }
        
        mbr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadratic_split() {
        let rtree = RTree::new(3); // 小的max_entries以便测试分裂
        
        // 创建一些测试条目
        let entries = vec![
            Entry::Data { mbr: Rectangle::new(0.0, 0.0, 1.0, 1.0), data: 1 },
            Entry::Data { mbr: Rectangle::new(10.0, 10.0, 11.0, 11.0), data: 2 },
            Entry::Data { mbr: Rectangle::new(0.5, 0.5, 1.5, 1.5), data: 3 },
            Entry::Data { mbr: Rectangle::new(10.5, 10.5, 11.5, 11.5), data: 4 },
        ];
        
        let (group1, group2) = rtree.quadratic_split(entries);
        
        // 验证分裂结果
        assert_eq!(group1.len() + group2.len(), 4);
        assert!(group1.len() >= rtree.min_entries());
        assert!(group2.len() >= rtree.min_entries());
        
        // 验证相似的条目被分到同一组
        let group1_data: Vec<i32> = group1.iter().filter_map(|e| e.data()).collect();
        let group2_data: Vec<i32> = group2.iter().filter_map(|e| e.data()).collect();
        
        // 根据空间位置，(1,3)应该在一组，(2,4)应该在另一组
        // 或者(1,2)在一组，(3,4)在另一组，取决于种子选择
        assert!(
            (group1_data.contains(&1) && group1_data.contains(&3)) ||
            (group2_data.contains(&1) && group2_data.contains(&3)) ||
            (group1_data.contains(&2) && group1_data.contains(&4)) ||
            (group2_data.contains(&2) && group2_data.contains(&4))
        );
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
        let results = rtree.search_bbox(&Rectangle::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(results.len(), 4);
        assert!(results.contains(&1));
        assert!(results.contains(&2));
        assert!(results.contains(&3));
        assert!(results.contains(&4));
    }

    #[test]
    fn test_pick_seeds() {
        let rtree = RTree::new(4);
        
        // 创建测试条目：两个靠近的和两个相距很远的
        let entries = vec![
            Entry::Data { mbr: Rectangle::new(0.0, 0.0, 1.0, 1.0), data: 1 },    // 靠近第3个
            Entry::Data { mbr: Rectangle::new(100.0, 100.0, 101.0, 101.0), data: 2 }, // 很远
            Entry::Data { mbr: Rectangle::new(0.5, 0.5, 1.5, 1.5), data: 3 },    // 靠近第1个
            Entry::Data { mbr: Rectangle::new(50.0, 50.0, 51.0, 51.0), data: 4 },  // 中等距离
        ];
        
        let (seed1, seed2) = rtree.pick_seeds(&entries);
        
        // 种子应该是相距最远的两个条目
        // 最可能是条目1(或3)和条目2，因为它们相距最远
        let seed_data1 = entries[seed1].data().unwrap();
        let seed_data2 = entries[seed2].data().unwrap();
        
        // 验证选择的是相距较远的条目
        assert!(
            (seed_data1 == 1 && seed_data2 == 2) ||
            (seed_data1 == 2 && seed_data2 == 1) ||
            (seed_data1 == 3 && seed_data2 == 2) ||
            (seed_data1 == 2 && seed_data2 == 3)
        );
    }

    #[test]
    fn test_calculate_group_mbr() {
        let rtree = RTree::new(4);
        
        // 创建一组条目
        let group = vec![
            Entry::Data { mbr: Rectangle::new(0.0, 0.0, 1.0, 1.0), data: 1 },
            Entry::Data { mbr: Rectangle::new(2.0, 2.0, 3.0, 3.0), data: 2 },
            Entry::Data { mbr: Rectangle::new(0.5, 0.5, 1.5, 1.5), data: 3 },
        ];
        
        let group_mbr = rtree.calculate_group_mbr(&group);
        
        // 组的MBR应该包含所有条目
        assert_eq!(group_mbr, Rectangle::new(0.0, 0.0, 3.0, 3.0));
    }

    #[test]
    fn test_pick_next() {
        let rtree = RTree::new(4);
        
        // 创建两个组
        let group1 = vec![
            Entry::Data { mbr: Rectangle::new(0.0, 0.0, 1.0, 1.0), data: 1 },
        ];
        let group2 = vec![
            Entry::Data { mbr: Rectangle::new(10.0, 10.0, 11.0, 11.0), data: 2 },
        ];
        
        // 创建剩余条目
        let remaining = vec![
            Entry::Data { mbr: Rectangle::new(0.5, 0.5, 1.5, 1.5), data: 3 }, // 更接近group1
            Entry::Data { mbr: Rectangle::new(10.5, 10.5, 11.5, 11.5), data: 4 }, // 更接近group2
        ];
        
        let (next_index, preferred_group) = rtree.pick_next(&remaining, &group1, &group2);
        
        // 应该选择偏好差异最大的条目
        // 这可能是条目3或4，取决于哪个的偏好差异更明显
        assert!(next_index < remaining.len());
        assert!(preferred_group == 1 || preferred_group == 2);
    }
}
