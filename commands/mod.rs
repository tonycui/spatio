pub mod args;
pub mod basic;
pub mod delete;
pub mod drop;
pub mod get;
pub mod intersects;
pub mod keys;
pub mod nearby;
pub mod registry;
pub mod set;

use crate::protocol::parser::RespValue;
use crate::Result;

use basic::{HelloCommand, PingCommand, QuitCommand};
use delete::DeleteCommand;
use drop::DropCommand;
use get::GetCommand;
use intersects::IntersectsCommand;
use keys::KeysCommand;
use nearby::NearbyCommand;
use set::SetCommand;

// 重新导出常用的类型
pub use args::{ArgumentParser, DeleteArgs, DropArgs, GetArgs, NearbyArgs, SetArgs};
pub use intersects::IntersectsArgs;
pub use registry::CommandRegistry;

pub trait Command {
    fn name(&self) -> &'static str;
    fn execute(
        &self,
        args: &[RespValue],
    ) -> impl std::future::Future<Output = Result<String>> + Send;
}

pub enum CommandType {
    Ping(PingCommand),
    Hello(HelloCommand),
    Quit(QuitCommand),
    Set(SetCommand),
    Get(GetCommand),
    Delete(DeleteCommand),
    Intersects(IntersectsCommand),
    Nearby(NearbyCommand),
    Drop(DropCommand),
    Keys(KeysCommand),
}

impl CommandType {
    fn name(&self) -> &'static str {
        match self {
            CommandType::Ping(cmd) => cmd.name(),
            CommandType::Hello(cmd) => cmd.name(),
            CommandType::Quit(cmd) => cmd.name(),
            CommandType::Set(cmd) => cmd.name(),
            CommandType::Get(cmd) => cmd.name(),
            CommandType::Delete(cmd) => cmd.name(),
            CommandType::Intersects(cmd) => cmd.name(),
            CommandType::Nearby(cmd) => cmd.name(),
            CommandType::Drop(cmd) => cmd.name(),
            CommandType::Keys(cmd) => cmd.name(),
        }
    }

    async fn execute(&self, args: &[RespValue]) -> Result<String> {
        match self {
            CommandType::Ping(cmd) => cmd.execute(args).await,
            CommandType::Hello(cmd) => cmd.execute(args).await,
            CommandType::Quit(cmd) => cmd.execute(args).await,
            CommandType::Set(cmd) => cmd.execute(args).await,
            CommandType::Get(cmd) => cmd.execute(args).await,
            CommandType::Delete(cmd) => cmd.execute(args).await,
            CommandType::Intersects(cmd) => cmd.execute(args).await,
            CommandType::Nearby(cmd) => cmd.execute(args).await,
            CommandType::Drop(cmd) => cmd.execute(args).await,
            CommandType::Keys(cmd) => cmd.execute(args).await,
        }
    }
}
