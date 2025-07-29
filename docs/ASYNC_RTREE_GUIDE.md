# 异步并发 R-tree 用法指南

## 📖 概述

`AsyncConcurrentRTree` 是 geo42 项目中 R-tree 的异步并发实现，专为 tokio 异步环境设计。它提供了非阻塞的空间索引操作，适用于高并发的地理空间数据库应用。

## 🚀 快速开始

### 基础用法

```rust
use rtree::{AsyncConcurrentRTree, Rectangle};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建异步并发 R-tree
    let rtree = AsyncConcurrentRTree::new(4);
    
    // 异步插入数据
    let rect = Rectangle::new(0.0, 0.0, 1.0, 1.0);
    rtree.insert(rect, 1).await?;
    
    // 异步搜索
    let results = rtree.search(&rect).await?;
    println!("找到 {} 个项目", results.len());
    
    Ok(())
}
```

### 并发操作

```rust
use rtree::{AsyncConcurrentRTree, Rectangle};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rtree = Arc::new(AsyncConcurrentRTree::new(4));
    
    // 并发插入
    let mut tasks = Vec::new();
    for i in 0..100 {
        let rtree_clone = Arc::clone(&rtree);
        let task = tokio::spawn(async move {
            let rect = Rectangle::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
            rtree_clone.insert(rect, i).await
        });
        tasks.push(task);
    }
    
    // 等待所有任务完成
    for task in tasks {
        task.await??;
    }
    
    println!("插入了 {} 个项目", rtree.len().await?);
    Ok(())
}
```

## 🔧 API 参考

### 构造函数

#### `AsyncConcurrentRTree::new(max_entries: usize)`
创建默认超时时间(30秒)的异步并发 R-tree。

#### `AsyncConcurrentRTree::with_timeout(max_entries: usize, timeout: Duration)`
创建自定义超时时间的异步并发 R-tree。

#### `AsyncConcurrentRTree::from(rtree: RTree)`
从现有的 RTree 创建异步并发版本。

### 数据操作

#### `async fn insert(&self, rect: Rectangle, data: i32) -> Result<(), AsyncConcurrentError>`
异步插入数据项。

#### `async fn delete(&self, rect: &Rectangle, data: i32) -> Result<bool, AsyncConcurrentError>`
异步删除数据项，返回是否成功删除。

#### `async fn search(&self, rect: &Rectangle) -> Result<Vec<i32>, AsyncConcurrentError>`
异步搜索与矩形相交的所有数据项。

#### `async fn clear(&self) -> Result<(), AsyncConcurrentError>`
异步清空所有数据。

### 查询操作

#### `async fn len(&self) -> Result<usize, AsyncConcurrentError>`
异步获取数据项数量。

#### `async fn is_empty(&self) -> Result<bool, AsyncConcurrentError>`
异步检查是否为空。

### 带超时的操作

所有操作都有对应的 `_with_timeout` 版本，允许自定义超时时间：

```rust
// 带超时的插入
rtree.insert_with_timeout(rect, data, Duration::from_secs(1)).await?;

// 带超时的搜索
rtree.search_with_timeout(&rect, Duration::from_millis(500)).await?;
```

## 🎯 性能特性

### 并发性能

- **读操作**: 高度并发，多个搜索可以同时进行
- **写操作**: 互斥访问，但不会阻塞 tokio 线程
- **混合负载**: 读写操作可以并发执行

### 超时控制

- **默认超时**: 30秒，可配置
- **操作级超时**: 每个操作可以设置独立的超时时间
- **超时处理**: 超时时返回明确的错误信息

### 内存效率

- **零拷贝克隆**: `clone()` 只增加引用计数
- **共享状态**: 多个实例共享同一个底层 R-tree
- **锁粒度**: 整个 R-tree 级别的锁，适合空间索引的使用模式

## 📊 性能对比

| 特性 | ConcurrentRTree | AsyncConcurrentRTree |
|------|------------------|----------------------|
| 锁类型 | std::sync::RwLock | tokio::sync::RwLock |
| 线程阻塞 | 会阻塞线程 | 不阻塞 tokio 线程 |
| 异步支持 | ❌ | ✅ |
| 超时控制 | ❌ | ✅ |
| 并发读取 | ✅ | ✅ |
| 适用场景 | CPU 密集任务 | I/O 密集的异步应用 |

## 🔍 错误处理

### 错误类型

```rust
pub enum AsyncConcurrentError {
    Timeout { timeout: Duration },     // 操作超时
    OperationFailed(String),          // 操作失败
    LockFailed,                       // 锁获取失败
}
```

### 错误处理示例

```rust
match rtree.search(&rect).await {
    Ok(results) => println!("找到 {} 个结果", results.len()),
    Err(AsyncConcurrentError::Timeout { timeout }) => {
        println!("搜索超时: {:?}", timeout);
    }
    Err(e) => println!("搜索失败: {}", e),
}
```

## 🧪 测试和基准

### 运行测试

```bash
# 运行所有异步并发测试
cargo test async_concurrent

# 运行演示程序
cargo run --example async_concurrent_demo
```

### 性能基准

演示程序包含了各种性能测试场景：

- 并发插入 100 个项目
- 并发读取 50 次搜索
- 混合读写操作
- 超时控制测试

## 🔄 与 geo42 集成

在 geo42 项目中，`AsyncConcurrentRTree` 将替代当前的同步 R-tree 实现，用于：

1. **INTERSECTS 命令**: 异步空间查询
2. **SET/GET 命令**: 非阻塞的数据插入和检索
3. **并发客户端**: 支持多个客户端同时操作

### 集成示例

```rust
// 在 GeoDatabase 中使用
pub struct CollectionData {
    pub items: HashMap<String, GeoItem>,
    pub rtree: Option<AsyncConcurrentRTree>,  // 使用异步版本
}

impl GeoDatabase {
    pub async fn intersects(&self, collection_id: &str, geometry: &Value) -> Result<Vec<GeoItem>> {
        let collection = self.get_collection(collection_id).await?;
        let bbox = extract_bbox(geometry)?;
        let candidate_ids = collection.rtree.search(&bbox).await?;
        // ... 处理结果
    }
}
```

## 📈 最佳实践

### 1. 合理设置超时时间

```rust
// 对于交互式应用，使用较短的超时
let rtree = AsyncConcurrentRTree::with_timeout(4, Duration::from_secs(5));

// 对于批处理应用，使用较长的超时
let rtree = AsyncConcurrentRTree::with_timeout(4, Duration::from_secs(60));
```

### 2. 错误处理策略

```rust
// 重试机制
async fn search_with_retry(rtree: &AsyncConcurrentRTree, rect: &Rectangle) -> Result<Vec<i32>, AsyncConcurrentError> {
    let mut attempts = 0;
    loop {
        match rtree.search(rect).await {
            Ok(results) => return Ok(results),
            Err(AsyncConcurrentError::Timeout { .. }) if attempts < 3 => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 3. 资源管理

```rust
// 使用 Arc 共享 R-tree 实例
let rtree = Arc::new(AsyncConcurrentRTree::new(4));

// 在多个任务中共享
let tasks: Vec<_> = (0..10).map(|i| {
    let rtree_clone = Arc::clone(&rtree);
    tokio::spawn(async move {
        // 使用 rtree_clone
    })
}).collect();
```

## 🔗 相关链接

- [R-tree 算法论文](https://www.cs.umb.edu/~poneil/lec20-Rtree.pdf)
- [tokio::sync 文档](https://docs.rs/tokio/latest/tokio/sync/)
- [geo42 项目主页](../README.md)

---

*最后更新: 2025年7月29日*
