pub mod server;
pub mod protocol;
pub mod commands;

use std::error::Error;

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
