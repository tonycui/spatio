use crate::commands::Command;
use crate::protocol::parser::RespValue;
use crate::protocol::RespResponse;
use crate::Result;

pub struct PingCommand;

impl Command for PingCommand {
    fn name(&self) -> &'static str {
        "PING"
    }

    fn execute(&self, _args: &[RespValue]) -> impl std::future::Future<Output = Result<String>> + Send {
        async move {
            Ok(RespResponse::simple_string("PONG"))
        }
    }
}

pub struct HelloCommand;

impl Command for HelloCommand {
    fn name(&self) -> &'static str {
        "HELLO"
    }

    fn execute(&self, _args: &[RespValue]) -> impl std::future::Future<Output = Result<String>> + Send {
        async move {
            Ok(RespResponse::simple_string("Hello, World!"))
        }
    }
}

pub struct QuitCommand;

impl Command for QuitCommand {
    fn name(&self) -> &'static str {
        "QUIT"
    }

    fn execute(&self, _args: &[RespValue]) -> impl std::future::Future<Output = Result<String>> + Send {
        async move {
            Ok(RespResponse::simple_string("Goodbye!"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ping_command() {
        let command = PingCommand;
        let result = command.execute(&[]).await.unwrap();
        assert_eq!(result, "+PONG\r\n");
    }

    #[tokio::test]
    async fn test_hello_command() {
        let command = HelloCommand;
        let result = command.execute(&[]).await.unwrap();
        assert_eq!(result, "+Hello, World!\r\n");
    }

    #[tokio::test]
    async fn test_quit_command() {
        let command = QuitCommand;
        let result = command.execute(&[]).await.unwrap();
        assert_eq!(result, "+Goodbye!\r\n");
    }
}
