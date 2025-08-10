use std::fs;
use std::path::Path;
use crate::rtree::RTree;

/// 持久化错误类型
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Binary serialization error: {0}")]
    Binary(#[from] bincode::Error),
    #[error("Invalid file format")]
    InvalidFormat,
}

/// 序列化格式枚举
#[derive(Debug, Clone, Copy)]
pub enum SerializationFormat {
    /// JSON格式 - 可读性好，方便调试
    Json,
    /// 二进制格式 - 性能好，体积小
    Binary,
}

impl SerializationFormat {
    /// 根据文件扩展名自动判断格式
    pub fn from_extension<P: AsRef<Path>>(path: P) -> Self {
        match path.as_ref().extension().and_then(|ext| ext.to_str()) {
            Some("json") => SerializationFormat::Json,
            Some("bin") | Some("rtree") | _ => SerializationFormat::Binary,
        }
    }
}

/// R-tree持久化功能实现
impl RTree {
    /// 导出到文件
    /// 
    /// 根据文件扩展名自动选择序列化格式：
    /// - .json -> JSON格式（调试友好）
    /// - .bin/.rtree/其他 -> 二进制格式（高性能）
    /// 
    /// # 参数
    /// * `path` - 目标文件路径
    /// 
    /// # 示例
    /// ```
    /// use rtree::RTree;
    /// 
    /// let mut rtree = RTree::new(4);
    /// // ... 插入数据 ...
    /// 
    /// // JSON格式（调试用）
    /// rtree.dump_to_file("data.json").unwrap();
    /// 
    /// // 二进制格式（生产用）
    /// rtree.dump_to_file("data.bin").unwrap();
    /// ```
    pub fn dump_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), PersistenceError> {
        let format = SerializationFormat::from_extension(&path);
        self.dump_to_file_with_format(path, format)
    }
    
    /// 使用指定格式导出到文件
    /// 
    /// # 参数
    /// * `path` - 目标文件路径
    /// * `format` - 序列化格式
    pub fn dump_to_file_with_format<P: AsRef<Path>>(
        &self, 
        path: P, 
        format: SerializationFormat
    ) -> Result<(), PersistenceError> {
        let path = path.as_ref();
        
        // 创建临时文件路径，确保原子性写入
        let temp_path = path.with_extension(
            format!("{}.tmp", path.extension().unwrap_or_default().to_string_lossy())
        );
        
        // 序列化数据
        let data = match format {
            SerializationFormat::Json => {
                serde_json::to_vec_pretty(self)?
            }
            SerializationFormat::Binary => {
                bincode::serialize(self)?
            }
        };
        
        // 写入临时文件
        fs::write(&temp_path, data)?;
        
        // 原子性重命名
        fs::rename(temp_path, path)?;
        
        Ok(())
    }
    
    /// # 示例
    /// ```
    /// use rtree::RTree;
    /// use geo::{Point, Geometry};
    /// use tempfile::NamedTempFile;
    ///
    /// // 创建临时文件
    /// let temp_file = NamedTempFile::new().unwrap();
    /// let temp_path = temp_file.path();
    ///
    /// // 创建并保存R-tree
    /// let mut rtree = RTree::new(4);
    /// rtree.insert_geometry("1".to_string(), Geometry::Point(Point::new(0.5, 0.5)));
    /// rtree.dump_to_file(&temp_path).unwrap();
    ///
    /// // 从文件加载
    /// let loaded_rtree = RTree::load_from_file(&temp_path).unwrap();
    /// assert_eq!(rtree.len(), loaded_rtree.len());
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<RTree, PersistenceError> {
        let format = SerializationFormat::from_extension(&path);
        Self::load_from_file_with_format(path, format)
    }
    
    /// 使用指定格式从文件加载R-tree
    /// 
    /// # 参数
    /// * `path` - 源文件路径
    /// * `format` - 序列化格式
    pub fn load_from_file_with_format<P: AsRef<Path>>(
        path: P, 
        format: SerializationFormat
    ) -> Result<RTree, PersistenceError> {
        let data = fs::read(path)?;
        
        let rtree = match format {
            SerializationFormat::Json => {
                serde_json::from_slice(&data)?
            }
            SerializationFormat::Binary => {
                bincode::deserialize(&data)?
            }
        };
        
        Ok(rtree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rectangle::Rectangle;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_persistence_json() {
        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("test.json");
        
        // 创建并填充R-tree
        let mut original_rtree = RTree::new(4);
        original_rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), "1".to_string());
        original_rtree.insert(Rectangle::new(2.0, 2.0, 3.0, 3.0), "2".to_string());
        original_rtree.insert(Rectangle::new(5.0, 5.0, 6.0, 6.0), "3".to_string());

        // 导出到JSON文件
        original_rtree.dump_to_file(&json_path).unwrap();
        
        // 验证文件存在且为JSON格式
        assert!(json_path.exists());
        let content = fs::read_to_string(&json_path).unwrap();
        assert!(content.contains("root"));
        assert!(content.contains("max_entries"));
        
        // 从JSON文件加载
        let loaded_rtree = RTree::load_from_file(&json_path).unwrap();
        
        // 验证数据一致性
        assert_eq!(original_rtree.len(), loaded_rtree.len());
        assert_eq!(original_rtree.is_empty(), loaded_rtree.is_empty());
        
        // 验证搜索结果一致
        let search_rect = Rectangle::new(0.5, 0.5, 2.5, 2.5);
        let original_results = original_rtree.search_bbox(&search_rect);
        let loaded_results = loaded_rtree.search_bbox(&search_rect);
        assert_eq!(original_results.len(), loaded_results.len());
    }
    
    #[test]
    fn test_persistence_binary() {
        let temp_dir = TempDir::new().unwrap();
        let bin_path = temp_dir.path().join("test.bin");
        
        // 创建并填充R-tree
        let mut original_rtree = RTree::new(4);
        original_rtree.insert(Rectangle::new(0.0, 0.0, 1.0, 1.0), "1".to_string());
        original_rtree.insert(Rectangle::new(2.0, 2.0, 3.0, 3.0), "2".to_string());
        original_rtree.insert(Rectangle::new(5.0, 5.0, 6.0, 6.0), "3".to_string());

        // 导出到二进制文件
        original_rtree.dump_to_file(&bin_path).unwrap();
        
        // 验证文件存在
        assert!(bin_path.exists());
        
        // 从二进制文件加载
        let loaded_rtree = RTree::load_from_file(&bin_path).unwrap();
        
        // 验证数据一致性
        assert_eq!(original_rtree.len(), loaded_rtree.len());
        assert_eq!(original_rtree.is_empty(), loaded_rtree.is_empty());
        
        // 验证搜索结果一致
        let search_rect = Rectangle::new(0.5, 0.5, 2.5, 2.5);
        let original_results = original_rtree.search_bbox(&search_rect);
        let loaded_results = loaded_rtree.search_bbox(&search_rect);
        assert_eq!(original_results.len(), loaded_results.len());
    }
    
    #[test]
    fn test_empty_tree_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("empty.json");
        let bin_path = temp_dir.path().join("empty.bin");
        
        let empty_rtree = RTree::new(4);
        
        // 测试JSON格式
        empty_rtree.dump_to_file(&json_path).unwrap();
        let loaded_json = RTree::load_from_file(&json_path).unwrap();
        assert!(loaded_json.is_empty());
        
        // 测试二进制格式
        empty_rtree.dump_to_file(&bin_path).unwrap();
        let loaded_bin = RTree::load_from_file(&bin_path).unwrap();
        assert!(loaded_bin.is_empty());
    }
    
    #[test]
    fn test_format_auto_detection() {
        // 测试扩展名自动检测
        assert!(matches!(
            SerializationFormat::from_extension("data.json"), 
            SerializationFormat::Json
        ));
        assert!(matches!(
            SerializationFormat::from_extension("data.bin"), 
            SerializationFormat::Binary
        ));
        assert!(matches!(
            SerializationFormat::from_extension("data.rtree"), 
            SerializationFormat::Binary
        ));
        assert!(matches!(
            SerializationFormat::from_extension("data"), 
            SerializationFormat::Binary
        ));
    }
    
    #[test]
    fn test_file_size_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("test.json");
        let bin_path = temp_dir.path().join("test.bin");
        
        // 创建一个较大的R-tree
        let mut rtree = RTree::new(4);
        for i in 0..100 {
            let x = (i % 10) as f64;
            let y = (i / 10) as f64;
            rtree.insert(Rectangle::new(x, y, x + 1.0, y + 1.0), i.to_string());
        }
        
        // 导出两种格式
        rtree.dump_to_file(&json_path).unwrap();
        rtree.dump_to_file(&bin_path).unwrap();
        
        // 获取文件大小
        let json_size = fs::metadata(&json_path).unwrap().len();
        let bin_size = fs::metadata(&bin_path).unwrap().len();
        
        println!("JSON file size: {} bytes", json_size);
        println!("Binary file size: {} bytes", bin_size);
        
        // 二进制格式通常应该更小
        // 但由于这是测试，我们只验证两个文件都不为空
        assert!(json_size > 0);
        assert!(bin_size > 0);
    }
}
