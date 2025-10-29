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
    /// * `geojson` - GeoJSON 数据
    pub fn insert(collection: String, key: String, geojson: String) -> Self {
        Self::Insert {
            ts: Self::now(),
            collection,
            key,
            geojson,
        }
    }

    /// 创建 DELETE 命令
    ///
    /// # 参数
    /// * `collection` - 集合名称
    /// * `key` - 对象 key
    pub fn delete(collection: String, key: String) -> Self {
        Self::Delete {
            ts: Self::now(),
            collection,
            key,
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
use std::io::{BufRead, BufReader, BufWriter, Write};
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
    ///     
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
// AOF Reader
// ============================================================================

/// AOF 读取器
///
/// 负责从 AOF 文件中读取命令，支持容错恢复
pub struct AofReader {
    reader: BufReader<File>,
    line_count: usize,
}

impl AofReader {
    /// 打开 AOF 文件
    ///
    /// # 参数
    /// * `file_path` - AOF 文件路径
    ///
    /// # 错误
    /// - 如果文件不存在，返回 `AofError::FileNotFound`
    /// - 如果无法打开文件，返回 IO 错误
    ///
    /// # 示例
    /// ```no_run
    /// use spatio::rtree::algorithms::aof::AofReader;
    /// use std::path::PathBuf;
    ///
    /// let reader = AofReader::open(PathBuf::from("appendonly.aof")).unwrap();
    /// ```
    pub fn open(file_path: PathBuf) -> Result<Self, AofError> {
        if !file_path.exists() {
            return Err(AofError::FileNotFound);
        }

        let file = File::open(&file_path)?;

        Ok(Self {
            reader: BufReader::new(file),
            line_count: 0,
        })
    }

    /// 读取下一条命令
    ///
    /// 逐行读取 AOF 文件，解析 JSON Lines 格式的命令。
    /// 自动跳过空行。
    ///
    /// # 返回
    /// - `Ok(Some(command))` - 成功读取到命令
    /// - `Ok(None)` - 到达文件末尾
    /// - `Err(...)` - 读取或解析错误
    ///
    /// # 示例
    /// ```no_run
    /// use spatio::rtree::algorithms::aof::AofReader;
    /// use std::path::PathBuf;
    ///
    /// let mut reader = AofReader::open(PathBuf::from("appendonly.aof")).unwrap();
    ///
    /// while let Some(cmd) = reader.read_next().unwrap() {
    ///     println!("Command: {:?}", cmd);
    /// }
    /// ```
    pub fn read_next(&mut self) -> Result<Option<AofCommand>, AofError> {
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line)?;

            if bytes_read == 0 {
                return Ok(None); // EOF
            }

            self.line_count += 1;
            let line = line.trim();

            if line.is_empty() {
                continue; // 跳过空行
            }

            match serde_json::from_str::<AofCommand>(line) {
                Ok(cmd) => return Ok(Some(cmd)),
                Err(e) => {
                    return Err(AofError::InvalidCommand {
                        line: self.line_count,
                        reason: e.to_string(),
                    });
                }
            }
        }
    }

    /// 恢复所有命令（容错模式）
    ///
    /// 读取整个 AOF 文件，尽可能恢复所有有效命令。
    /// 遇到损坏的行会记录错误但继续读取。
    ///
    /// # 返回
    /// 返回 `RecoveryResult`，包含成功恢复的命令、错误列表和统计信息。
    ///
    /// # 示例
    /// ```no_run
    /// use spatio::rtree::algorithms::aof::AofReader;
    /// use std::path::PathBuf;
    ///
    /// let mut reader = AofReader::open(PathBuf::from("appendonly.aof")).unwrap();
    /// let result = reader.recover_all().unwrap();
    ///
    /// println!("Recovered {} commands", result.commands.len());
    /// println!("Success rate: {:.2}%", result.success_rate());
    ///
    /// if !result.is_complete() {
    ///     eprintln!("Encountered {} errors during recovery", result.errors.len());
    /// }
    /// ```
    pub fn recover_all(&mut self) -> Result<RecoveryResult, AofError> {
        let mut commands = Vec::new();
        let mut errors = Vec::new();

        loop {
            match self.read_next() {
                Ok(Some(cmd)) => {
                    commands.push(cmd);
                }
                Ok(None) => break,
                Err(e) => {
                    // 记录错误但继续恢复
                    errors.push(e);
                }
            }
        }

        Ok(RecoveryResult {
            commands,
            errors,
            total_lines: self.line_count,
        })
    }

    /// 获取当前行号
    pub fn current_line(&self) -> usize {
        self.line_count
    }
}

