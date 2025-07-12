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

    /// 搜索与查询矩形相交的所有条目 - 遵循论文Algorithm Search
    pub fn search(&self, query: &Rectangle) -> Vec<i32> {
        let mut results = Vec::new();
        
        if let Some(root) = self.root_ref() {
            self.search_recursive(root, query, &mut results);
        }
        
        results
    }

    /// 删除指定的数据条目 - 使用简化的下溢处理策略
    pub fn delete(&mut self, rect: &Rectangle, data: i32) -> bool {
        // // test
        // if data == 953 || data == 953 || data == 953 {
        //     println!("Debug: Attempting to delete data {}", data);
        //     println!("\n=== COMPLETE TREE STRUCTURE ===");
        //     Self::print_tree_structure_debug(self);
        // }


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
                        !(mbr == rect && *entry_data == data)
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

                // // for test
                // println!("leaf_node:{:#?}",leaf_node);
                
                (true, leaf_node.entries.len())
            };
            // if data == 953 {
            //     println!("Debug: Deleted data {}, leaf entries count: {}", data, leaf_entries_count);
            // }
            
            if deleted {
                // D3: 检查叶子节点是否下溢
                let min_entries = self.min_entries_internal();

                // // for test
                // println!("leaf_path:{:#?}, leaf_entries_count:{leaf_entries_count}",leaf_path);
                
                if leaf_entries_count < min_entries && !leaf_path.is_empty() {
                    // 叶子节点下溢且不是根节点 - 使用简化的处理方案
                    // println!("Before handle_leaf_underflow");
                    self.handle_leaf_underflow(leaf_path.clone());
                    // for test
                    // println!("After handle_leaf_underflow,Complete R-tree structure:");
                    // Self::print_tree_structure_debug(self);

                    // let parent_path = &leaf_path[..leaf_path.len() - 1];
                    // let parent_path = &leaf_path[..leaf_path.len() - 1];
                    // let parent = self.get_last_node_mut(parent_path);
                    // if parent.entries.is_empty() {
                    //     // 父节点也变空了，递归处理父节点
                    //     self.remove_empty_nodes(parent_path.to_vec());
                    // } else {
                    //     // 父节点不为空，向上调整MBR
                    //     // 因为在handle_leaf_underflow里面已经adjust_tree_upward，这里
                    //     // 暂时先注释掉
                    //     // self.adjust_tree_upward(parent_path);
                    // }
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
                // // 添加详细的调试信息
                // println!("=== PANIC DEBUG INFO ===");
                // println!("Current node details:");
                // println!("  Level: {}", current.level);
                // println!("  Type: {:?}", current.node_type);
                // println!("  MBR: [{},{},{},{}]", 
                //     current.mbr.min[0], current.mbr.min[1], 
                //     current.mbr.max[0], current.mbr.max[1]);
                // println!("  Total entries: {}", current.entries.len());
                // println!("  Best index chosen: {}", best_index);
                
                // println!("\nAll entries in current node:");
                // for (i, entry) in current.entries.iter().enumerate() {
                //     match entry {
                //         Entry::Data { mbr, data } => {
                //             println!("  [{}] Data: {} at [{},{},{},{}]", 
                //                 i, data, mbr.min[0], mbr.min[1], mbr.max[0], mbr.max[1]);
                //         }
                //         Entry::Node { mbr, node } => {
                //             println!("  [{}] Node: mbr=[{},{},{},{}], level={}, type={:?}, {} entries", 
                //                 i, mbr.min[0], mbr.min[1], mbr.max[0], mbr.max[1],
                //                 node.level, node.node_type, node.entries.len());
                //         }
                //     }
                // }
                
                // println!("\nTarget rectangle: [{},{},{},{}]", 
                //     rect.min[0], rect.min[1], rect.max[0], rect.max[1]);
                
                // println!("\nChoose subtree calculation details:");
                // for (i, entry) in current.entries.iter().enumerate() {
                //     let mbr = entry.mbr();
                //     let enlargement = mbr.enlargement(rect);
                //     let area = mbr.area();
                //     println!("  Entry[{}]: enlargement={:.2}, area={:.2}", i, enlargement, area);
                // }
                
                // // // 添加完整的树结构打印
                // // println!("\n=== COMPLETE TREE STRUCTURE ===");
                // // Self::print_tree_structure_debug(self);


                // panic!("Expected child entry at index {} but found {:?}", 
                //     best_index, 
                //     current.entries.get(best_index).map(|e| match e {
                //         Entry::Data { data, .. } => format!("Data({})", data),
                //         Entry::Node { .. } => "Node".to_string(),
                //     }).unwrap_or_else(|| "None".to_string())
                // );
            
                // panic!("Expected child entry");
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

    /// 处理节点溢出 - 使用二次分裂算法
    fn handle_overflow(&mut self, path: Vec<usize>) {
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
    fn split_and_propagate(&mut self, mut path: Vec<usize>) {
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

    /// 向上调整树 - 更新MBR
    fn adjust_tree_upward(&mut self, mut path: Vec<usize>) {
        // 从叶子节点向上更新每一层的MBR
        while !path.is_empty() {
            
            // 获取当前节点,如果当前节点为空，则跳过当前节点
            let node = match self.get_last_node_mut(&path) {
                    Some(node) => node,
                    None => {
                        path.pop().unwrap();
                        break;
                    }
                };
            
            // 更新当前节点的MBR
            node.update_mbr();

            // 更新当前节点的MBR
            // let current_mbr = {
            //     let mut current_path = path.clone();
            //     current_path.push(current_node_index);
            //     let node = match self.get_last_node_mut(&current_path) {
            //         Some(node) => node,
            //         None => {
            //             println!("Warning: Failed to get node during MBR update");
            //             break;
            //         }
            //     };
            //     node.update_mbr();
            //     node.mbr.clone()
            // };

            // 将当前节点从 path 中移除
            let current_node_index = path[path.len() - 1];
            path.pop().unwrap();

            let current_mbr = node.mbr.clone();
            
            // 更新父节点中指向当前节点的条目的MBR
            if path.is_empty() {
                // 当前节点是根节点的直接子节点，更新根节点中的条目
                if let Some(root) = self.root_mut() {
                    if let Some(Entry::Node { mbr, .. }) = root.entries.get_mut(current_node_index) {
                        *mbr = current_mbr;
                    }
                }
            } else {
                // 更新中间层的父节点
                let parent = match self.get_last_node_mut(&path) {
                    Some(node) => node,
                    None => {
                        println!("Warning: Failed to get parent node (Empty Node) during MBR update");
                        break;
                    }
                };
                if let Some(Entry::Node { mbr, .. }) = parent.entries.get_mut(current_node_index) {
                    *mbr = current_mbr;
                }
            }
        }
        
        // 更新根节点自身的MBR
        if let Some(root) = self.root_mut() {
            root.update_mbr();
        }
    }

    /// 根据路径获取节点的可变引用
    fn get_last_node_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        let mut current = self.root_mut().as_mut().unwrap();
        
        for &index in path {
            if let Some(Entry::Node { node, .. }) = current.entries.get_mut(index) {
                current = node;
            } else {
                return None;
            }
        }
        
        Some(current)
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
    fn remove_empty_nodes(&mut self, node_path: Vec<usize>) {
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
    
    /// 查找包含指定数据条目的叶子节点路径
    /// 
    /// 返回从根节点到包含目标条目的叶子节点的路径
    fn find_leaf_path(&self, rect: &Rectangle, data: i32) -> Option<Vec<usize>> {
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
    
    // 注意：这里移除了原始的condense_tree方法，改为使用简化的handle_leaf_underflow方案
    // 原始方案处理所有层次的节点下溢，但逻辑复杂
    // 新方案只处理叶子节点下溢，通过重新插入的方式解决，逻辑更简单清晰
    
    /// 处理叶子节点下溢 - 简化方案
    /// 
    /// 1. 收集下溢叶子节点中的所有数据条目
    /// 2. 将这些条目重新插入到树中
    /// 3. 从父节点中移除下溢的叶子节点
    /// 4. 向上调整MBR
    fn handle_leaf_underflow(&mut self, leaf_path: Vec<usize>) {
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
        
        // // for test
        // println!("In handle_leaf_underflow, before insert,Complete R-tree structure:");
        // Self::print_tree_structure_debug(self);

        // 4. 重新插入收集到的数据条目
        for (mbr, data) in entries_to_reinsert {
            self.insert(mbr, data);
        }
    }
    
    /// 缩短树 - 如果根节点只有一个条目且为索引节点，则将其子节点作为新的根节点
    fn shorten_tree(&mut self) {
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

/// 节点分裂算法 - 实现完整的二次分裂(Quadratic Split)
impl RTree {
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

/// 树结构调整算法
impl RTree {
    /// 计算扩大成本
    #[allow(dead_code)]
    fn enlargement_cost(&self, mbr: &Rectangle, rect: &Rectangle) -> f64 {
        mbr.enlargement(rect)
    }
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
