//! AOF (Append-Only File) 持久化模块
//!
//! 实现基于 JSON Lines 格式的追加式日志持久化系统，支持：
//! - 写入命令到 AOF 文件
//! - 从 AOF 文件恢复数据
//! - 三种同步策略（Always、EverySecond、No）
//! - 容错恢复机制

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

// ============================================================================
// 错误类型
// ============================================================================

/// AOF 操作相关的错误类型
#[derive(Debug, Error)]
pub enum AofError {
    /// IO 错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON 序列化错误
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// AOF 文件不存在
    #[error("AOF file not found")]
    FileNotFound,

    /// 无效的命令（包含行号和原因）
    #[error("Invalid command at line {line}: {reason}")]
    InvalidCommand { line: usize, reason: String },

    /// AOF 功能被禁用
    #[error("AOF is disabled")]
    Disabled,
}

// ============================================================================
// 同步策略
// ============================================================================

/// AOF 同步策略
///
/// 决定何时将数据从内存缓冲区刷新到磁盘
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AofSyncPolicy {
    /// 每次写入都立即 fsync（最安全，性能最低）
    ///
    /// - 数据丢失风险：几乎为 0
    /// - 适用场景：金融系统、关键业务数据
    /// - 预期性能：~1000-5000 TPS
    Always,

    /// 每秒 fsync 一次（推荐，平衡性能和安全性）
    ///
    /// - 数据丢失风险：最多 1 秒的数据
    /// - 适用场景：大多数生产环境
    /// - 预期性能：~50000-100000 TPS
    /// - 默认策略（与 Redis 保持一致）
    #[default]
    EverySecond,

    /// 不主动 fsync，由操作系统决定（性能最高，最不安全）
    ///
    /// - 数据丢失风险：最多 30 秒的数据
    /// - 适用场景：可容忍数据丢失的缓存场景
    /// - 预期性能：~100000+ TPS
    No,
}

// ============================================================================
// AOF 配置
// ============================================================================

/// AOF 配置
#[derive(Debug, Clone)]
pub struct AofConfig {
    /// AOF 文件路径
    pub file_path: PathBuf,

    /// 同步策略
    pub sync_policy: AofSyncPolicy,

    /// 是否启用 AOF（可以临时关闭）
    pub enabled: bool,
}

impl Default for AofConfig {
    fn default() -> Self {
        Self {
            file_path: PathBuf::from("data/appendonly.aof"),
            sync_policy: AofSyncPolicy::EverySecond,
            enabled: true,
        }
    }
}

impl AofConfig {
    /// 创建新的 AOF 配置
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            ..Default::default()
        }
    }

    /// 设置同步策略
    pub fn set_sync_policy(mut self, policy: AofSyncPolicy) -> Self {
        self.sync_policy = policy;
        self
    }

    /// 设置是否启用
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

// ============================================================================
// AOF 命令
// ============================================================================

/// AOF 命令（JSON Lines 格式）
///
/// 每条命令都会被序列化为一行 JSON 并追加到 AOF 文件中
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "cmd", rename_all = "UPPERCASE")]
pub enum AofCommand {
    /// 插入命令
    Insert {
        /// 时间戳（纳秒）
        ts: u64,
        /// 集合名称
        collection: String,
        /// 对象 key
        key: String,
        /// 边界框 [min_x, min_y, max_x, max_y]
        bbox: [f64; 4],
        /// GeoJSON 数据
        geojson: String,
    },

    /// 删除命令
    Delete {
        /// 时间戳（纳秒）
        ts: u64,
        /// 集合名称
        collection: String,
        /// 对象 key
        key: String,
        /// 边界框 [min_x, min_y, max_x, max_y]
        bbox: [f64; 4],
    },

    /// 删除集合命令
    Drop {
        /// 时间戳（纳秒）
        ts: u64,
        /// 集合名称
        collection: String,
    },
}

impl AofCommand {
    /// 获取命令的时间戳
    pub fn timestamp(&self) -> u64 {
        match self {
            Self::Insert { ts, .. } => *ts,
            Self::Delete { ts, .. } => *ts,
            Self::Drop { ts, .. } => *ts,
        }
    }

    /// 获取命令关联的集合名称
    pub fn collection(&self) -> &str {
        match self {
            Self::Insert { collection, .. } => collection,
            Self::Delete { collection, .. } => collection,
            Self::Drop { collection, .. } => collection,
        }
    }

