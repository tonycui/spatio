pub mod basic;

use crate::protocol::parser::RespValue;
use crate::Result;

pub trait Command {
    fn name(&self) -> &'static str;
    fn execute(&self, args: &[RespValue]) -> Result<String>;
}

pub struct CommandRegistry {
    commands: std::collections::HashMap<String, Box<dyn Command + Send + Sync>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            commands: std::collections::HashMap::new(),
        };
        
        // 注册基础命令
        registry.register(Box::new(basic::PingCommand));
        registry.register(Box::new(basic::HelloCommand));
        registry.register(Box::new(basic::QuitCommand));
        
        registry
    }

    pub fn register(&mut self, command: Box<dyn Command + Send + Sync>) {
        let name = command.name().to_uppercase();
        self.commands.insert(name, command);
    }

    pub fn execute(&self, command_name: &str, args: &[RespValue]) -> Result<String> {
        let name = command_name.to_uppercase();
        match self.commands.get(&name) {
            Some(command) => command.execute(args),
            None => Ok(format!("-ERR unknown command '{}'\r\n", command_name)),
        }
    }
}
