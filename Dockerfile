# 阶段 1: 构建阶段 - 使用与本地开发环境一致的 Rust 1.89
FROM rust:1.89-slim AS builder

# 显示构建时使用的 Rust 版本（用于调试）
RUN rustc --version && cargo --version

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 复制 Cargo 配置文件
COPY Cargo.toml Cargo.lock ./

# 复制所有源代码（排除由 .dockerignore 处理）
COPY . .

# 构建 release 版本
RUN cargo build --release --bin spatio-server

# 使用 strip 减小二进制文件大小
RUN strip target/release/spatio-server

# 阶段 2: 运行阶段 - 使用最小化镜像
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    netcat-traditional \
    && rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN useradd -m -u 1000 spatio

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/spatio-server /usr/local/bin/spatio-server

# 创建数据目录
RUN mkdir -p /data && chown spatio:spatio /data

# 切换到非 root 用户
USER spatio
WORKDIR /data

# 暴露端口
EXPOSE 9851

# 健康检查：使用 RESP 格式的 PING 命令
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD printf '*1\r\n$4\r\nPING\r\n' | nc localhost 9851 | grep -q PONG || exit 1

# 设置环境变量
ENV RUST_LOG=info

# 启动命令：绑定到 0.0.0.0 使容器可以从外部访问
CMD ["spatio-server", "--host", "0.0.0.0"]
