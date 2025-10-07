use crate::Result;
use std::io::{BufRead, BufReader, Cursor};

#[derive(Debug, Clone, PartialEq)]
pub enum RespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<String>),
    Array(Option<Vec<RespValue>>),
}

pub struct RespParser;

impl Default for RespParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RespParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, input: &[u8]) -> Result<RespValue> {
        let mut cursor = Cursor::new(input);
        let mut reader = BufReader::new(&mut cursor);
        self.parse_value(&mut reader)
    }

    fn parse_value<R: BufRead>(&self, reader: &mut R) -> Result<RespValue> {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;

        if bytes_read == 0 {
            return Err("Unexpected EOF".into());
        }

        let line = line.trim_end_matches('\n').trim_end_matches('\r');
        if line.is_empty() {
            return Err("Empty line".into());
        }

        let first_char = line.chars().next().unwrap();
        let content = &line[1..];

        match first_char {
            '+' => Ok(RespValue::SimpleString(content.to_string())),
            '-' => Ok(RespValue::Error(content.to_string())),
            ':' => {
                let num = content.parse::<i64>()?;
                Ok(RespValue::Integer(num))
            }
            '$' => {
                let len = content.parse::<i64>()?;
                if len == -1 {
                    Ok(RespValue::BulkString(None))
                } else if len == 0 {
                    // 读取空字符串的 \r\n
                    let mut end = String::new();
                    reader.read_line(&mut end)?;
                    Ok(RespValue::BulkString(Some(String::new())))
                } else {
                    let mut buf = vec![0; len as usize];
                    reader.read_exact(&mut buf)?;
                    // 读取结尾的 \r\n
                    let mut end = String::new();
                    reader.read_line(&mut end)?;
                    let s = String::from_utf8(buf)?;
                    Ok(RespValue::BulkString(Some(s)))
                }
            }
            '*' => {
                let len = content.parse::<i64>()?;
                if len == -1 {
                    Ok(RespValue::Array(None))
                } else {
                    let mut arr = Vec::with_capacity(len as usize);
                    for _ in 0..len {
                        let value = self.parse_value(reader)?;
                        arr.push(value);
                    }
                    Ok(RespValue::Array(Some(arr)))
                }
            }
            _ => Err(format!("Unknown RESP type: {}", first_char).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_string() {
        let parser = RespParser::new();
        let result = parser.parse(b"+OK\r\n").unwrap();
        assert_eq!(result, RespValue::SimpleString("OK".to_string()));
    }

    #[test]
    fn test_error() {
        let parser = RespParser::new();
        let result = parser.parse(b"-Error message\r\n").unwrap();
        assert_eq!(result, RespValue::Error("Error message".to_string()));
    }

    #[test]
    fn test_integer() {
        let parser = RespParser::new();
        let result = parser.parse(b":1000\r\n").unwrap();
        assert_eq!(result, RespValue::Integer(1000));
    }

    #[test]
    fn test_bulk_string() {
        let parser = RespParser::new();
        let result = parser.parse(b"$6\r\nfoobar\r\n").unwrap();
        assert_eq!(result, RespValue::BulkString(Some("foobar".to_string())));
    }

    #[test]
    fn test_null_bulk_string() {
        let parser = RespParser::new();
        let result = parser.parse(b"$-1\r\n").unwrap();
        assert_eq!(result, RespValue::BulkString(None));
    }

    #[test]
    fn test_array() {
        let parser = RespParser::new();
        let result = parser.parse(b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n").unwrap();
        let expected = RespValue::Array(Some(vec![
            RespValue::BulkString(Some("foo".to_string())),
            RespValue::BulkString(Some("bar".to_string())),
        ]));
        assert_eq!(result, expected);
    }
}
