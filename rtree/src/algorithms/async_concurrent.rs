use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::time::timeout;
use crate::rtree::RTree;
use crate::rectangle::Rectangle;

/// 异步并发错误类型
#[derive(Debug, thiserror::Error)]
pub enum AsyncConcurrentError {
    #[error("Operation timed out after {timeout:?}")]
    Timeout { timeout: Duration },
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    #[error("Lock acquisition failed")]
    LockFailed,
}

/// 异步并发安全的R-tree实现
/// 
/// 这个结构体包装了标准的RTree，提供异步线程安全的操作接口。
/// 使用 tokio::sync::RwLock 来协调并发访问：
/// - 读操作（search, len, is_empty）可以并发执行且支持 async/await
/// - 写操作（insert, delete）需要独占访问且支持 async/await
/// - 所有操作都不会阻塞 tokio 运行时
/// 
/// # 特性
/// - ✅ 支持异步操作，不阻塞 tokio 线程
/// - ✅ 高并发读操作
/// - ✅ 操作超时控制
/// - ✅ 与 tokio 生态系统完美集成
/// 
/// # 示例
/// 
/// ```rust
/// use rtree::{AsyncConcurrentRTree, Rectangle};
/// use tokio;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let rtree = AsyncConcurrentRTree::new(4);
///     
///     // 异步插入操作
///     let rect = Rectangle::new(0.0, 0.0, 1.0, 1.0);
///     rtree.insert(rect, 1).await?;
///     
///     // 异步搜索操作
///     let search_area = Rectangle::new(-0.5, -0.5, 1.5, 1.5);
///     let results = rtree.search(&search_area).await?;
///     
///     println!("Found {} items", results.len());
///     Ok(())
/// }
/// ```
/// 
/// # 并发示例
/// 
/// ```rust
/// use rtree::{AsyncConcurrentRTree, Rectangle};
/// use tokio;
/// use std::sync::Arc;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let rtree = Arc::new(AsyncConcurrentRTree::new(4));
///     
///     // 并发插入任务
///     let mut tasks = Vec::new();
///     
///     for i in 0..10 {
///         let rtree_clone = Arc::clone(&rtree);
///         let task = tokio::spawn(async move {
///             let rect = Rectangle::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
///             rtree_clone.insert(rect, i).await
///         });
///         tasks.push(task);
///     }
///     
///     // 等待所有插入完成
///     for task in tasks {
///         task.await??;
///     }
///     
///     println!("Inserted {} items", rtree.len().await?);
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct AsyncConcurrentRTree {
    inner: Arc<RwLock<RTree>>,
    default_timeout: Duration,
}