    /// 生成当前时间戳（纳秒）
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// 创建 INSERT 命令
    ///
    /// # 参数
    /// * `collection` - 集合名称
    /// * `key` - 对象 key
    /// * `bbox` - 边界框 [min_x, min_y, max_x, max_y]
    /// * `geojson` - GeoJSON 数据
    pub fn insert(
        collection: String,
        key: String,
        bbox: [f64; 4],
        geojson: String,
    ) -> Self {
        Self::Insert {
            ts: Self::now(),
            collection,
            key,
            bbox,
            geojson,
        }
    }

    /// 创建 DELETE 命令
    ///
    /// # 参数
    /// * `collection` - 集合名称
    /// * `key` - 对象 key
    /// * `bbox` - 边界框 [min_x, min_y, max_x, max_y]
    pub fn delete(collection: String, key: String, bbox: [f64; 4]) -> Self {
        Self::Delete {
            ts: Self::now(),
            collection,
            key,
            bbox,
        }
    }

    /// 创建 DROP 命令
    ///
    /// # 参数
    /// * `collection` - 集合名称
    pub fn drop(collection: String) -> Self {
        Self::Drop {
            ts: Self::now(),
            collection,
        }
    }
}

// ============================================================================
// AOF Writer
// ============================================================================

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::time::Instant;

/// AOF 写入器
///
/// 负责将命令追加到 AOF 文件中，支持三种同步策略
pub struct AofWriter {
    writer: BufWriter<File>,
    config: AofConfig,
    last_sync: Instant,
    bytes_written: u64,
}

impl AofWriter {
    /// 创建新的 AOF Writer
    ///
    /// # 参数
    /// * `config` - AOF 配置
    ///
    /// # 错误
    /// - 如果 AOF 被禁用，返回 `AofError::Disabled`
    /// - 如果无法创建目录或打开文件，返回 IO 错误
    ///
    /// # 示例
    /// ```
    /// use spatio::rtree::algorithms::aof::{AofConfig, AofWriter};
    /// use std::path::PathBuf;
    ///
    /// let config = AofConfig::new(PathBuf::from("test.aof"));
    /// let writer = AofWriter::new(config).unwrap();
    /// ```
    pub fn new(config: AofConfig) -> Result<Self, AofError> {
        if !config.enabled {
            return Err(AofError::Disabled);
        }

        // 创建目录（如果不存在）
        if let Some(parent) = config.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 打开文件（追加模式）
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.file_path)?;

