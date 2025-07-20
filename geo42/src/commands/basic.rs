use crate::commands::Command;
use crate::protocol::parser::RespValue;
use crate::protocol::response::RespResponse;
use crate::Result;

pub struct PingCommand;

impl Command for PingCommand {
    fn name(&self) -> &'static str {
        "PING"
    }

    fn execute(&self, args: &[RespValue]) -> Result<String> {
        match args.len() {
            0 => Ok(RespResponse::simple_string("PONG")),
            1 => {
                if let RespValue::BulkString(Some(msg)) = &args[0] {
                    Ok(RespResponse::bulk_string(Some(msg)))
                } else {
                    Ok(RespResponse::error("ERR wrong argument type"))
                }
            }
            _ => Ok(RespResponse::error("ERR wrong number of arguments for 'ping' command")),
        }
    }
}

pub struct HelloCommand;

impl Command for HelloCommand {
    fn name(&self) -> &'static str {
        "HELLO"
    }

    fn execute(&self, _args: &[RespValue]) -> Result<String> {
        let info = vec![
            RespValue::BulkString(Some("server".to_string())),
            RespValue::BulkString(Some("geo42".to_string())),
            RespValue::BulkString(Some("version".to_string())),
            RespValue::BulkString(Some("0.1.0".to_string())),
            RespValue::BulkString(Some("proto".to_string())),
            RespValue::Integer(3),
            RespValue::BulkString(Some("id".to_string())),
            RespValue::Integer(1),
            RespValue::BulkString(Some("mode".to_string())),
            RespValue::BulkString(Some("standalone".to_string())),
        ];
        
        Ok(RespResponse::array(Some(&info)))
    }
}

pub struct QuitCommand;

impl Command for QuitCommand {
    fn name(&self) -> &'static str {
        "QUIT"
    }

    fn execute(&self, _args: &[RespValue]) -> Result<String> {
        Ok(RespResponse::simple_string("OK"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_no_args() {
        let cmd = PingCommand;
        let result = cmd.execute(&[]).unwrap();
        assert_eq!(result, "+PONG\r\n");
    }

    #[test]
    fn test_ping_with_message() {
        let cmd = PingCommand;
        let args = vec![RespValue::BulkString(Some("hello".to_string()))];
        let result = cmd.execute(&args).unwrap();
        assert_eq!(result, "$5\r\nhello\r\n");
    }

    #[test]
    fn test_quit() {
        let cmd = QuitCommand;
        let result = cmd.execute(&[]).unwrap();
        assert_eq!(result, "+OK\r\n");
    }
}
