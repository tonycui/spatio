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
//! - 支持并发安全访问（同步和异步）
//! 
//! ## 使用示例
//! 
//! ### 基础用法
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
//! 
//! ### 并发使用（同步）
//! ```rust
//! use rtree::{ConcurrentRTree, Rectangle};
//! use std::thread;
//! 
//! let rtree = ConcurrentRTree::new(4);
//! let rtree_clone = rtree.clone();
//! 
//! let handle = thread::spawn(move || {
//!     rtree_clone.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).unwrap();
//! });
//! 
//! handle.join().unwrap();
//! assert_eq!(rtree.len().unwrap(), 1);
//! ```
//! 
//! ### 并发使用（异步）
//! ```rust
//! use rtree::{AsyncConcurrentRTree, Rectangle};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let rtree = AsyncConcurrentRTree::new(4);
//!     rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).await?;
//!     assert_eq!(rtree.len().await?, 1);
//!     Ok(())
//! }
//! ```

pub mod rectangle;
pub mod node;
pub mod rtree;
pub mod algorithms;

// 重新导出主要的公共接口
pub use rectangle::Rectangle;
pub use node::{Node, Entry};
pub use rtree::RTree;

// 并发版本
pub use algorithms::concurrent::{ConcurrentRTree, ConcurrentError};

// 异步并发版本
pub use algorithms::async_concurrent::{AsyncConcurrentRTree, AsyncConcurrentError};
