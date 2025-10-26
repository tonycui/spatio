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
}
