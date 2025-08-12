use spatio::{Config, Result};
use spatio::server::TcpServer;
use tracing::{info, Level};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to bind to
    #[arg(short, long, default_value_t = 9851)]
    port: u16,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 初始化日志系统
    let log_level = match args.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    info!("Starting Spatio server...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // 创建配置
    let config = Config {
        host: args.host,
        port: args.port,
    };

    // 启动服务器
    let server = TcpServer::new(config);
    server.start().await?;

    Ok(())
}
