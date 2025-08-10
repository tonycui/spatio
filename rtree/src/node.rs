use crate::rectangle::Rectangle;
use serde::{Deserialize, Serialize};

/// R-tree节点类型
/// 
/// 用于明确区分R-tree中的两种节点类型，避免概念混淆
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    /// 叶子节点：包含用户插入的真实数据条目
    /// 这些节点位于R-tree的叶子层，直接存储用户数据
    Leaf,
    /// 索引节点：包含指向子节点的引用条目
    /// 这些节点位于R-tree的内部层，用于索引和导航
    Index,
}

/// R-tree节点条目
/// 
/// 每个条目都包含一个MBR（最小边界矩形）和对应的内容：
/// - Data条目：存储用户插入的真实数据，只出现在叶子节点中
/// - Node条目：存储子节点的引用，只出现在索引节点中
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Entry {
    /// 数据条目：存储用户插入的真实数据
    /// 
    /// 包含：
    /// - mbr: 数据的最小边界矩形
    /// - data: 用户数据的ID或值
    /// 
    /// 只会出现在NodeType::Leaf类型的节点中
    Data { mbr: Rectangle, data: String },
    
    /// 节点条目：存储指向子节点的引用
    /// 
    /// 包含：
    /// - mbr: 子节点的最小边界矩形（包含该子节点所有条目的MBR）
    /// - node: 指向子节点的Box智能指针
    /// 
    /// 只会出现在NodeType::Index类型的节点中
    Node { mbr: Rectangle, node: Box<Node> },
}

impl Entry {
    /// 获取条目的MBR（最小边界矩形）
    /// 
    /// 无论是数据条目还是节点条目，都有对应的MBR
    pub fn mbr(&self) -> &Rectangle {
        match self {
            Entry::Data { mbr, .. } => mbr,
            Entry::Node { mbr, .. } => mbr,
        }
    }

    /// 获取条目的MBR（可变引用）
    /// 
    /// 用于需要修改MBR的场景，如节点分裂后更新MBR
    pub fn mbr_mut(&mut self) -> &mut Rectangle {
        match self {
            Entry::Data { mbr, .. } => mbr,
            Entry::Node { mbr, .. } => mbr,
        }
    }

    /// 检查是否为数据条目
    /// 
    /// 返回true表示这是存储真实用户数据的条目
    pub fn is_data(&self) -> bool {
        matches!(self, Entry::Data { .. })
    }

    /// 获取数据条目的数据值（如果是数据条目）
    /// 
    /// 只有Entry::Data类型的条目才会返回Some(data)
    /// Entry::Node类型的条目返回None
    pub fn data(&self) -> Option<String> {
        match self {
            Entry::Data { data, .. } => Some(data.clone()),
            Entry::Node { .. } => None,
        }
    }

    /// 获取节点条目的子节点引用（如果是节点条目）
    /// 
    /// 只有Entry::Node类型的条目才会返回Some(node)
    /// Entry::Data类型的条目返回None
    pub fn child(&self) -> Option<&Node> {
        match self {
            Entry::Data { .. } => None,
            Entry::Node { node, .. } => Some(node),
        }
    }

    /// 获取节点条目的子节点引用（可变，如果是节点条目）
    /// 
    /// 用于需要修改子节点的场景
    pub fn child_mut(&mut self) -> Option<&mut Node> {
        match self {
            Entry::Data { .. } => None,
            Entry::Node { node, .. } => Some(node),
        }
    }
}

/// R-tree节点
/// 
/// R-tree的核心数据结构，表示树中的一个节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// 节点的最小边界矩形
    /// 
    /// 对于数据节点：包含该节点所有数据条目的MBR
    /// 对于索引节点：包含该节点所有子节点的MBR
    pub mbr: Rectangle,
    
    /// 节点包含的条目列表
    /// 
    /// 条目类型取决于节点类型：
    /// - 叶子节点：只包含Entry::Data条目（用户数据）
    /// - 索引节点：只包含Entry::Node条目（子节点引用）
    pub entries: Vec<Entry>,
    
    /// 节点类型
    /// 
    /// NodeType::Leaf - 叶子节点，包含用户插入的真实数据
    /// NodeType::Index - 索引节点，包含指向子节点的引用
    pub node_type: NodeType,
    
    /// 节点在树中的层级
    /// 
    /// 数据节点（叶子层）的层级为0
    /// 索引节点的层级 > 0，根节点层级最高
    pub level: usize,
}

