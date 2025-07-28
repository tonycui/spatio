use std::sync::Arc;
use std::collections::HashMap;

use crate::protocol::parser::RespValue;
use crate::storage::GeoDatabase;
use crate::Result;

use super::{CommandType, basic::{PingCommand, HelloCommand, QuitCommand}, set::SetCommand, get::GetCommand};

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
        assert!(names.len() >= 5);  // 至少有 5 个命令
    }
}
