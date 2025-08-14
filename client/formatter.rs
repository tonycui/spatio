use colored::*;
use crate::protocol::parser::RespValue;

pub struct OutputFormatter;

impl OutputFormatter {
    pub fn format_response(value: &RespValue) -> String {
        match value {
            RespValue::SimpleString(s) => Self::format_simple_string(s),
            RespValue::Error(err) => Self::format_error(err),
            RespValue::Integer(i) => Self::format_integer(*i),
            RespValue::BulkString(s) => Self::format_bulk_string(s),
            RespValue::Array(arr) => Self::format_array(arr),
        }
    }

    fn format_simple_string(s: &str) -> String {
        s.green().to_string()
    }

    fn format_error(err: &str) -> String {
        format!("(error) {}", err.red())
    }

    fn format_integer(i: i64) -> String {
        format!("(integer) {}", i.to_string().cyan())
    }

    fn format_bulk_string(s: &Option<String>) -> String {
        match s {
            Some(s) => {
                if s.is_empty() {
                    "(empty string)".yellow().to_string()
                } else {
                    s.clone()
                }
            },
            None => "(nil)".red().to_string(),
        }
    }

    fn format_array(arr: &Option<Vec<RespValue>>) -> String {
        match arr {
            Some(values) => {
                if values.is_empty() {
                    "(empty array)".yellow().to_string()
                } else {
                    let mut result = String::new();
                    for (i, value) in values.iter().enumerate() {
                        let formatted_value = match value {
                            RespValue::BulkString(Some(s)) => s.clone(),
                            RespValue::BulkString(None) => "(nil)".to_string(),
                            RespValue::Integer(n) => n.to_string(),
                            RespValue::SimpleString(s) => s.clone(),
                            RespValue::Error(e) => format!("(error) {}", e),
                            RespValue::Array(_) => Self::format_response(value), // 递归处理嵌套数组
                        };
                        result.push_str(&format!("{}) {}\n", (i + 1).to_string().blue(), formatted_value));
                    }
                    result.trim_end().to_string()
                }
            },
            None => "(nil)".red().to_string(),
        }
    }

    pub fn format_prompt(host: &str, port: u16) -> String {
        format!("{}:{}> ", host.blue(), port.to_string().blue())
    }

    pub fn format_connecting_message(host: &str, port: u16) -> String {
        format!("Connecting to {}:{}...", host.cyan(), port.to_string().cyan())
    }

    pub fn format_connected_message(host: &str, port: u16) -> String {
        format!("Connected to {}:{}", host.green(), port.to_string().green())
    }

    pub fn format_disconnected_message() -> String {
        "Disconnected".red().to_string()
    }

    pub fn format_help_message() -> String {
        let help = r#"
Available commands:
  PING [message]     - Test server connection
  HELLO              - Get server information  
  QUIT               - Close connection and exit
  HELP               - Show this help message
  
Use Ctrl+C or Ctrl+D to exit interactive mode.
"#;
        help.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_string() {
        let value = RespValue::SimpleString("PONG".to_string());
        let result = OutputFormatter::format_response(&value);
        // 注意：测试时不检查颜色代码，只检查内容
        assert!(result.contains("PONG"));
    }

    #[test]
    fn test_format_bulk_string() {
        let value = RespValue::BulkString(Some("hello".to_string()));
        let result = OutputFormatter::format_response(&value);
        assert!(result.contains("hello"));
        
        let value = RespValue::BulkString(None);
        let result = OutputFormatter::format_response(&value);
        assert!(result.contains("nil"));
    }

    #[test]
    fn test_format_integer() {
        let value = RespValue::Integer(42);
        let result = OutputFormatter::format_response(&value);
        assert!(result.contains("42"));
        assert!(result.contains("integer"));
    }
}
