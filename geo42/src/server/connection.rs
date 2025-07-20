use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, info};

use crate::commands::CommandRegistry;
use crate::protocol::{RespParser, RespResponse};
use crate::protocol::parser::RespValue;
use crate::Result;

pub struct Connection {
    stream: TcpStream,
    registry: CommandRegistry,
    buffer: Vec<u8>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            registry: CommandRegistry::new(),
            buffer: Vec::with_capacity(4096),
        }
    }

    pub async fn handle(&mut self) -> Result<()> {
        let peer_addr = self.stream.peer_addr()?;
        info!("New connection from {}", peer_addr);

        loop {
            // 读取数据
            self.buffer.clear();
            match self.read_command().await {
                Ok(0) => {
                    info!("Connection closed by {}", peer_addr);
                    break;
                }
                Ok(_) => {
                    if let Err(e) = self.process_command().await {
                        error!("Error processing command: {}", e);
                        let error_response = RespResponse::error(&format!("ERR {}", e));
                        if let Err(write_err) = self.stream.write_all(error_response.as_bytes()).await {
                            error!("Failed to write error response: {}", write_err);
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read from socket: {}", e);
                    break;
                }
            }
        }

        info!("Connection with {} closed", peer_addr);
        Ok(())
    }

    async fn read_command(&mut self) -> Result<usize> {
        let mut temp_buffer = [0; 1024];
        let bytes_read = self.stream.read(&mut temp_buffer).await?;
        
        if bytes_read > 0 {
            self.buffer.extend_from_slice(&temp_buffer[..bytes_read]);
            debug!("Read {} bytes: {:?}", bytes_read, String::from_utf8_lossy(&self.buffer));
        }
        
        Ok(bytes_read)
    }

    async fn process_command(&mut self) -> Result<()> {
        // 查找完整的命令
        while let Some(command_bytes) = self.extract_complete_command() {
            // 解析命令
            let parser = RespParser::new();
            let command = parser.parse(&command_bytes)?;
            
            debug!("Parsed command: {:?}", command);

            // 执行命令
            let response = self.execute_command(command)?;
            
            // 发送响应
            self.stream.write_all(response.as_bytes()).await?;
            debug!("Sent response: {}", response.trim_end());
        }

        Ok(())
    }

    fn extract_complete_command(&mut self) -> Option<Vec<u8>> {
        // 简单实现：假设每次接收到的数据都是完整的命令
        if !self.buffer.is_empty() {
            let command = self.buffer.clone();
            self.buffer.clear();
            return Some(command);
        }
        None
    }

    fn execute_command(&self, command: RespValue) -> Result<String> {
        match command {
            RespValue::Array(Some(arr)) if !arr.is_empty() => {
                // 第一个元素是命令名
                if let RespValue::BulkString(Some(cmd_name)) = &arr[0] {
                    let args = &arr[1..];
                    self.registry.execute(cmd_name, args)
                } else {
                    Ok(RespResponse::error("ERR invalid command format"))
                }
            }
            RespValue::BulkString(Some(cmd_name)) => {
                // 简单命令（如直接输入 PING）
                self.registry.execute(&cmd_name, &[])
            }
            _ => Ok(RespResponse::error("ERR invalid command format")),
        }
    }
}
