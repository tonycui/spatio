use crate::rectangle::Rectangle;
use crate::node::Node;

/// R-tree主结构
pub struct RTree {
    /// 根节点
    root: Option<Box<Node>>,
    /// 最大条目数M
    max_entries: usize,
    /// 最小条目数m（通常为M/2）
    min_entries: usize,
}

impl RTree {
    /// 创建新的R-tree
    pub fn new(max_entries: usize) -> Self {
        assert!(max_entries >= 2, "Max entries must be at least 2");
        let min_entries = max_entries / 2;
        
        RTree {
            root: None,
            max_entries,
            min_entries,
        }
    }

    /// 使用默认参数创建R-tree（M=10, m=5）
    pub fn default() -> Self {
        Self::new(10)
    }

    /// 检查R-tree是否为空
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// 获取R-tree的根节点MBR
    pub fn root_mbr(&self) -> Option<&Rectangle> {
        self.root.as_ref().map(|node| &node.mbr)
    }

    /// 获取最大条目数
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// 获取最小条目数
    pub fn min_entries(&self) -> usize {
        self.min_entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtree_creation() {
        let rtree = RTree::new(10);
        assert_eq!(rtree.max_entries(), 10);
        assert_eq!(rtree.min_entries(), 5);
        assert!(rtree.is_empty());
    }
}
