// R-tree算法模块
//
// 这个模块包含R-tree的所有核心算法实现，按功能分解为不同的子模块：
// - search: 搜索和查询算法
// - insert: 插入和树构建算法
// - split: 节点分裂算法
// - delete: 删除和树维护算法
// - knn: K-最近邻搜索算法
// - utils: 共用的工具函数
// - debug: 调试和可视化工具
// - persistence: 持久化和序列化功能（RDB 快照）
// - aof: AOF (Append-Only File) 持久化功能
// - concurrent: 并发安全的R-tree实现（使用 std::sync）
// - async_concurrent: 异步并发安全的R-tree实现（使用 tokio::sync）

pub mod aof;
pub mod debug;
pub mod delete;
pub mod insert;
pub mod knn;
pub mod persistence;
pub mod search;
pub mod split;
pub mod utils;