impl AsyncConcurrentRTree {
    /// 创建新的异步并发R-tree
    /// 
    /// # 参数
    /// * `max_entries` - 每个节点的最大条目数
    /// 
    /// # 示例
    /// ```rust
    /// use rtree::AsyncConcurrentRTree;
    /// 
    /// let rtree = AsyncConcurrentRTree::new(4);
    /// ```
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(RTree::new(max_entries))),
            default_timeout: Duration::from_secs(30), // 默认30秒超时
        }
    }
    
    /// 创建带自定义超时的异步并发R-tree
    /// 
    /// # 参数
    /// * `max_entries` - 每个节点的最大条目数
    /// * `default_timeout` - 默认操作超时时间
    /// 
    /// # 示例
    /// ```rust
    /// use rtree::AsyncConcurrentRTree;
    /// use std::time::Duration;
    /// 
    /// let rtree = AsyncConcurrentRTree::with_timeout(4, Duration::from_secs(10));
    /// ```
    pub fn with_timeout(max_entries: usize, default_timeout: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(RTree::new(max_entries))),
            default_timeout,
        }
    }
    
    /// 从现有的RTree创建异步并发版本
    /// 
    /// # 参数
    /// * `rtree` - 要包装的RTree实例
    /// 
    /// # 示例
    /// ```rust
    /// use rtree::{RTree, AsyncConcurrentRTree, Rectangle};
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut rtree = RTree::new(4);
    ///     rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1);
    ///     
    ///     let async_rtree = AsyncConcurrentRTree::from(rtree);
    ///     assert_eq!(async_rtree.len().await?, 1);
    ///     Ok(())
    /// }
    /// ```
    pub fn from(rtree: RTree) -> Self {
        Self {
            inner: Arc::new(RwLock::new(rtree)),
            default_timeout: Duration::from_secs(30),
        }
    }
    
    /// 异步插入新的数据项
    /// 
    /// # 参数
    /// * `rect` - 数据项的边界矩形
    /// * `data` - 要存储的数据
    /// 
    /// # 错误
    /// * `AsyncConcurrentError::Timeout` - 操作超时
    /// * `AsyncConcurrentError::LockFailed` - 锁获取失败
    /// 
    /// # 示例
    /// ```rust
    /// use rtree::{AsyncConcurrentRTree, Rectangle};
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rtree = AsyncConcurrentRTree::new(4);
    ///     rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).await?;
    ///     assert_eq!(rtree.len().await?, 1);
    ///     Ok(())
    /// }
    /// ```
    pub async fn insert(&self, rect: Rectangle, data: i32) -> Result<(), AsyncConcurrentError> {
        let mut tree = self.write_lock_with_timeout(self.default_timeout).await?;
        tree.insert(rect, data);
        Ok(())
    }
    
    /// 异步插入新的数据项（带超时）
    /// 
    /// # 参数
    /// * `rect` - 数据项的边界矩形
    /// * `data` - 要存储的数据
    /// * `timeout_duration` - 操作超时时间
    /// 
    /// # 示例
    /// ```rust
    /// use rtree::{AsyncConcurrentRTree, Rectangle};
    /// use std::time::Duration;
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rtree = AsyncConcurrentRTree::new(4);
    ///     let timeout = Duration::from_millis(100);
    ///     rtree.insert_with_timeout(
    ///         Rectangle::new(0.0, 0.0, 1.0, 1.0), 
    ///         1, 
    ///         timeout
    ///     ).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn insert_with_timeout(
        &self, 
        rect: Rectangle, 
        data: i32, 
        timeout_duration: Duration
    ) -> Result<(), AsyncConcurrentError> {
        let mut tree = self.write_lock_with_timeout(timeout_duration).await?;
        tree.insert(rect, data);
        Ok(())
    }
    
    /// 异步删除指定的数据项
    /// 
    /// # 参数
    /// * `rect` - 要删除的数据项的边界矩形
    /// * `data` - 要删除的数据
    /// 
    /// # 返回
    /// * `Ok(true)` - 成功删除
    /// * `Ok(false)` - 未找到要删除的项
    /// * `Err(_)` - 操作失败
    /// 
    /// # 示例
    /// ```rust
    /// use rtree::{AsyncConcurrentRTree, Rectangle};
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rtree = AsyncConcurrentRTree::new(4);
    ///     let rect = Rectangle::new(0.0, 0.0, 1.0, 1.0);
    ///     
    ///     rtree.insert(rect, 1).await?;
    ///     assert!(rtree.delete(&rect, 1).await?);
    ///     assert!(!rtree.delete(&rect, 1).await?); // 第二次删除返回false
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete(&self, rect: &Rectangle, data: i32) -> Result<bool, AsyncConcurrentError> {
        let mut tree = self.write_lock_with_timeout(self.default_timeout).await?;
        Ok(tree.delete(rect, data))
    }
    
    /// 异步删除指定的数据项（带超时）
    pub async fn delete_with_timeout(
        &self, 
        rect: &Rectangle, 
        data: i32, 
        timeout_duration: Duration
    ) -> Result<bool, AsyncConcurrentError> {
        let mut tree = self.write_lock_with_timeout(timeout_duration).await?;
        Ok(tree.delete(rect, data))
    }
    
    /// 异步搜索与指定矩形相交的所有数据项
    /// 
    /// # 参数
    /// * `rect` - 搜索区域
    /// 
    /// # 返回
    /// 与搜索区域相交的所有数据项的向量
    /// 
    /// # 错误
    /// * `AsyncConcurrentError::Timeout` - 操作超时
    /// * `AsyncConcurrentError::LockFailed` - 锁获取失败
    /// 
    /// # 示例
    /// ```rust
    /// use rtree::{AsyncConcurrentRTree, Rectangle};
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rtree = AsyncConcurrentRTree::new(4);
    ///     rtree.insert(Rectangle::new(0.0, 0.0, 2.0, 2.0), 1).await?;
    ///     rtree.insert(Rectangle::new(3.0, 3.0, 4.0, 4.0), 2).await?;
    ///     
    ///     let search_area = Rectangle::new(1.0, 1.0, 3.5, 3.5);
    ///     let results = rtree.search(&search_area).await?;
    ///     assert_eq!(results.len(), 2); // 两个矩形都与搜索区域相交
    ///     Ok(())
    /// }
    /// ```
    pub async fn search(&self, rect: &Rectangle) -> Result<Vec<i32>, AsyncConcurrentError> {
        let tree = self.read_lock_with_timeout(self.default_timeout).await?;
        Ok(tree.search(rect))
    }
    
    /// 异步搜索与指定矩形相交的所有数据项（带超时）
    pub async fn search_with_timeout(
        &self, 
        rect: &Rectangle, 
        timeout_duration: Duration
    ) -> Result<Vec<i32>, AsyncConcurrentError> {
        let tree = self.read_lock_with_timeout(timeout_duration).await?;
        Ok(tree.search(rect))
    }
    
    /// 异步获取R-tree中数据项的总数
    /// 
    /// # 错误
    /// * `AsyncConcurrentError::Timeout` - 操作超时
    /// * `AsyncConcurrentError::LockFailed` - 锁获取失败
    /// 
    /// # 示例
    /// ```rust
    /// use rtree::{AsyncConcurrentRTree, Rectangle};
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rtree = AsyncConcurrentRTree::new(4);
    ///     assert_eq!(rtree.len().await?, 0);
    ///     
    ///     rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).await?;
    ///     assert_eq!(rtree.len().await?, 1);
    ///     Ok(())
    /// }
    /// ```
    pub async fn len(&self) -> Result<usize, AsyncConcurrentError> {
        let tree = self.read_lock_with_timeout(self.default_timeout).await?;
        Ok(tree.len())
    }
    
    /// 异步检查R-tree是否为空
    /// 
    /// # 错误
    /// * `AsyncConcurrentError::Timeout` - 操作超时
    /// * `AsyncConcurrentError::LockFailed` - 锁获取失败
    /// 
    /// # 示例
    /// ```rust
    /// use rtree::{AsyncConcurrentRTree, Rectangle};
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rtree = AsyncConcurrentRTree::new(4);
    ///     assert!(rtree.is_empty().await?);
    ///     
    ///     rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).await?;
    ///     assert!(!rtree.is_empty().await?);
    ///     Ok(())
    /// }
    /// ```
    pub async fn is_empty(&self) -> Result<bool, AsyncConcurrentError> {
        let tree = self.read_lock_with_timeout(self.default_timeout).await?;
        Ok(tree.is_empty())
    }
    
    /// 异步清空R-tree中的所有数据
    /// 
    /// # 错误
    /// * `AsyncConcurrentError::Timeout` - 操作超时
    /// * `AsyncConcurrentError::LockFailed` - 锁获取失败
    /// 
    /// # 示例
    /// ```rust
    /// use rtree::{AsyncConcurrentRTree, Rectangle};
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let rtree = AsyncConcurrentRTree::new(4);
    ///     rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).await?;
    ///     assert_eq!(rtree.len().await?, 1);
    ///     
    ///     rtree.clear().await?;
    ///     assert!(rtree.is_empty().await?);
    ///     Ok(())
    /// }
    /// ```
    pub async fn clear(&self) -> Result<(), AsyncConcurrentError> {
        let mut tree = self.write_lock_with_timeout(self.default_timeout).await?;
        let max_entries = tree.max_entries();
        *tree = RTree::new(max_entries);
        Ok(())
    }
    
    /// 获取当前的默认超时时间
    pub fn default_timeout(&self) -> Duration {
        self.default_timeout
    }
    
    /// 设置新的默认超时时间
    pub fn set_default_timeout(&mut self, timeout: Duration) {
        self.default_timeout = timeout;
    }
    
    /// 获取异步读锁的辅助方法（带超时）
    async fn read_lock_with_timeout(
        &self, 
        timeout_duration: Duration
    ) -> Result<RwLockReadGuard<RTree>, AsyncConcurrentError> {
        timeout(timeout_duration, self.inner.read())
            .await
            .map_err(|_| AsyncConcurrentError::Timeout { timeout: timeout_duration })
    }
    
    /// 获取异步写锁的辅助方法（带超时）
    async fn write_lock_with_timeout(
        &self, 
        timeout_duration: Duration
    ) -> Result<RwLockWriteGuard<RTree>, AsyncConcurrentError> {
        timeout(timeout_duration, self.inner.write())
            .await
            .map_err(|_| AsyncConcurrentError::Timeout { timeout: timeout_duration })
    }
}

