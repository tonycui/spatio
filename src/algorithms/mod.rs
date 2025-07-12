// 算法模块声明文件
// 重新导出所有子模块

pub mod search;
pub mod insert;

// 临时包含原始algorithms文件的其余功能
// 这些功能将在后续步骤中逐步迁移到专门的模块中
#[path = "../algorithms_old.rs"]
pub mod algorithms_old;
