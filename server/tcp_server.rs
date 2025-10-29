use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

use crate::server::ServerConnection;
use crate::storage::GeoDatabase;
use crate::{Result, SpatioConfig};

pub struct TcpServer {
    config: SpatioConfig,
    database: Arc<GeoDatabase>,
}

impl TcpServer {
    pub fn new(config: SpatioConfig, database: GeoDatabase) -> Self {
        Self {
            config,
            database: Arc::new(database),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.server.host, self.config.server.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("Spatio server listening on {}", addr);
        info!("Ready to accept connections");

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("Accepted connection from {}", addr);

                    // 克隆数据库引用以便在异步任务中使用
                    let database = Arc::clone(&self.database);

                    // 为每个连接创建一个异步任务
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, database).await {
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

    async fn handle_client(stream: TcpStream, database: Arc<GeoDatabase>) -> Result<()> {
        let mut connection = ServerConnection::new(stream, database);
        connection.handle().await
    }
}

impl Drop for TcpServer {
    fn drop(&mut self) {
        info!("TCP server shutting down");
    }
}
