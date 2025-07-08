use crate::rectangle::Rectangle;

/// R-tree节点条目
#[derive(Debug, Clone)]
pub enum Entry {
    /// 叶子条目：存储数据的MBR和数据ID
    Leaf { mbr: Rectangle, data: i32 },
    /// 内部条目：存储子节点的MBR和子节点引用
    Internal { mbr: Rectangle, child: Box<Node> },
}

impl Entry {
    /// 获取条目的MBR
    pub fn mbr(&self) -> &Rectangle {
        match self {
            Entry::Leaf { mbr, .. } => mbr,
            Entry::Internal { mbr, .. } => mbr,
        }
    }

    /// 获取条目的MBR（可变引用）
    pub fn mbr_mut(&mut self) -> &mut Rectangle {
        match self {
            Entry::Leaf { mbr, .. } => mbr,
            Entry::Internal { mbr, .. } => mbr,
        }
    }

    /// 检查是否为叶子条目
    pub fn is_leaf(&self) -> bool {
        matches!(self, Entry::Leaf { .. })
    }

    /// 获取叶子条目的数据（如果是叶子条目）
    pub fn data(&self) -> Option<i32> {
        match self {
            Entry::Leaf { data, .. } => Some(*data),
            Entry::Internal { .. } => None,
        }
    }

    /// 获取内部条目的子节点引用（如果是内部条目）
    pub fn child(&self) -> Option<&Node> {
        match self {
            Entry::Leaf { .. } => None,
            Entry::Internal { child, .. } => Some(child),
        }
    }

    /// 获取内部条目的子节点引用（可变，如果是内部条目）
    pub fn child_mut(&mut self) -> Option<&mut Node> {
        match self {
            Entry::Leaf { .. } => None,
            Entry::Internal { child, .. } => Some(child),
        }
    }
}

/// R-tree节点
#[derive(Debug, Clone)]
pub struct Node {
    /// 节点的最小边界矩形
    pub mbr: Rectangle,
    /// 节点包含的条目列表
    pub entries: Vec<Entry>,
    /// 是否为叶子节点
    pub is_leaf: bool,
    /// 节点在树中的层级（叶子节点层级为0）
    pub level: usize,
}

impl Node {
    /// 创建新的叶子节点
    pub fn new_leaf() -> Self {
        Node {
            mbr: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            entries: Vec::new(),
            is_leaf: true,
            level: 0,
        }
    }

    /// 创建新的内部节点
    pub fn new_internal(level: usize) -> Self {
        Node {
            mbr: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            entries: Vec::new(),
            is_leaf: false,
            level,
        }
    }

    /// 创建指定类型和层级的节点
    pub fn new(is_leaf: bool, level: usize) -> Self {
        Node {
            mbr: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            entries: Vec::new(),
            is_leaf,
            level,
        }
    }

    /// 更新节点的MBR以包含所有条目
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
    pub fn add_entry(&mut self, entry: Entry) {
        self.entries.push(entry);
        self.update_mbr();
    }

    /// 检查节点是否已满
    pub fn is_full(&self, max_entries: usize) -> bool {
        self.entries.len() >= max_entries
    }

    /// 检查节点是否需要更多条目
    pub fn needs_more_entries(&self, min_entries: usize) -> bool {
        self.entries.len() < min_entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let leaf = Node::new_leaf();
        assert!(leaf.is_leaf);
        assert_eq!(leaf.level, 0);
        
        let internal = Node::new_internal(1);
        assert!(!internal.is_leaf);
        assert_eq!(internal.level, 1);
    }

    #[test]
    fn test_node_update_mbr() {
        let mut node = Node::new_leaf();
        
        let entry1 = Entry::Leaf { 
            mbr: Rectangle::new(0.0, 0.0, 5.0, 5.0), 
            data: 1 
        };
        let entry2 = Entry::Leaf { 
            mbr: Rectangle::new(3.0, 3.0, 8.0, 8.0), 
            data: 2 
        };
        
        node.add_entry(entry1);
        node.add_entry(entry2);
        
        assert_eq!(node.mbr, Rectangle::new(0.0, 0.0, 8.0, 8.0));
    }

    #[test]
    fn test_entry_operations() {
        let entry = Entry::Leaf { 
            mbr: Rectangle::new(0.0, 0.0, 5.0, 5.0), 
            data: 42 
        };
        
        assert!(entry.is_leaf());
        assert_eq!(entry.data(), Some(42));
        assert_eq!(entry.mbr(), &Rectangle::new(0.0, 0.0, 5.0, 5.0));
    }
}
