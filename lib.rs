pub mod client;
pub mod commands;
pub mod config;
pub mod protocol;
pub mod rtree;
pub mod server;
pub mod storage;

use std::error::Error;

// 重新导出主要的公共接口
pub use rtree::{Entry, GeoItem, Node, RTree, Rectangle};

// 重新导出常用类型，便于二进制文件使用
pub use client::{CliArgs, ClientConnection, OutputFormatter};
pub use config::SpatioConfig;
pub use server::TcpServer;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
