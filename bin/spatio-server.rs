use clap::Parser;
use spatio::server::TcpServer;
use spatio::{Result, SpatioConfig};
use tracing::{info, Level};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 配置文件路径
    #[arg(short, long, default_value = "spatio.toml")]
    config: String,

    /// 生成默认配置文件并退出
    #[arg(long)]
    generate_config: bool,

    /// Host to bind to (overrides config file)
    #[arg(long)]
    host: Option<String>,

    /// Port to bind to (overrides config file)
    #[arg(short, long)]
    port: Option<u16>,

    /// Log level (overrides config file)
    #[arg(long)]
    log_level: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // 生成默认配置文件
    if args.generate_config {
        let config = SpatioConfig::default();
        config.save_to_file(&args.config)?;
        println!("✅ Generated default configuration: {}", args.config);
        println!("📝 You can edit this file and restart the server.");
        return Ok(());
    }

    // 加载配置
    let mut config = SpatioConfig::from_file(&args.config)?;

    // 命令行参数覆盖配置文件
    if let Some(host) = args.host {
        config.server.host = host;
    }
    if let Some(port) = args.port {
        config.server.port = port;
    }
    if let Some(log_level) = args.log_level {
        config.logging.level = log_level;
    }

    // 验证配置
    config.validate()?;

    // 初始化日志系统
    init_logging(&config.logging);

    info!("🚀 Starting Spatio server...");
    info!("📦 Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    // 打印配置摘要
    config.print_summary();

    // 创建数据库实例
    let _db = if config.aof.enabled {
        use spatio::rtree::algorithms::aof::{AofConfig as AofWriterConfig, AofSyncPolicy};

        // 转换同步策略
        let sync_policy = match config.aof.sync_policy.as_str() {
            "always" => AofSyncPolicy::Always,
            "everysec" => AofSyncPolicy::EverySecond,
            "no" => AofSyncPolicy::No,
            _ => AofSyncPolicy::EverySecond,
        };

        let aof_config =
            AofWriterConfig::new(config.aof.filename.clone()).set_sync_policy(sync_policy);

        info!(
            "💾 AOF enabled with sync policy: {}",
            config.aof.sync_policy
        );

        let db = spatio::storage::GeoDatabase::with_aof(aof_config)?;

        // 从 AOF 恢复数据
        if config.aof.filename.exists() {
            info!("📖 Recovering from AOF file...");
            let (commands, errors) = db.recover_from_aof(config.aof.filename.clone()).await?;

            if errors > 0 {
                tracing::warn!("⚠️  Recovered {} commands with {} errors", commands, errors);
            } else {
                info!("✅ Successfully recovered {} commands", commands);
            }
        }

        db
    } else {
        info!("⚠️  AOF disabled - data will not be persisted");
        spatio::storage::GeoDatabase::new()
    };

    info!(
        "🌐 Server listening on {}:{}",
        config.server.host, config.server.port
    );
    println!();

    // 启动服务器（传入配置和数据库实例）
    let server = TcpServer::new(config, _db);
    server.start().await?;

    Ok(())
}

/// 初始化日志系统
fn init_logging(config: &spatio::config::LoggingConfig) {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let filter = match config.level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    match config.output.as_str() {
        "stdout" => {
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer().with_target(false))
                .with(tracing_subscriber::filter::LevelFilter::from_level(filter))
                .init();
        }
        "file" => {
            if let Some(log_file) = &config.log_file {
                // 确保日志目录存在
                if let Some(parent) = log_file.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_file)
                    .expect("Failed to open log file");

                tracing_subscriber::registry()
                    .with(
                        tracing_subscriber::fmt::layer()
                            .with_writer(file)
                            .with_target(false),
                    )
                    .with(tracing_subscriber::filter::LevelFilter::from_level(filter))
                    .init();
            }
        }
        _ => {
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer().with_target(false))
                .with(tracing_subscriber::filter::LevelFilter::from_level(filter))
                .init();
        }
    }
}