        Ok(Self {
            writer: BufWriter::new(file),
            config,
            last_sync: Instant::now(),
            bytes_written: 0,
        })
    }

    /// 追加命令到 AOF
    ///
    /// 将命令序列化为 JSON Lines 格式并写入文件，根据同步策略决定是否立即同步到磁盘
    ///
    /// # 参数
    /// * `cmd` - 要追加的命令
    ///
    /// # 错误
    /// - JSON 序列化错误
    /// - IO 写入错误
    /// - fsync 错误
    ///
    /// # 示例
    /// ```no_run
    /// use spatio::rtree::algorithms::aof::{AofCommand, AofConfig, AofWriter};
    /// use std::path::PathBuf;
    ///
    /// let config = AofConfig::new(PathBuf::from("test.aof"));
    /// let mut writer = AofWriter::new(config).unwrap();
    ///
    /// let cmd = AofCommand::insert(
    ///     "cities".to_string(),
    ///     "beijing".to_string(),
    ///     [116.0, 39.0, 117.0, 40.0],
    ///     r#"{"type":"Point"}"#.to_string(),
    /// );
    ///
    /// writer.append(&cmd).unwrap();
    /// ```
    pub fn append(&mut self, cmd: &AofCommand) -> Result<(), AofError> {
        // 序列化为 JSON（单行，不换行）
        let json = serde_json::to_string(cmd)?;

        // 写入一行（JSON + \n）
        writeln!(self.writer, "{}", json)?;

        self.bytes_written += (json.len() + 1) as u64;

        // 根据同步策略决定是否 fsync
        self.sync_if_needed()?;

        Ok(())
    }

    /// 根据策略执行同步
    ///
    /// - `Always`: 立即 flush 并 fsync
    /// - `EverySecond`: 每秒 flush 并 fsync
    /// - `No`: 每 1MB flush（不 fsync）
    fn sync_if_needed(&mut self) -> Result<(), AofError> {
        match self.config.sync_policy {
            AofSyncPolicy::Always => {
                // 立即刷新并同步到磁盘
                self.writer.flush()?;
                self.writer.get_ref().sync_data()?;
            }
            AofSyncPolicy::EverySecond => {
                // 每秒同步一次
                if self.last_sync.elapsed().as_secs() >= 1 {
                    self.writer.flush()?;
                    self.writer.get_ref().sync_data()?;
                    self.last_sync = Instant::now();
                }
            }
            AofSyncPolicy::No => {
                // 每 1MB 刷新一次缓冲区（但不 fsync）
                if self.bytes_written % (1024 * 1024) == 0 {
                    self.writer.flush()?;
                }
            }
        }
        Ok(())
    }

    /// 手动刷新缓冲区并同步到磁盘
    ///
    /// 用于确保所有数据都写入磁盘，通常在关闭前调用
    ///
    /// # 示例
    /// ```no_run
    /// use spatio::rtree::algorithms::aof::{AofConfig, AofWriter};
    /// use std::path::PathBuf;
    ///
    /// let config = AofConfig::new(PathBuf::from("test.aof"));
    /// let mut writer = AofWriter::new(config).unwrap();
    /// // ... 写入数据 ...
    /// writer.flush().unwrap();
    /// ```
    pub fn flush(&mut self) -> Result<(), AofError> {
        self.writer.flush()?;
        self.writer.get_ref().sync_all()?;
        Ok(())
    }

    /// 获取已写入的字节数
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// 获取配置的引用
    pub fn config(&self) -> &AofConfig {
        &self.config
    }
}

impl Drop for AofWriter {
    /// 析构时自动刷新缓冲区
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aof_command_insert_creation() {
        let cmd = AofCommand::insert(
            "cities".to_string(),
            "beijing".to_string(),
            [116.0, 39.0, 117.0, 40.0],
            r#"{"type":"Point","coordinates":[116.4,39.9]}"#.to_string(),
        );

        assert!(matches!(cmd, AofCommand::Insert { .. }));
        assert_eq!(cmd.collection(), "cities");
        assert!(cmd.timestamp() > 0);
    }

    #[test]
    fn test_aof_command_delete_creation() {
        let cmd = AofCommand::delete(
            "cities".to_string(),
            "beijing".to_string(),
            [116.0, 39.0, 117.0, 40.0],
        );

        assert!(matches!(cmd, AofCommand::Delete { .. }));
        assert_eq!(cmd.collection(), "cities");
        assert!(cmd.timestamp() > 0);
    }

    #[test]
    fn test_aof_command_drop_creation() {
        let cmd = AofCommand::drop("cities".to_string());

        assert!(matches!(cmd, AofCommand::Drop { .. }));
        assert_eq!(cmd.collection(), "cities");
        assert!(cmd.timestamp() > 0);
    }