impl Node {
    /// 创建新的叶子节点
    /// 
    /// 叶子节点位于R-tree的叶子层，用于存储用户插入的真实数据
    /// 层级固定为0
    pub fn new_leaf_node() -> Self {
        Node {
            mbr: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            entries: Vec::new(),
            node_type: NodeType::Leaf,
            level: 0,
        }
    }

    /// 创建新的索引节点
    /// 
    /// 索引节点位于R-tree的内部层，用于存储指向子节点的引用
    /// 
    /// # 参数
    /// * `level` - 节点在树中的层级，必须 > 0
    pub fn new_index_node(level: usize) -> Self {
        Node {
            mbr: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            entries: Vec::new(),
            node_type: NodeType::Index,
            level,
        }
    }

    /// 创建指定类型和层级的节点
    /// 
    /// # 参数
    /// * `node_type` - 节点类型（数据节点或索引节点）
    /// * `level` - 节点在树中的层级
    pub fn new(node_type: NodeType, level: usize) -> Self {
        Node {
            mbr: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            entries: Vec::new(),
            node_type,
            level,
        }
    }

    /// 检查是否为叶子节点
    /// 
    /// 返回true表示这是存储用户数据的叶子节点
    pub fn is_leaf_node(&self) -> bool {
        matches!(self.node_type, NodeType::Leaf)
    }

    /// 检查是否为索引节点
    /// 
    /// 返回true表示这是存储子节点引用的内部节点
    pub fn is_index_node(&self) -> bool {
        matches!(self.node_type, NodeType::Index)
    }

    /// 更新节点的MBR以包含所有条目
    /// 
    /// 遍历节点中的所有条目，计算能够包含所有条目MBR的最小边界矩形
    pub fn update_mbr(&mut self) {
        if self.entries.is_empty() {
            self.mbr = Rectangle::new(0.0, 0.0, 0.0, 0.0);
            return;
        }

        let first_mbr = self.entries[0].mbr();
        let mut min_x = first_mbr.min[0];
        let mut min_y = first_mbr.min[1];
        let mut max_x = first_mbr.max[0];
        let mut max_y = first_mbr.max[1];

        for entry in &self.entries[1..] {
            let mbr = entry.mbr();
            min_x = min_x.min(mbr.min[0]);
            min_y = min_y.min(mbr.min[1]);
            max_x = max_x.max(mbr.max[0]);
            max_y = max_y.max(mbr.max[1]);
        }

        self.mbr = Rectangle::new(min_x, min_y, max_x, max_y);
    }

    /// 添加条目到节点
    /// 
    /// 添加条目后会自动更新节点的MBR
    /// 
    /// # 参数
    /// * `entry` - 要添加的条目
    /// 
    /// # 注意
    /// 调用者需要确保条目类型与节点类型匹配：
    /// - 叶子节点只能添加Entry::Data条目
    /// - 索引节点只能添加Entry::Node条目
    pub fn add_entry(&mut self, entry: Entry) {
        self.entries.push(entry);
        self.update_mbr();
    }

    /// 检查节点是否已满
    /// 
    /// # 参数
    /// * `max_entries` - 节点允许的最大条目数
    pub fn is_full(&self, max_entries: usize) -> bool {
        self.entries.len() >= max_entries
    }

    /// 检查节点是否需要更多条目
    /// 
    /// # 参数
    /// * `min_entries` - 节点要求的最小条目数
    pub fn needs_more_entries(&self, min_entries: usize) -> bool {
        self.entries.len() < min_entries
    }

    // 为了兼容现有代码，保留旧的方法名（标记为已废弃）
    
    /// 创建新的叶子节点
    /// 
    /// @deprecated 请使用 new_leaf_node() 替代
    #[deprecated(note = "请使用 new_leaf_node() 替代")]
    pub fn new_leaf() -> Self {
        Self::new_leaf_node()
    }

