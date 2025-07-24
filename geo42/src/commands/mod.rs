pub mod basic;
pub mod set;
pub mod get;

use std::sync::Arc;

use crate::protocol::parser::RespValue;
use crate::storage::GeoDatabase;
use crate::Result;

use basic::{PingCommand, HelloCommand, QuitCommand};
use set::SetCommand;
use get::GetCommand;

pub trait Command {
    fn name(&self) -> &'static str;
    fn execute(&self, args: &[RespValue]) -> impl std::future::Future<Output = Result<String>> + Send;
}

pub enum CommandType {
    Ping(PingCommand),
    Hello(HelloCommand),
    Quit(QuitCommand),
    Set(SetCommand),
    Get(GetCommand),
}

impl CommandType {
    fn name(&self) -> &'static str {
        match self {
            CommandType::Ping(cmd) => cmd.name(),
            CommandType::Hello(cmd) => cmd.name(),
            CommandType::Quit(cmd) => cmd.name(),
            CommandType::Set(cmd) => cmd.name(),
            CommandType::Get(cmd) => cmd.name(),
        }
    }

    async fn execute(&self, args: &[RespValue]) -> Result<String> {
        match self {
            CommandType::Ping(cmd) => cmd.execute(args).await,
            CommandType::Hello(cmd) => cmd.execute(args).await,
            CommandType::Quit(cmd) => cmd.execute(args).await,
            CommandType::Set(cmd) => cmd.execute(args).await,
            CommandType::Get(cmd) => cmd.execute(args).await,
        }
    }
}

pub struct CommandRegistry {
    commands: std::collections::HashMap<String, CommandType>,
}

impl CommandRegistry {
    pub fn new(database: Arc<GeoDatabase>) -> Self {
        let mut registry = Self {
            commands: std::collections::HashMap::new(),
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

    pub fn register(&mut self, command: CommandType) {
        let name = command.name().to_uppercase();
        self.commands.insert(name, command);
    }

    pub async fn execute(&self, command_name: &str, args: &[RespValue]) -> Result<String> {
        let name = command_name.to_uppercase();
        match self.commands.get(&name) {
            Some(command) => command.execute(args).await,
            None => Ok(format!("-ERR unknown command '{}'\r\n", command_name)),
        }
    }
}