    #[test]
    fn test_aof_command_json_serialization() {
        let cmd = AofCommand::insert(
            "test".to_string(),
            "key1".to_string(),
            [0.0, 0.0, 1.0, 1.0],
            "{}".to_string(),
        );

        // 序列化为 JSON
        let json = serde_json::to_string(&cmd).unwrap();

        // 验证 JSON 格式
        assert!(json.contains(r#""cmd":"INSERT""#));
        assert!(json.contains(r#""collection":"test""#));
        assert!(json.contains(r#""key":"key1""#));
        assert!(json.contains(r#""bbox""#));
        assert!(json.contains(r#""geojson""#));
        assert!(json.contains(r#""ts""#));

        // 反序列化
        let deserialized: AofCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.collection(), "test");
    }

    #[test]
    fn test_aof_command_json_deserialization() {
        let json = r#"{"cmd":"INSERT","ts":1698234567890123456,"collection":"cities","key":"beijing","bbox":[116.0,39.0,117.0,40.0],"geojson":"{\"type\":\"Point\",\"coordinates\":[116.4,39.9]}"}"#;

        let cmd: AofCommand = serde_json::from_str(json).unwrap();

        assert!(matches!(cmd, AofCommand::Insert { .. }));
        assert_eq!(cmd.collection(), "cities");
        assert_eq!(cmd.timestamp(), 1698234567890123456);

        if let AofCommand::Insert {
            key,
            bbox,
            geojson,
            ..
        } = cmd
        {
            assert_eq!(key, "beijing");
            assert_eq!(bbox, [116.0, 39.0, 117.0, 40.0]);
            assert!(geojson.contains("Point"));
        }
    }

    #[test]
    fn test_aof_command_all_types_serialization() {
        let commands = vec![
            AofCommand::insert(
                "test".to_string(),
                "key1".to_string(),
                [0.0, 0.0, 1.0, 1.0],
                "{}".to_string(),
            ),
            AofCommand::delete("test".to_string(), "key1".to_string(), [0.0, 0.0, 1.0, 1.0]),
            AofCommand::drop("test".to_string()),
        ];

        for cmd in commands {
            let json = serde_json::to_string(&cmd).unwrap();
            let deserialized: AofCommand = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, deserialized);
        }
    }

    #[test]
    fn test_aof_sync_policy_default() {
        let policy = AofSyncPolicy::default();
        assert_eq!(policy, AofSyncPolicy::EverySecond);
    }

    #[test]
    fn test_aof_config_default() {
        let config = AofConfig::default();
        assert_eq!(config.file_path, PathBuf::from("data/appendonly.aof"));
        assert_eq!(config.sync_policy, AofSyncPolicy::EverySecond);
        assert!(config.enabled);
    }

    #[test]
    fn test_aof_config_builder() {
        let config = AofConfig::new(PathBuf::from("custom.aof"))
            .set_sync_policy(AofSyncPolicy::Always)
            .with_enabled(false);

        assert_eq!(config.file_path, PathBuf::from("custom.aof"));
        assert_eq!(config.sync_policy, AofSyncPolicy::Always);
        assert!(!config.enabled);
    }

    #[test]
    fn test_aof_error_display() {
        let error = AofError::InvalidCommand {
            line: 42,
            reason: "malformed JSON".to_string(),
        };
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("42"));
        assert!(error_msg.contains("malformed JSON"));
    }

    // ========================================================================
    // AOF Writer 测试
    // ========================================================================

    use tempfile::TempDir;

    #[test]
    fn test_aof_writer_basic() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test.aof");

        let config = AofConfig::new(aof_path.clone())
            .set_sync_policy(AofSyncPolicy::Always)
            .with_enabled(true);

        let mut writer = AofWriter::new(config).unwrap();

        // 写入测试命令
        let cmd1 = AofCommand::insert(
            "cities".to_string(),
            "beijing".to_string(),
            [116.0, 39.0, 117.0, 40.0],
            r#"{"type":"Point","coordinates":[116.4,39.9]}"#.to_string(),
        );
        writer.append(&cmd1).unwrap();

        let cmd2 = AofCommand::delete(
            "cities".to_string(),
            "beijing".to_string(),
            [116.0, 39.0, 117.0, 40.0],
        );
        writer.append(&cmd2).unwrap();

        writer.flush().unwrap();
        drop(writer);

        // 验证文件存在且有内容
        assert!(aof_path.exists());
        let content = std::fs::read_to_string(&aof_path).unwrap();
        assert!(content.contains(r#""cmd":"INSERT""#));
        assert!(content.contains(r#""cmd":"DELETE""#));
    }

    #[test]
    fn test_aof_writer_json_lines_format() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test.aof");

        let config = AofConfig::new(aof_path.clone())
            .set_sync_policy(AofSyncPolicy::No)
            .with_enabled(true);

        let mut writer = AofWriter::new(config).unwrap();

        // 写入多条命令
        for i in 0..10 {
            let cmd = AofCommand::insert(
                "test".to_string(),
                format!("key{}", i),
                [0.0, 0.0, 1.0, 1.0],
                "{}".to_string(),
            );
            writer.append(&cmd).unwrap();
        }

        writer.flush().unwrap();
        drop(writer);

        // 验证每行都是有效的 JSON
        let content = std::fs::read_to_string(&aof_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 10);

        for line in lines {
            serde_json::from_str::<AofCommand>(line).unwrap();
        }
    }

    #[test]
    fn test_aof_writer_sync_policy_always() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test_always.aof");

        let config = AofConfig::new(aof_path.clone())
            .set_sync_policy(AofSyncPolicy::Always);

        let mut writer = AofWriter::new(config).unwrap();

        let cmd = AofCommand::insert(
            "test".to_string(),
            "key1".to_string(),
            [0.0, 0.0, 1.0, 1.0],
            "{}".to_string(),
        );

        // Always 策略应该立即写入
        writer.append(&cmd).unwrap();

        // 不需要显式 flush，数据应该已经在磁盘上
        let content = std::fs::read_to_string(&aof_path).unwrap();
        assert!(content.contains(r#""cmd":"INSERT""#));
    }

    #[test]
    fn test_aof_writer_sync_policy_everysecond() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test_everysec.aof");

        let config = AofConfig::new(aof_path.clone())
            .set_sync_policy(AofSyncPolicy::EverySecond);

        let mut writer = AofWriter::new(config).unwrap();

        let cmd = AofCommand::insert(
            "test".to_string(),
            "key1".to_string(),
            [0.0, 0.0, 1.0, 1.0],
            "{}".to_string(),
        );

        writer.append(&cmd).unwrap();

        // 等待超过 1 秒
        std::thread::sleep(std::time::Duration::from_millis(1100));

        // 再写入一条，应该触发同步
        writer.append(&cmd).unwrap();

        let content = std::fs::read_to_string(&aof_path).unwrap();
        assert!(content.contains(r#""cmd":"INSERT""#));
    }

    #[test]
    fn test_aof_writer_sync_policy_no() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test_no.aof");

        let config = AofConfig::new(aof_path.clone()).set_sync_policy(AofSyncPolicy::No);

        let mut writer = AofWriter::new(config).unwrap();

        let cmd = AofCommand::insert(
            "test".to_string(),
            "key1".to_string(),
            [0.0, 0.0, 1.0, 1.0],
            "{}".to_string(),
        );

        writer.append(&cmd).unwrap();

        // No 策略不会立即同步，需要显式 flush
        writer.flush().unwrap();

        let content = std::fs::read_to_string(&aof_path).unwrap();
        assert!(content.contains(r#""cmd":"INSERT""#));
    }

    #[test]
    fn test_aof_writer_bytes_written() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test_bytes.aof");

        let config = AofConfig::new(aof_path).set_sync_policy(AofSyncPolicy::No);

        let mut writer = AofWriter::new(config).unwrap();

        assert_eq!(writer.bytes_written(), 0);

        let cmd = AofCommand::insert(
            "test".to_string(),
            "key1".to_string(),
            [0.0, 0.0, 1.0, 1.0],
            "{}".to_string(),
        );

        writer.append(&cmd).unwrap();
        assert!(writer.bytes_written() > 0);

        let bytes1 = writer.bytes_written();
        writer.append(&cmd).unwrap();
        let bytes2 = writer.bytes_written();

        assert!(bytes2 > bytes1);
    }

    #[test]
    fn test_aof_writer_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test_disabled.aof");

        let config = AofConfig::new(aof_path).with_enabled(false);

        let result = AofWriter::new(config);
        assert!(matches!(result, Err(AofError::Disabled)));
    }

    #[test]
    fn test_aof_writer_drop_flushes() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test_drop.aof");

        {
            let config = AofConfig::new(aof_path.clone()).set_sync_policy(AofSyncPolicy::No);
            let mut writer = AofWriter::new(config).unwrap();

            let cmd = AofCommand::insert(
                "test".to_string(),
                "key1".to_string(),
                [0.0, 0.0, 1.0, 1.0],
                "{}".to_string(),
            );

            writer.append(&cmd).unwrap();
            // writer 在这里被 drop，应该自动 flush
        }

        // 验证数据已写入
        let content = std::fs::read_to_string(&aof_path).unwrap();
        assert!(content.contains(r#""cmd":"INSERT""#));
    }

    #[test]
    fn test_aof_writer_create_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested/dir/test.aof");

        let config = AofConfig::new(nested_path.clone());
        let mut writer = AofWriter::new(config).unwrap();

        let cmd = AofCommand::insert(
            "test".to_string(),
            "key1".to_string(),
            [0.0, 0.0, 1.0, 1.0],
            "{}".to_string(),
        );

        writer.append(&cmd).unwrap();
        writer.flush().unwrap();

        assert!(nested_path.exists());
    }
}
