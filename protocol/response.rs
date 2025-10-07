use crate::protocol::parser::RespValue;

pub struct RespResponse;

impl RespResponse {
    pub fn simple_string(s: &str) -> String {
        format!("+{}\r\n", s)
    }

    pub fn error(msg: &str) -> String {
        format!("-{}\r\n", msg)
    }

    pub fn integer(n: i64) -> String {
        format!(":{}\r\n", n)
    }

    pub fn bulk_string(s: Option<&str>) -> String {
        match s {
            Some(s) => format!("${}\r\n{}\r\n", s.len(), s),
            None => "$-1\r\n".to_string(),
        }
    }

    pub fn array(items: Option<&[RespValue]>) -> String {
        match items {
            Some(items) => {
                let mut result = format!("*{}\r\n", items.len());
                for item in items {
                    result.push_str(&Self::value_to_string(item));
                }
                result
            }
            None => "*-1\r\n".to_string(),
        }
    }

    fn value_to_string(value: &RespValue) -> String {
        match value {
            RespValue::SimpleString(s) => Self::simple_string(s),
            RespValue::Error(s) => Self::error(s),
            RespValue::Integer(n) => Self::integer(*n),
            RespValue::BulkString(s) => Self::bulk_string(s.as_deref()),
            RespValue::Array(arr) => Self::array(arr.as_deref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_string() {
        assert_eq!(RespResponse::simple_string("OK"), "+OK\r\n");
    }

    #[test]
    fn test_error() {
        assert_eq!(
            RespResponse::error("ERR unknown command"),
            "-ERR unknown command\r\n"
        );
    }

    #[test]
    fn test_integer() {
        assert_eq!(RespResponse::integer(1000), ":1000\r\n");
    }

    #[test]
    fn test_bulk_string() {
        assert_eq!(
            RespResponse::bulk_string(Some("foobar")),
            "$6\r\nfoobar\r\n"
        );
        assert_eq!(RespResponse::bulk_string(None), "$-1\r\n");
    }
}
