pub mod client;
pub mod commands;
pub mod protocol;
pub mod server;
pub mod storage;
pub mod rtree;

use std::error::Error;

// 重新导出主要的公共接口
pub use rtree::{Rectangle, Node, Entry, RTree, GeoItem};

// 重新导出常用类型，便于二进制文件使用
pub use server::TcpServer;
pub use client::{ClientConnection, CliArgs, OutputFormatter};

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 9851, // Tile38 默认端口
        }
    }
}