/// 恢复结果
///
/// 包含 AOF 恢复过程的统计信息和结果
#[derive(Debug)]
pub struct RecoveryResult {
    /// 成功恢复的命令列表
    pub commands: Vec<AofCommand>,

    /// 恢复过程中遇到的错误列表
    pub errors: Vec<AofError>,

    /// 总行数（包括空行）
    pub total_lines: usize,
}

impl RecoveryResult {
    /// 是否完全成功（无错误）
    pub fn is_complete(&self) -> bool {
        self.errors.is_empty()
    }

    /// 成功率（百分比）
    pub fn success_rate(&self) -> f64 {
        if self.total_lines == 0 {
            return 100.0;
        }
        (self.commands.len() as f64 / self.total_lines as f64) * 100.0
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
            r#"{"type":"Point","coordinates":[116.4,39.9]}"#.to_string(),
        );

        assert!(matches!(cmd, AofCommand::Insert { .. }));
        assert_eq!(cmd.collection(), "cities");
        assert!(cmd.timestamp() > 0);
    }

    #[test]
    fn test_aof_command_delete_creation() {
        let cmd = AofCommand::delete("cities".to_string(), "beijing".to_string());

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
        let cmd = AofCommand::insert("test".to_string(), "key1".to_string(), "{}".to_string());

        // 序列化为 JSON
        let json = serde_json::to_string(&cmd).unwrap();

        // 验证 JSON 格式
        assert!(json.contains(r#""cmd":"INSERT""#));
        assert!(json.contains(r#""collection":"test""#));
        assert!(json.contains(r#""key":"key1""#));
        assert!(json.contains(r#""geojson""#));
        assert!(json.contains(r#""ts""#));

        // 反序列化
        let deserialized: AofCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.collection(), "test");
    }

    #[test]
    fn test_aof_command_json_deserialization() {
        let json = r#"{"cmd":"INSERT","ts":1698234567890123456,"collection":"cities","key":"beijing","geojson":"{\"type\":\"Point\",\"coordinates\":[116.4,39.9]}"}"#;

        let cmd: AofCommand = serde_json::from_str(json).unwrap();

        assert!(matches!(cmd, AofCommand::Insert { .. }));
        assert_eq!(cmd.collection(), "cities");
        assert_eq!(cmd.timestamp(), 1698234567890123456);

        if let AofCommand::Insert { key, geojson, .. } = cmd {
            assert_eq!(key, "beijing");
            assert!(geojson.contains("Point"));
        }
    }

    #[test]
    fn test_aof_command_all_types_serialization() {
        let commands = vec![
            AofCommand::insert("test".to_string(), "key1".to_string(), "{}".to_string()),
            AofCommand::delete("test".to_string(), "key1".to_string()),
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
            r#"{"type":"Point","coordinates":[116.4,39.9]}"#.to_string(),
        );
        writer.append(&cmd1).unwrap();

        let cmd2 = AofCommand::delete("cities".to_string(), "beijing".to_string());
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
            let cmd = AofCommand::insert("test".to_string(), format!("key{}", i), "{}".to_string());
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

        let config = AofConfig::new(aof_path.clone()).set_sync_policy(AofSyncPolicy::Always);

        let mut writer = AofWriter::new(config).unwrap();

        let cmd = AofCommand::insert("test".to_string(), "key1".to_string(), "{}".to_string());

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

        let config = AofConfig::new(aof_path.clone()).set_sync_policy(AofSyncPolicy::EverySecond);

        let mut writer = AofWriter::new(config).unwrap();

        let cmd = AofCommand::insert("test".to_string(), "key1".to_string(), "{}".to_string());

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

        let cmd = AofCommand::insert("test".to_string(), "key1".to_string(), "{}".to_string());

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

        let cmd = AofCommand::insert("test".to_string(), "key1".to_string(), "{}".to_string());

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

            let cmd = AofCommand::insert("test".to_string(), "key1".to_string(), "{}".to_string());

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

        let cmd = AofCommand::insert("test".to_string(), "key1".to_string(), "{}".to_string());

        writer.append(&cmd).unwrap();
        writer.flush().unwrap();

        assert!(nested_path.exists());
    }

    // ========================================================================
    // AOF Reader 测试
    // ========================================================================

    #[test]
    fn test_aof_reader_basic() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test.aof");

        // 写入测试数据
        {
            let config = AofConfig::new(aof_path.clone());
            let mut writer = AofWriter::new(config).unwrap();

            let cmd1 = AofCommand::insert(
                "cities".to_string(),
                "beijing".to_string(),
                r#"{"type":"Point"}"#.to_string(),
            );
            writer.append(&cmd1).unwrap();

            let cmd2 = AofCommand::delete("cities".to_string(), "shanghai".to_string());
            writer.append(&cmd2).unwrap();

            let cmd3 = AofCommand::drop("cities".to_string());
            writer.append(&cmd3).unwrap();

            writer.flush().unwrap();
        }

        // 读取并验证
        let mut reader = AofReader::open(aof_path).unwrap();

        let cmd1 = reader.read_next().unwrap().unwrap();
        assert!(matches!(cmd1, AofCommand::Insert { .. }));
        assert_eq!(cmd1.collection(), "cities");

        let cmd2 = reader.read_next().unwrap().unwrap();
        assert!(matches!(cmd2, AofCommand::Delete { .. }));

        let cmd3 = reader.read_next().unwrap().unwrap();
        assert!(matches!(cmd3, AofCommand::Drop { .. }));

        let cmd4 = reader.read_next().unwrap();
        assert!(cmd4.is_none());
    }

    #[test]
    fn test_aof_reader_recover_all() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test.aof");

        // 写入 100 条命令
        {
            let config = AofConfig::new(aof_path.clone());
            let mut writer = AofWriter::new(config).unwrap();

            for i in 0..100 {
                let cmd =
                    AofCommand::insert("test".to_string(), format!("key{}", i), "{}".to_string());
                writer.append(&cmd).unwrap();
            }

            writer.flush().unwrap();
        }

        // 恢复所有命令
        let mut reader = AofReader::open(aof_path).unwrap();
        let result = reader.recover_all().unwrap();

        assert_eq!(result.commands.len(), 100);
        assert_eq!(result.total_lines, 100);
        assert!(result.is_complete());
        assert_eq!(result.success_rate(), 100.0);
    }

    #[test]
    fn test_aof_reader_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("empty.aof");

        // 创建空文件
        File::create(&aof_path).unwrap();

        let mut reader = AofReader::open(aof_path).unwrap();
        let result = reader.recover_all().unwrap();

        assert_eq!(result.commands.len(), 0);
        assert_eq!(result.total_lines, 0);
        assert!(result.is_complete());
        assert_eq!(result.success_rate(), 100.0);
    }

    #[test]
    fn test_aof_reader_file_not_found() {
        let result = AofReader::open(PathBuf::from("/nonexistent/file.aof"));
        assert!(matches!(result, Err(AofError::FileNotFound)));
    }

    #[test]
    fn test_aof_reader_corrupted_file() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("corrupted.aof");

        // 写入混合数据：有效命令 + 损坏数据 + 有效命令
        {
            let mut file = File::create(&aof_path).unwrap();

            // 有效命令
            let cmd1 = AofCommand::insert("test".to_string(), "key1".to_string(), "{}".to_string());
            writeln!(file, "{}", serde_json::to_string(&cmd1).unwrap()).unwrap();

            // 损坏的 JSON
            writeln!(file, "{{invalid json}}").unwrap();

            // 不完整的 JSON
            write!(file, "{{\"cmd\":\"INSERT\"").unwrap();

            file.flush().unwrap();
        }

        // 恢复（容错模式）
        let mut reader = AofReader::open(aof_path).unwrap();
        let result = reader.recover_all().unwrap();

        // 应该恢复 1 条有效命令，遇到 2 个错误
        assert_eq!(result.commands.len(), 1);
        assert_eq!(result.errors.len(), 2);
        assert!(!result.is_complete());
        assert_eq!(result.total_lines, 3);
    }

    #[test]
    fn test_aof_reader_skip_empty_lines() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test.aof");

        // 写入带空行的数据
        {
            let mut file = File::create(&aof_path).unwrap();

            let cmd1 = AofCommand::insert("test".to_string(), "key1".to_string(), "{}".to_string());
            writeln!(file, "{}", serde_json::to_string(&cmd1).unwrap()).unwrap();

            writeln!(file).unwrap(); // 空行
            writeln!(file).unwrap(); // 空行

            let cmd2 = AofCommand::delete("test".to_string(), "key2".to_string());
            writeln!(file, "{}", serde_json::to_string(&cmd2).unwrap()).unwrap();

            file.flush().unwrap();
        }

        let mut reader = AofReader::open(aof_path).unwrap();
        let result = reader.recover_all().unwrap();

        // 应该恢复 2 条命令，跳过空行
        assert_eq!(result.commands.len(), 2);
        assert!(result.is_complete());
        assert_eq!(result.total_lines, 4); // 包括空行
    }

    #[test]
    fn test_aof_reader_write_then_read() {
        let temp_dir = TempDir::new().unwrap();
        let aof_path = temp_dir.path().join("test.aof");

        // 写入不同类型的命令
        let original_commands = vec![
            AofCommand::insert(
                "cities".to_string(),
                "beijing".to_string(),
                r#"{"type":"Point","coordinates":[116.4,39.9]}"#.to_string(),
            ),
            AofCommand::insert(
                "cities".to_string(),
                "shanghai".to_string(),
                r#"{"type":"Point","coordinates":[121.5,31.2]}"#.to_string(),
            ),
            AofCommand::delete("cities".to_string(), "beijing".to_string()),
            AofCommand::drop("cities".to_string()),
        ];

        {
            let config = AofConfig::new(aof_path.clone());
            let mut writer = AofWriter::new(config).unwrap();

            for cmd in &original_commands {
                writer.append(cmd).unwrap();
            }

            writer.flush().unwrap();
        }

        // 读取并验证
        let mut reader = AofReader::open(aof_path).unwrap();
        let result = reader.recover_all().unwrap();

        assert_eq!(result.commands.len(), 4);
        assert!(result.is_complete());

        // 验证命令类型
        assert!(matches!(result.commands[0], AofCommand::Insert { .. }));
        assert!(matches!(result.commands[1], AofCommand::Insert { .. }));
        assert!(matches!(result.commands[2], AofCommand::Delete { .. }));
        assert!(matches!(result.commands[3], AofCommand::Drop { .. }));

        // 验证集合名称
        for cmd in &result.commands {
            assert_eq!(cmd.collection(), "cities");
        }
    }

    #[test]
    fn test_recovery_result_success_rate() {
        // 100% 成功率
        let result = RecoveryResult {
            commands: vec![
                AofCommand::drop("test".to_string()),
                AofCommand::drop("test".to_string()),
            ],
            errors: vec![],
            total_lines: 2,
        };
        assert_eq!(result.success_rate(), 100.0);
        assert!(result.is_complete());

        // 50% 成功率
        let result = RecoveryResult {
            commands: vec![AofCommand::drop("test".to_string())],
            errors: vec![AofError::FileNotFound],
            total_lines: 2,
        };
        assert_eq!(result.success_rate(), 50.0);
        assert!(!result.is_complete());

        // 空文件
        let result = RecoveryResult {
            commands: vec![],
            errors: vec![],
            total_lines: 0,
        };
        assert_eq!(result.success_rate(), 100.0);
        assert!(result.is_complete());
    }
}
