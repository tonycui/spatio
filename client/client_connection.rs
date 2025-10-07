use std::io::{Read, Write};
use std::net::TcpStream;

use crate::protocol::parser::RespValue;
use crate::protocol::RespParser;
use crate::Result;

pub struct ClientConnection {
    stream: Option<TcpStream>,
    host: String,
    port: u16,
}

impl ClientConnection {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            stream: None,
            host: host.to_string(),
            port,
        }
    }

    pub fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        let stream = TcpStream::connect(&addr)?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn send_command(&mut self, cmd: &[String]) -> Result<RespValue> {
        if self.stream.is_none() {
            self.connect()?;
        }

        // 构建 RESP 命令
        let command = Self::build_resp_command(cmd);

        let stream = self.stream.as_mut().unwrap();

        // 发送命令
        stream.write_all(command.as_bytes())?;

        // 读取响应
        let mut buffer = Vec::new();
        let mut temp = [0; 4096];

        loop {
            let n = stream.read(&mut temp)?;
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&temp[..n]);

            // 简单检查是否收到完整响应（以 \r\n 结尾）
            if buffer.len() >= 2 && &buffer[buffer.len() - 2..] == b"\r\n" {
                break;
            }
        }

        // 解析响应
        let parser = RespParser::new();
        let response = parser.parse(&buffer)?;

        Ok(response)
    }

    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(stream) = self.stream.take() {
            stream.shutdown(std::net::Shutdown::Both)?;
        }
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    fn build_resp_command(cmd: &[String]) -> String {
        if cmd.is_empty() {
            return String::new();
        }

        // 构建 RESP 数组格式
        let mut result = format!("*{}\r\n", cmd.len());

        for arg in cmd {
            result.push_str(&format!("${}\r\n{}\r\n", arg.len(), arg));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_resp_command() {
        let cmd = vec!["PING".to_string()];
        let result = ClientConnection::build_resp_command(&cmd);
        assert_eq!(result, "*1\r\n$4\r\nPING\r\n");

        let cmd = vec!["PING".to_string(), "hello".to_string()];
        let result = ClientConnection::build_resp_command(&cmd);
        assert_eq!(result, "*2\r\n$4\r\nPING\r\n$5\r\nhello\r\n");
    }
}
