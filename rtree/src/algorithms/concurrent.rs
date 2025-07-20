use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::rtree::RTree;
use crate::rectangle::Rectangle;

/// 并发错误类型
#[derive(Debug, thiserror::Error)]
pub enum ConcurrentError {
    #[error("Lock was poisoned by a panicked thread")]
    LockPoisoned,
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

/// 并发安全的R-tree实现
/// 
/// 这个结构体包装了标准的RTree，提供线程安全的操作接口。
/// 使用读写锁来协调并发访问：
/// - 读操作（search, len, is_empty）可以并发执行
/// - 写操作（insert, delete）需要独占访问
/// 
/// # 示例
/// 
/// ```
/// use rtree::{ConcurrentRTree, Rectangle};
/// use std::thread;
/// 
/// let rtree = ConcurrentRTree::new(4);
/// 
/// // 在多个线程中并发插入
/// let handles: Vec<_> = (0..4).map(|i| {
///     let rtree_clone = rtree.clone(); // 通过clone共享同一个树
///     thread::spawn(move || {
///         let rect = Rectangle::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
///         rtree_clone.insert(rect, i).unwrap();
///     })
/// }).collect();
/// 
/// // 等待所有线程完成
/// for handle in handles {
///     handle.join().unwrap();
/// }
/// 
/// assert_eq!(rtree.len().unwrap(), 4);
/// ```
#[derive(Debug)]
pub struct ConcurrentRTree {
    inner: Arc<RwLock<RTree>>,
}

impl ConcurrentRTree {
    /// 创建新的并发R-tree
    /// 
    /// # 参数
    /// * `max_entries` - 每个节点的最大条目数
    /// 
    /// # 示例
    /// ```
    /// use rtree::ConcurrentRTree;
    /// 
    /// let rtree = ConcurrentRTree::new(4);
    /// ```
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(RTree::new(max_entries))),
        }
    }
    
    /// 从现有的RTree创建并发版本
    /// 
    /// # 参数
    /// * `rtree` - 要包装的RTree实例
    /// 
    /// # 示例
    /// ```
    /// use rtree::{RTree, ConcurrentRTree, Rectangle};
    /// 
    /// let mut rtree = RTree::new(4);
    /// rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1);
    /// 
    /// let concurrent_rtree = ConcurrentRTree::from(rtree);
    /// assert_eq!(concurrent_rtree.len().unwrap(), 1);
    /// ```
    pub fn from(rtree: RTree) -> Self {
        Self {
            inner: Arc::new(RwLock::new(rtree)),
        }
    }
    
    /// 插入新的数据项
    /// 
    /// # 参数
    /// * `rect` - 数据项的边界矩形
    /// * `data` - 要存储的数据
    /// 
    /// # 错误
    /// 如果锁被毒化，返回 `ConcurrentError::LockPoisoned`
    /// 
    /// # 示例
    /// ```
    /// use rtree::{ConcurrentRTree, Rectangle};
    /// 
    /// let rtree = ConcurrentRTree::new(4);
    /// rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).unwrap();
    /// assert_eq!(rtree.len().unwrap(), 1);
    /// ```
    pub fn insert(&self, rect: Rectangle, data: i32) -> Result<(), ConcurrentError> {
        let mut tree = self.write_lock()?;
        tree.insert(rect, data);
        Ok(())
    }
    
    /// 删除指定的数据项
    /// 
    /// # 参数
    /// * `rect` - 要删除的数据项的边界矩形
    /// * `data` - 要删除的数据
    /// 
    /// # 返回
    /// * `Ok(true)` - 成功删除
    /// * `Ok(false)` - 未找到要删除的项
    /// * `Err(_)` - 锁错误
    /// 
    /// # 示例
    /// ```
    /// use rtree::{ConcurrentRTree, Rectangle};
    /// 
    /// let rtree = ConcurrentRTree::new(4);
    /// let rect = Rectangle::new(0.0, 0.0, 1.0, 1.0);
    /// 
    /// rtree.insert(rect, 1).unwrap();
    /// assert!(rtree.delete(&rect, 1).unwrap());
    /// assert!(!rtree.delete(&rect, 1).unwrap()); // 第二次删除返回false
    /// ```
    pub fn delete(&self, rect: &Rectangle, data: i32) -> Result<bool, ConcurrentError> {
        let mut tree = self.write_lock()?;
        Ok(tree.delete(rect, data))
    }
    
    /// 搜索与指定矩形相交的所有数据项
    /// 
    /// # 参数
    /// * `rect` - 搜索区域
    /// 
    /// # 返回
    /// 与搜索区域相交的所有数据项的向量
    /// 
    /// # 错误
    /// 如果锁被毒化，返回 `ConcurrentError::LockPoisoned`
    /// 
    /// # 示例
    /// ```
    /// use rtree::{ConcurrentRTree, Rectangle};
    /// 
    /// let rtree = ConcurrentRTree::new(4);
    /// rtree.insert(Rectangle::new(0.0, 0.0, 2.0, 2.0), 1).unwrap();
    /// rtree.insert(Rectangle::new(3.0, 3.0, 4.0, 4.0), 2).unwrap();
    /// 
    /// let search_area = Rectangle::new(1.0, 1.0, 3.5, 3.5);
    /// let results = rtree.search(&search_area).unwrap();
    /// assert_eq!(results.len(), 2); // 两个矩形都与搜索区域相交
    /// ```
    pub fn search(&self, rect: &Rectangle) -> Result<Vec<i32>, ConcurrentError> {
        let tree = self.read_lock()?;
        Ok(tree.search(rect))
    }
    
    /// 获取R-tree中数据项的总数
    /// 
    /// # 错误
    /// 如果锁被毒化，返回 `ConcurrentError::LockPoisoned`
    /// 
    /// # 示例
    /// ```
    /// use rtree::{ConcurrentRTree, Rectangle};
    /// 
    /// let rtree = ConcurrentRTree::new(4);
    /// assert_eq!(rtree.len().unwrap(), 0);
    /// 
    /// rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).unwrap();
    /// assert_eq!(rtree.len().unwrap(), 1);
    /// ```
    pub fn len(&self) -> Result<usize, ConcurrentError> {
        let tree = self.read_lock()?;
        Ok(tree.len())
    }
    
    /// 检查R-tree是否为空
    /// 
    /// # 错误
    /// 如果锁被毒化，返回 `ConcurrentError::LockPoisoned`
    /// 
    /// # 示例
    /// ```
    /// use rtree::{ConcurrentRTree, Rectangle};
    /// 
    /// let rtree = ConcurrentRTree::new(4);
    /// assert!(rtree.is_empty().unwrap());
    /// 
    /// rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).unwrap();
    /// assert!(!rtree.is_empty().unwrap());
    /// ```
    pub fn is_empty(&self) -> Result<bool, ConcurrentError> {
        let tree = self.read_lock()?;
        Ok(tree.is_empty())
    }
    
    /// 清空R-tree中的所有数据
    /// 
    /// # 错误
    /// 如果锁被毒化，返回 `ConcurrentError::LockPoisoned`
    /// 
    /// # 示例
    /// ```
    /// use rtree::{ConcurrentRTree, Rectangle};
    /// 
    /// let rtree = ConcurrentRTree::new(4);
    /// rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).unwrap();
    /// assert_eq!(rtree.len().unwrap(), 1);
    /// 
    /// rtree.clear().unwrap();
    /// assert!(rtree.is_empty().unwrap());
    /// ```
    pub fn clear(&self) -> Result<(), ConcurrentError> {
        let mut tree = self.write_lock()?;
        *tree = RTree::new(tree.max_entries());
        Ok(())
    }
    
    /// 获取读锁的辅助方法
    fn read_lock(&self) -> Result<RwLockReadGuard<RTree>, ConcurrentError> {
        self.inner.read().map_err(|_| ConcurrentError::LockPoisoned)
    }
    
    /// 获取写锁的辅助方法
    fn write_lock(&self) -> Result<RwLockWriteGuard<RTree>, ConcurrentError> {
        self.inner.write().map_err(|_| ConcurrentError::LockPoisoned)
    }
}