    /// 创建新的内部节点
    /// 
    /// @deprecated 请使用 new_index_node() 替代
    #[deprecated(note = "请使用 new_index_node() 替代")]
    pub fn new_internal(level: usize) -> Self {
        Self::new_index_node(level)
    }

    /// 检查是否为叶子节点
    /// 
    /// @deprecated 请使用 is_leaf_node() 替代
    #[deprecated(note = "请使用 is_leaf_node() 替代")]
    pub fn is_leaf(&self) -> bool {
        self.is_leaf_node()
    }

    /// 创建新的数据节点
    /// 
    /// @deprecated 请使用 new_leaf_node() 替代
    #[deprecated(note = "请使用 new_leaf_node() 替代")]
    pub fn new_data_node() -> Self {
        Self::new_leaf_node()
    }

    /// 检查是否为数据节点
    /// 
    /// @deprecated 请使用 is_leaf_node() 替代
    #[deprecated(note = "请使用 is_leaf_node() 替代")]
    pub fn is_data_node(&self) -> bool {
        self.is_leaf_node()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        // 测试叶子节点创建
        let leaf_node = Node::new_leaf_node();
        assert!(leaf_node.is_leaf_node());
        assert!(!leaf_node.is_index_node());
        assert_eq!(leaf_node.level, 0);
        assert_eq!(leaf_node.node_type, NodeType::Leaf);
        
        // 测试索引节点创建
        let index_node = Node::new_index_node(1);
        assert!(!index_node.is_leaf_node());
        assert!(index_node.is_index_node());
        assert_eq!(index_node.level, 1);
        assert_eq!(index_node.node_type, NodeType::Index);
    }

    #[test]
    fn test_node_update_mbr() {
        let mut node = Node::new_leaf_node();
        
        let entry1 = Entry::Data { 
            mbr: Rectangle::new(0.0, 0.0, 5.0, 5.0), 
            data: "1".to_string() 
        };
        let entry2 = Entry::Data { 
            mbr: Rectangle::new(3.0, 3.0, 8.0, 8.0), 
            data: "2".to_string() 
        };
        
        node.add_entry(entry1);
        node.add_entry(entry2);
        
        assert_eq!(node.mbr, Rectangle::new(0.0, 0.0, 8.0, 8.0));
    }

    #[test]
    fn test_entry_operations() {
        // 测试数据条目
        let data_entry = Entry::Data { 
            mbr: Rectangle::new(0.0, 0.0, 5.0, 5.0), 
            data: "42".to_string() 
        };
        
        assert!(data_entry.is_data());
        assert_eq!(data_entry.data(), Some("42".to_string()));
        assert_eq!(data_entry.mbr(), &Rectangle::new(0.0, 0.0, 5.0, 5.0));
        assert!(data_entry.child().is_none());

        // 测试节点条目
        let child_node = Box::new(Node::new_leaf_node());
        let node_entry = Entry::Node {
            mbr: Rectangle::new(1.0, 1.0, 6.0, 6.0),
            node: child_node,
        };
        
        assert!(!node_entry.is_data());
        assert_eq!(node_entry.data(), None);
        assert_eq!(node_entry.mbr(), &Rectangle::new(1.0, 1.0, 6.0, 6.0));
        assert!(node_entry.child().is_some());
    }

    #[test]
    fn test_node_types() {
        let leaf_node = Node::new(NodeType::Leaf, 0);
        let index_node = Node::new(NodeType::Index, 1);
        
        assert_eq!(leaf_node.node_type, NodeType::Leaf);
        assert_eq!(index_node.node_type, NodeType::Index);
        
        assert!(leaf_node.is_leaf_node());
        assert!(!leaf_node.is_index_node());
        
        assert!(!index_node.is_leaf_node());
        assert!(index_node.is_index_node());
    }

    #[test] 
    fn test_deprecated_methods() {
        // 测试向后兼容的废弃方法
        #[allow(deprecated)]
        {
            let leaf = Node::new_leaf();
            assert!(leaf.is_leaf());
            
            let internal = Node::new_internal(1);
            assert!(!internal.is_leaf());
        }
    }
}
