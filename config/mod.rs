use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Spatio 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatioConfig {
    /// 服务器配置
    pub server: ServerConfig,

    /// 存储配置
    pub storage: StorageConfig,

    /// AOF 持久化配置
    pub aof: AofConfig,

    /// 日志配置
    pub logging: LoggingConfig,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 监听地址
    #[serde(default = "default_host")]
    pub host: String,

    /// 监听端口
    #[serde(default = "default_port")]
    pub port: u16,

    /// 最大连接数
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// 请求超时时间（秒）
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// 数据目录
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// R-tree 最大子节点数
    #[serde(default = "default_max_children")]
    pub max_children: usize,
}

/// AOF 持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AofConfig {
    /// 是否启用 AOF
    #[serde(default = "default_aof_enabled")]
    pub enabled: bool,

    /// AOF 文件路径
    #[serde(default = "default_aof_filename")]
    pub filename: PathBuf,

    /// 同步策略：always, everysec, no
    #[serde(default = "default_sync_policy")]
    pub sync_policy: String,

    /// 是否启用 AOF 重写
    #[serde(default = "default_auto_rewrite")]
    pub auto_rewrite_enabled: bool,

    /// AOF 重写触发的最小大小（MB）
    #[serde(default = "default_auto_rewrite_min_size")]
    pub auto_rewrite_min_size: u64,

    /// AOF 重写触发的增长百分比
    #[serde(default = "default_auto_rewrite_percentage")]
    pub auto_rewrite_percentage: u64,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别：trace, debug, info, warn, error
    #[serde(default = "default_log_level")]
    pub level: String,

    /// 日志输出：stdout, file
    #[serde(default = "default_log_output")]
    pub output: String,

    /// 日志文件路径（当 output = file 时）
    pub log_file: Option<PathBuf>,
}

// ============================================================================
// 默认值函数
// ============================================================================

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    6379
}

fn default_max_connections() -> usize {
    10000
}

fn default_timeout() -> u64 {
    30
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("./data")
}

fn default_max_children() -> usize {
    10
}

fn default_aof_enabled() -> bool {
    true
}

fn default_aof_filename() -> PathBuf {
    PathBuf::from("./data/appendonly.aof")
}

fn default_sync_policy() -> String {
    "everysec".to_string()
}

fn default_auto_rewrite() -> bool {
    true
}

fn default_auto_rewrite_min_size() -> u64 {
    64
}

fn default_auto_rewrite_percentage() -> u64 {
    100
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_output() -> String {
    "stdout".to_string()
}

// ============================================================================
// 实现
// ============================================================================

impl Default for SpatioConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: default_host(),
                port: default_port(),
                max_connections: default_max_connections(),
                timeout: default_timeout(),
            },
            storage: StorageConfig {
                data_dir: default_data_dir(),
                max_children: default_max_children(),
            },
            aof: AofConfig {
                enabled: default_aof_enabled(),
                filename: default_aof_filename(),
                sync_policy: default_sync_policy(),
                auto_rewrite_enabled: default_auto_rewrite(),
                auto_rewrite_min_size: default_auto_rewrite_min_size(),
                auto_rewrite_percentage: default_auto_rewrite_percentage(),
            },
            logging: LoggingConfig {
                level: default_log_level(),
                output: default_log_output(),
                log_file: None,
            },
        }
    }
}

impl SpatioConfig {
    /// 从文件加载配置
    ///
    /// 配置加载顺序（优先级从低到高）：
    /// 1. 默认配置（内嵌的 default.toml）
    /// 2. 用户配置文件（可选）
    /// 3. 环境变量（SPATIO__ 前缀，使用双下划线分隔嵌套）
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use spatio::config::SpatioConfig;
    ///
    /// // 加载配置（如果文件不存在，使用默认配置）
    /// let config = SpatioConfig::from_file("spatio.toml").unwrap();
    /// ```
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let settings = config::Config::builder()
            // 1. 加载默认配置（内嵌）
            .add_source(config::File::from_str(
                include_str!("default.toml"),
                config::FileFormat::Toml,
            ))
            // 2. 加载用户配置（可选，不存在不报错）
            .add_source(config::File::with_name(path).required(false))
            // 3. 加载环境变量（SPATIO__ 前缀，双下划线分隔嵌套）
            .add_source(config::Environment::with_prefix("SPATIO").separator("__"))
            .build()
            .map_err(|e| format!("Failed to load config: {}", e))?;