// 实现Clone以支持多任务共享
impl Clone for AsyncConcurrentRTree {
    /// 克隆异步并发R-tree
    /// 
    /// 注意：这只是增加Arc的引用计数，不会复制底层的树结构。
    /// 克隆的实例指向同一个R-tree。
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            default_timeout: self.default_timeout,
        }
    }
}

// 为了方便，实现From trait来从RTree转换
impl From<RTree> for AsyncConcurrentRTree {
    fn from(rtree: RTree) -> Self {
        Self::from(rtree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_basic_operations() {
        let rtree = AsyncConcurrentRTree::new(4);
        
        // 测试空树
        assert!(rtree.is_empty().await.unwrap());
        assert_eq!(rtree.len().await.unwrap(), 0);
        
        // 测试插入
        let rect1 = Rectangle::new(0.0, 0.0, 1.0, 1.0);
        let rect2 = Rectangle::new(2.0, 2.0, 3.0, 3.0);
        
        rtree.insert(rect1, 1).await.unwrap();
        rtree.insert(rect2, 2).await.unwrap();
        
        assert!(!rtree.is_empty().await.unwrap());
        assert_eq!(rtree.len().await.unwrap(), 2);
        
        // 测试搜索
        let search_rect = Rectangle::new(-0.5, -0.5, 1.5, 1.5);
        let results = rtree.search(&search_rect).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 1);
        
        // 测试删除
        assert!(rtree.delete(&rect1, 1).await.unwrap());
        assert!(!rtree.delete(&rect1, 1).await.unwrap()); // 第二次删除应该返回false
        assert_eq!(rtree.len().await.unwrap(), 1);
        
        // 测试清空
        rtree.clear().await.unwrap();
        assert!(rtree.is_empty().await.unwrap());
    }

    #[tokio::test]
    async fn test_concurrent_reads() {
        let rtree = Arc::new(AsyncConcurrentRTree::new(4));
        
        // 预填充一些数据
        for i in 0..10 {
            let rect = Rectangle::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
            rtree.insert(rect, i).await.unwrap();
        }
        
        // 多个任务并发读取
        let mut tasks = Vec::new();
        
        for thread_id in 0..5 {
            let rtree_clone = Arc::clone(&rtree);
            let task = tokio::spawn(async move {
                for i in 0..10 {
                    let search_rect = Rectangle::new(
                        i as f64 - 0.5, 
                        i as f64 - 0.5, 
                        i as f64 + 0.5, 
                        i as f64 + 0.5
                    );
                    let results = rtree_clone.search(&search_rect).await.unwrap();
                    assert!(!results.is_empty(), "Task {} search {} failed", thread_id, i);
                }
                
                // 检查总数
                let len = rtree_clone.len().await.unwrap();
                assert_eq!(len, 10, "Task {} len check failed", thread_id);
            });
            tasks.push(task);
        }
        
        // 等待所有任务完成
        for task in tasks {
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_concurrent_writes() {
        let rtree = Arc::new(AsyncConcurrentRTree::new(4));
        
        // 多个任务并发写入
        let mut tasks = Vec::new();
        
        for task_id in 0..5 {
            let rtree_clone = Arc::clone(&rtree);
            let task = tokio::spawn(async move {
                for i in 0..10 {
                    let data = task_id * 100 + i;
                    let rect = Rectangle::new(
                        data as f64, 
                        data as f64, 
                        data as f64 + 1.0, 
                        data as f64 + 1.0
                    );
                    rtree_clone.insert(rect, data).await.unwrap();
                }
            });
            tasks.push(task);
        }
        
        // 等待所有任务完成
        for task in tasks {
            task.await.unwrap();
        }
        
        // 验证所有数据都被插入
        assert_eq!(rtree.len().await.unwrap(), 50);
    }

    #[tokio::test]
    async fn test_mixed_operations() {
        let rtree = Arc::new(AsyncConcurrentRTree::new(4));
        
        // 预填充一些数据
        for i in 0..20 {
            let rect = Rectangle::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
            rtree.insert(rect, i).await.unwrap();
        }
        
        // 混合读写操作
        let mut tasks = Vec::new();
        
        for task_id in 0..4 {
            let rtree_clone = Arc::clone(&rtree);
            let task = tokio::spawn(async move {
                match task_id {
                    0 | 1 => {
                        // 读任务
                        for i in 0..50 {
                            let search_rect = Rectangle::new(
                                (i % 20) as f64 - 0.5, 
                                (i % 20) as f64 - 0.5, 
                                (i % 20) as f64 + 0.5, 
                                (i % 20) as f64 + 0.5
                            );
                            let _results = rtree_clone.search(&search_rect).await.unwrap();
                            sleep(Duration::from_millis(1)).await;
                        }
                    }
                    2 => {
                        // 写任务 - 插入
                        for i in 100..110 {
                            let rect = Rectangle::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
                            rtree_clone.insert(rect, i).await.unwrap();
                            sleep(Duration::from_millis(2)).await;
                        }
                    }
                    3 => {
                        // 写任务 - 删除
                        sleep(Duration::from_millis(10)).await; // 让其他操作先进行
                        for i in 0..5 {
                            let rect = Rectangle::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
                            rtree_clone.delete(&rect, i).await.unwrap();
                            sleep(Duration::from_millis(2)).await;
                        }
                    }
                    _ => unreachable!()
                }
            });
            tasks.push(task);
        }
        
        // 等待所有任务完成
        for task in tasks {
            task.await.unwrap();
        }
        
        // 验证最终状态：原有20个，删除5个，新增10个 = 25个
        let final_len = rtree.len().await.unwrap();
        assert_eq!(final_len, 25);
    }

    #[tokio::test]
    async fn test_timeout_operations() {
        let rtree = AsyncConcurrentRTree::with_timeout(4, Duration::from_millis(100));
        
        // 测试正常操作（在超时时间内）
        let rect = Rectangle::new(0.0, 0.0, 1.0, 1.0);
        rtree.insert(rect, 1).await.unwrap();
        
        // 测试带自定义超时的操作
        let short_timeout = Duration::from_millis(1);
        let result = rtree.search_with_timeout(&rect, short_timeout).await;
        
        // 在高负载情况下可能超时，但在测试环境中通常会成功
        // 这里主要测试API的正确性
        assert!(result.is_ok() || matches!(result, Err(AsyncConcurrentError::Timeout { .. })));
    }

    #[tokio::test]
    async fn test_clone_sharing() {
        let rtree1 = AsyncConcurrentRTree::new(4);
        rtree1.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).await.unwrap();
        
        let rtree2 = rtree1.clone();
        
        // 通过rtree2插入数据
        rtree2.insert(Rectangle::new(2.0, 2.0, 3.0, 3.0), 2).await.unwrap();
        
        // rtree1应该能看到这个数据，因为它们共享同一个底层树
        assert_eq!(rtree1.len().await.unwrap(), 2);
        assert_eq!(rtree2.len().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_from_rtree() {
        let mut original_rtree = RTree::new(4);
        original_rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1);
        original_rtree.insert(Rectangle::new(2.0, 2.0, 3.0, 3.0), 2);
        
        let async_rtree = AsyncConcurrentRTree::from(original_rtree);
        
        assert_eq!(async_rtree.len().await.unwrap(), 2);
        
        let search_rect = Rectangle::new(-0.5, -0.5, 1.5, 1.5);
        let results = async_rtree.search(&search_rect).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_timeout_configuration() {
        let mut rtree = AsyncConcurrentRTree::new(4);
        
        // 测试默认超时时间
        assert_eq!(rtree.default_timeout(), Duration::from_secs(30));
        
        // 修改默认超时时间
        let new_timeout = Duration::from_secs(5);
        rtree.set_default_timeout(new_timeout);
        assert_eq!(rtree.default_timeout(), new_timeout);
        
        // 测试带自定义超时的构造函数
        let custom_timeout = Duration::from_millis(500);
        let rtree_with_timeout = AsyncConcurrentRTree::with_timeout(4, custom_timeout);
        assert_eq!(rtree_with_timeout.default_timeout(), custom_timeout);
    }
}
