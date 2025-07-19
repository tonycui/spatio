//! # R-tree 空间索引数据结构
//! 
//! 这是一个基于 Antonin Guttman 的论文 "R-trees: A Dynamic Index Structure for Spatial Searching" 
//! 的 R-tree 实现。
//! 
//! ## 主要特性
//! 
//! - 支持二维空间数据的高效索引
//! - 实现了论文中的插入、搜索、删除算法
//! - 使用二次分裂算法进行节点分裂
//! - 可配置的最大/最小条目数
//! 
//! ## 使用示例
//! 
//! ```rust
//! use rtree::{RTree, Rectangle};
//! 
//! // 创建 R-tree
//! let rtree = RTree::new(10);
//! 
//! // 创建矩形
//! let rect = Rectangle::new(0.0, 0.0, 10.0, 10.0);
//! 
//! // 检查 R-tree 是否为空
//! assert!(rtree.is_empty());
//! ```

pub mod rectangle;
pub mod node;
pub mod rtree;
pub mod algorithms;

// 重新导出主要的公共接口
pub use rectangle::Rectangle;
pub use node::{Node, Entry};
pub use rtree::RTree;
pub use algorithms::concurrent::{ConcurrentRTree, ConcurrentError};
