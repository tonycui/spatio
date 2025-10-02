use std::sync::Arc;
use std::collections::HashMap;

use crate::protocol::parser::RespValue;
use crate::storage::GeoDatabase;
use crate::Result;

use super::{CommandType, basic::{PingCommand, HelloCommand, QuitCommand}, set::SetCommand, get::GetCommand, intersects::IntersectsCommand, nearby::NearbyCommand, drop::DropCommand, keys::KeysCommand};

/// 命令注册表，管理所有可用的命令
pub struct CommandRegistry {
    commands: HashMap<String, CommandType>,
}

impl CommandRegistry {
    /// 创建新的命令注册表
    pub fn new(database: Arc<GeoDatabase>) -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
        };
        
        // 注册基础命令
        registry.register(CommandType::Ping(PingCommand));
        registry.register(CommandType::Hello(HelloCommand));
        registry.register(CommandType::Quit(QuitCommand));
        
        // 注册存储命令
        registry.register(CommandType::Set(SetCommand::new(Arc::clone(&database))));
        registry.register(CommandType::Get(GetCommand::new(Arc::clone(&database))));
        
        // 注册空间查询命令
        registry.register(CommandType::Intersects(IntersectsCommand::new(Arc::clone(&database))));
        registry.register(CommandType::Nearby(NearbyCommand::new(Arc::clone(&database))));
        
        // 注册管理命令
        registry.register(CommandType::Drop(DropCommand::new(Arc::clone(&database))));
        registry.register(CommandType::Keys(KeysCommand::new(Arc::clone(&database))));
        
        registry
    }

    /// 注册一个命令
    pub fn register(&mut self, command: CommandType) {
        let name = command.name().to_uppercase();
        self.commands.insert(name, command);
    }

    /// 执行指定的命令
    pub async fn execute(&self, command_name: &str, args: &[RespValue]) -> Result<String> {
        let name = command_name.to_uppercase();
        match self.commands.get(&name) {
            Some(command) => command.execute(args).await,
            None => Ok(format!("-ERR unknown command '{}'\r\n", command_name)),
        }
    }

    /// 获取所有注册的命令名称
    pub fn command_names(&self) -> Vec<&str> {
        self.commands.keys().map(|s| s.as_str()).collect()
    }

    /// 检查命令是否存在
    pub fn has_command(&self, command_name: &str) -> bool {
        let name = command_name.to_uppercase();
        self.commands.contains_key(&name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_command_registry_basic() {
        let database = Arc::new(GeoDatabase::new());
        let registry = CommandRegistry::new(database);
        
        // 测试基础命令
        assert!(registry.has_command("PING"));
        assert!(registry.has_command("ping"));  // 大小写不敏感
        assert!(registry.has_command("SET"));
        assert!(registry.has_command("GET"));
        
        // 测试不存在的命令
        assert!(!registry.has_command("UNKNOWN"));
    }

    #[tokio::test]
    async fn test_command_registry_execute() {
        let database = Arc::new(GeoDatabase::new());
        let registry = CommandRegistry::new(database);
        
        // 测试 PING 命令
        let result = registry.execute("PING", &[]).await.unwrap();
        assert!(result.contains("PONG"));
        
        // 测试未知命令
        let result = registry.execute("UNKNOWN", &[]).await.unwrap();
        assert!(result.contains("unknown command"));
    }

    #[test]
    fn test_command_names() {
        let database = Arc::new(GeoDatabase::new());
        let registry = CommandRegistry::new(database);
        
        let names = registry.command_names();
        assert!(names.contains(&"PING"));
        assert!(names.contains(&"SET"));
        assert!(names.contains(&"GET"));
        assert!(names.contains(&"NEARBY"));
        assert!(names.len() >= 6);  // 至少有 6 个命令
    }

    #[tokio::test]
    async fn test_nearby_command_integration() {
        use crate::protocol::parser::RespValue;
        use serde_json::json;
        
        let database = Arc::new(GeoDatabase::new());
        let registry = CommandRegistry::new(Arc::clone(&database));
        
        // 1. 插入测试数据
        let point1 = json!({"type": "Point", "coordinates": [116.4, 39.9]});
        let point2 = json!({"type": "Point", "coordinates": [116.5, 40.0]});
        let point3 = json!({"type": "Point", "coordinates": [116.3, 39.8]});
        
        let set_args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("v1".to_string())),
            RespValue::BulkString(Some(point1.to_string())),
        ];
        registry.execute("SET", &set_args).await.unwrap();
        
        let set_args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("v2".to_string())),
            RespValue::BulkString(Some(point2.to_string())),
        ];
        registry.execute("SET", &set_args).await.unwrap();
        
        let set_args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("v3".to_string())),
            RespValue::BulkString(Some(point3.to_string())),
        ];
        registry.execute("SET", &set_args).await.unwrap();
        
        // 2. 执行 NEARBY 查询
        let nearby_args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
            RespValue::BulkString(Some("116.4".to_string())),
            RespValue::BulkString(Some("39.9".to_string())),
            RespValue::BulkString(Some("COUNT".to_string())),
            RespValue::BulkString(Some("2".to_string())),
        ];
        
        let result = registry.execute("NEARBY", &nearby_args).await.unwrap();
        
        // 3. 验证结果
        println!("Integration test result:\n{}", result);
        
        // 应该返回 2 个结果
        assert!(result.starts_with("*2"));  // 数组有 2 个元素
        
        // 结果应该包含点的坐标
        assert!(result.contains("116.4"));
        assert!(result.contains("39.9"));
        
        // 结果应该包含距离信息
        assert!(result.contains("0.00")); // v1 的距离应该是 0（完全匹配）
    }

    #[tokio::test]
    async fn test_nearby_command_with_registry() {
        use crate::protocol::parser::RespValue;
        
        let database = Arc::new(GeoDatabase::new());
        let registry = CommandRegistry::new(database);
        
        // 测试 NEARBY 命令是否注册
        assert!(registry.has_command("NEARBY"));
        assert!(registry.has_command("nearby"));  // 大小写不敏感
        
        // 测试参数错误的情况
        let invalid_args = vec![
            RespValue::BulkString(Some("fleet".to_string())),
            RespValue::BulkString(Some("POINT".to_string())),
        ];
        
        let result = registry.execute("NEARBY", &invalid_args).await.unwrap();
        assert!(result.contains("ERR"));
        assert!(result.contains("wrong number of arguments"));
    }
}
