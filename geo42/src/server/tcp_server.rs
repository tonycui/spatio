use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

use crate::{Config, Result};
use crate::server::Connection;

pub struct TcpServer {
    config: Config,
}

impl TcpServer {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;
        
        info!("Geo42 server listening on {}", addr);
        info!("Ready to accept connections");

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("Accepted connection from {}", addr);
                    
                    // 为每个连接创建一个异步任务
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream).await {
                            error!("Error handling client {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_client(stream: TcpStream) -> Result<()> {
        let mut connection = Connection::new(stream);
        connection.handle().await
    }
}

impl Drop for TcpServer {
    fn drop(&mut self) {
        info!("TCP server shutting down");
    }
}