        Ok(settings
            .try_deserialize()
            .map_err(|e| format!("Failed to parse config: {}", e))?)
    }

    /// 保存配置到文件
    ///
    /// # 示例
    ///
    /// ```no_run
    /// use spatio::config::SpatioConfig;
    ///
    /// let config = SpatioConfig::default();
    /// config.save_to_file("spatio.toml").unwrap();
    /// ```
    pub fn save_to_file(&self, path: &str) -> crate::Result<()> {
        let toml_string = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        std::fs::write(path, toml_string)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        Ok(())
    }

    /// 验证配置
    ///
    /// 检查配置的合法性，包括：
    /// - 端口范围
    /// - 同步策略
    /// - 日志级别
    /// - 数据目录
    pub fn validate(&self) -> Result<(), String> {
        // 验证端口（非特权端口）
        if self.server.port < 1024 {
            return Err(format!(
                "Server port {} is below 1024 (privileged range)",
                self.server.port
            ));
        }

        // 验证同步策略
        match self.aof.sync_policy.as_str() {
            "always" | "everysec" | "no" => {}
            _ => {
                return Err(format!(
                    "Invalid AOF sync policy: '{}'. Must be one of: always, everysec, no",
                    self.aof.sync_policy
                ))
            }
        }

        // 验证日志级别
        match self.logging.level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => {
                return Err(format!(
                    "Invalid log level: '{}'. Must be one of: trace, debug, info, warn, error",
                    self.logging.level
                ))
            }
        }

        // 验证日志文件配置
        if self.logging.output == "file" && self.logging.log_file.is_none() {
            return Err("Log output is 'file' but log_file path is not specified".to_string());
        }

        // 验证数据目录（尝试创建）
        if !self.storage.data_dir.exists() {
            std::fs::create_dir_all(&self.storage.data_dir).map_err(|e| {
                format!(
                    "Failed to create data directory '{}': {}",
                    self.storage.data_dir.display(),
                    e
                )
            })?;
        }

        Ok(())
    }

    /// 打印配置摘要
    pub fn print_summary(&self) {
        println!("📋 Spatio Configuration:");
        println!("   Server:      {}:{}", self.server.host, self.server.port);
        println!("   Max Connections: {}", self.server.max_connections);
        println!("   Timeout:     {} seconds", self.server.timeout);
        println!();
        println!("   Data Dir:    {}", self.storage.data_dir.display());
        println!("   Max Children: {}", self.storage.max_children);
        println!();
        println!(
            "   AOF:         {}",
            if self.aof.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
        if self.aof.enabled {
            println!("   AOF File:    {}", self.aof.filename.display());
            println!("   Sync Policy: {}", self.aof.sync_policy);
            println!(
                "   Auto Rewrite: {}",
                if self.aof.auto_rewrite_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
        println!();
        println!("   Log Level:   {}", self.logging.level);
        println!("   Log Output:  {}", self.logging.output);
        if let Some(ref log_file) = self.logging.log_file {
            println!("   Log File:    {}", log_file.display());
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SpatioConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 6379);
        assert!(config.aof.enabled);
        assert_eq!(config.aof.sync_policy, "everysec");
    }

    #[test]
    fn test_config_validation() {
        let mut config = SpatioConfig::default();

        // 有效配置
        assert!(config.validate().is_ok());

        // 无效端口
        config.server.port = 80;
        assert!(config.validate().is_err());
        config.server.port = 6379;

        // 无效同步策略
        config.aof.sync_policy = "invalid".to_string();
        assert!(config.validate().is_err());
        config.aof.sync_policy = "everysec".to_string();

        // 无效日志级别
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_save_and_load() {
        use tempfile::NamedTempFile;

        let config = SpatioConfig::default();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        // 保存
        config.save_to_file(path).unwrap();

        // 加载
        let loaded = SpatioConfig::from_file(path).unwrap();
        assert_eq!(loaded.server.port, config.server.port);
        assert_eq!(loaded.aof.sync_policy, config.aof.sync_policy);
    }
}