// 实现Clone以支持多线程共享
impl Clone for ConcurrentRTree {
    /// 克隆并发R-tree
    /// 
    /// 注意：这只是增加Arc的引用计数，不会复制底层的树结构。
    /// 克隆的实例指向同一个R-tree。
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// 为了方便，实现From trait来从RTree转换
impl From<RTree> for ConcurrentRTree {
    fn from(rtree: RTree) -> Self {
        Self::from(rtree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_operations() {
        let rtree = ConcurrentRTree::new(4);
        
        // 测试空树
        assert!(rtree.is_empty().unwrap());
        assert_eq!(rtree.len().unwrap(), 0);
        
        // 测试插入
        let rect1 = Rectangle::new(0.0, 0.0, 1.0, 1.0);
        let rect2 = Rectangle::new(2.0, 2.0, 3.0, 3.0);
        
        rtree.insert(rect1, 1).unwrap();
        rtree.insert(rect2, 2).unwrap();
        
        assert!(!rtree.is_empty().unwrap());
        assert_eq!(rtree.len().unwrap(), 2);
        
        // 测试搜索
        let search_rect = Rectangle::new(-0.5, -0.5, 1.5, 1.5);
        let results = rtree.search(&search_rect).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], 1);
        
        // 测试删除
        assert!(rtree.delete(&rect1, 1).unwrap());
        assert!(!rtree.delete(&rect1, 1).unwrap()); // 第二次删除应该返回false
        assert_eq!(rtree.len().unwrap(), 1);
        
        // 测试清空
        rtree.clear().unwrap();
        assert!(rtree.is_empty().unwrap());
    }

