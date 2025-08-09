pub mod rectangle;
pub mod node;
pub mod rtree;
pub mod algorithms;

// 重新导出主要的公共接口
pub use rectangle::Rectangle;
pub use node::{Node, Entry};
pub use rtree::RTree;