    #[test]
    fn test_concurrent_reads() {
        let rtree = Arc::new(ConcurrentRTree::new(4));
        
        // 预填充一些数据
        for i in 0..10 {
            let rect = Rectangle::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
            rtree.insert(rect, i).unwrap();
        }
        
        // 多个线程并发读取
        let handles: Vec<_> = (0..5).map(|thread_id| {
            let rtree_clone = Arc::clone(&rtree);
            thread::spawn(move || {
                for i in 0..10 {
                    let search_rect = Rectangle::new(i as f64 - 0.5, i as f64 - 0.5, i as f64 + 0.5, i as f64 + 0.5);
                    let results = rtree_clone.search(&search_rect).unwrap();
                    assert!(!results.is_empty(), "Thread {} search {} failed", thread_id, i);
                }
                
                // 检查总数
                let len = rtree_clone.len().unwrap();
                assert_eq!(len, 10, "Thread {} len check failed", thread_id);
            })
        }).collect();
        
        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_writes() {
        let rtree = Arc::new(ConcurrentRTree::new(4));
        
        // 多个线程并发写入
        let handles: Vec<_> = (0..5).map(|thread_id| {
            let rtree_clone = Arc::clone(&rtree);
            thread::spawn(move || {
                for i in 0..10 {
                    let data = thread_id * 100 + i;
                    let rect = Rectangle::new(
                        data as f64, 
                        data as f64, 
                        data as f64 + 1.0, 
                        data as f64 + 1.0
                    );
                    rtree_clone.insert(rect, data).unwrap();
                }
            })
        }).collect();
        
        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }
        
        // 验证所有数据都被插入
        assert_eq!(rtree.len().unwrap(), 50);
    }

    #[test]
    fn test_mixed_operations() {
        let rtree = Arc::new(ConcurrentRTree::new(4));
        
        // 预填充一些数据
        for i in 0..20 {
            let rect = Rectangle::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
            rtree.insert(rect, i).unwrap();
        }
        
        // 混合读写操作
        let handles: Vec<_> = (0..4).map(|thread_id| {
            let rtree_clone = Arc::clone(&rtree);
            thread::spawn(move || {
                match thread_id {
                    0 | 1 => {
                        // 读线程
                        for i in 0..50 {
                            let search_rect = Rectangle::new(
                                (i % 20) as f64 - 0.5, 
                                (i % 20) as f64 - 0.5, 
                                (i % 20) as f64 + 0.5, 
                                (i % 20) as f64 + 0.5
                            );
                            let _results = rtree_clone.search(&search_rect).unwrap();
                            thread::sleep(Duration::from_millis(1));
                        }
                    }
                    2 => {
                        // 写线程 - 插入
                        for i in 100..110 {
                            let rect = Rectangle::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
                            rtree_clone.insert(rect, i).unwrap();
                            thread::sleep(Duration::from_millis(2));
                        }
                    }
                    3 => {
                        // 写线程 - 删除
                        thread::sleep(Duration::from_millis(10)); // 让其他操作先进行
                        for i in 0..5 {
                            let rect = Rectangle::new(i as f64, i as f64, i as f64 + 1.0, i as f64 + 1.0);
                            rtree_clone.delete(&rect, i).unwrap();
                            thread::sleep(Duration::from_millis(2));
                        }
                    }
                    _ => unreachable!()
                }
            })
        }).collect();
        
        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }
        
        // 验证最终状态：原有20个，删除5个，新增10个 = 25个
        let final_len = rtree.len().unwrap();
        assert_eq!(final_len, 25);
    }

    #[test]
    fn test_clone_sharing() {
        let rtree1 = ConcurrentRTree::new(4);
        rtree1.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1).unwrap();
        
        let rtree2 = rtree1.clone();
        
        // 通过rtree2插入数据
        rtree2.insert(Rectangle::new(2.0, 2.0, 3.0, 3.0), 2).unwrap();
        
        // rtree1应该能看到这个数据，因为它们共享同一个底层树
        assert_eq!(rtree1.len().unwrap(), 2);
        assert_eq!(rtree2.len().unwrap(), 2);
    }

    #[test]
    fn test_from_rtree() {
        let mut original_rtree = RTree::new(4);
        original_rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), 1);
        original_rtree.insert(Rectangle::new(2.0, 2.0, 3.0, 3.0), 2);
        
        let concurrent_rtree = ConcurrentRTree::from(original_rtree);
        
        assert_eq!(concurrent_rtree.len().unwrap(), 2);
        
        let search_rect = Rectangle::new(-0.5, -0.5, 1.5, 1.5);
        let results = concurrent_rtree.search(&search_rect).unwrap();
        assert_eq!(results.len(), 1);
    }
}
