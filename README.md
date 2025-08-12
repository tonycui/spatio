# Geo42

> 🌍 A modern geospatial database built with Rust

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

Geo42 是一个基于 RTree 高性能的地理空间索引服务，旨在提供最优秀的性能和使用体验。

## ✨ 特性

- 🚀 **高性能**: 目前基于 RTree 性能最好的空间索引服务
- 🔒 **内存安全**: Rust 类型系统保证内存安全  
- ⚡ **高并发**: 原生异步支持
- 🌐 **协议兼容**: 支持 RESP 协议（Redis 兼容）
- 📍 **空间索引**: 集成 R-tree 空间索引
- 🛠️ **开发友好**: 清晰的错误信息和现代化工具

## 🚀 快速开始

### 安装

```bash
# 从源码构建
git clone https://github.com/your-org/geo42.git
cd geo42
cargo build --release
```

### 启动服务器

```bash
# 启动 Geo42 服务器
cargo run --bin geo42-server
# 服务器将在 127.0.0.1:9851 启动
```

### 使用客户端

```bash
# 交互模式
cargo run --bin geo42-cli -- --interactive

# 直接执行命令
cargo run --bin geo42-cli -- PING
cargo run --bin geo42-cli -- SET fleet truck1 '{"type":"Point","coordinates":[116.3,39.9]}'
cargo run --bin geo42-cli -- GET fleet truck1
```

## 📖 基础用法

### 存储地理空间数据

```bash
# 存储一个点
SET fleet truck1 {"type":"Point","coordinates":[116.3974,39.9093]}

# 存储一个多边形
SET boundaries beijing {
  "type": "Polygon",
  "coordinates": [[
    [116.0,39.5], [117.0,39.5], 
    [117.0,40.5], [116.0,40.5], 
    [116.0,39.5]
  ]]
}
```

### 查询数据

```bash
# 获取对象
GET fleet truck1

# 测试连接
PING
```

## 🏗️ 架构

```
┌─────────────────┐    ┌─────────────────┐
│   Geo42-CLI     │    │   Your App      │
└─────────┬───────┘    └─────────┬───────┘
          │                      │
          └──────────┬───────────┘
                     │ RESP Protocol
          ┌──────────▼───────────┐
          │    Geo42 Server      │
          │                      │
          │  ┌─────────────────┐ │
          │  │ Command System  │ │
          │  └─────────────────┘ │
          │  ┌─────────────────┐ │
          │  │ Storage Engine  │ │
          │  │   + R-tree      │ │
          │  └─────────────────┘ │
          └──────────────────────┘
```

## 🛣️ 开发路线

查看我们的详细 [路线图](ROADMAP.md) 了解项目计划和进展。

### 当前状态

- ✅ 基础存储系统
- ✅ RESP 协议支持  
- ✅ R-tree 空间索引
- ✅ SET/GET 命令
- 🚧 空间查询命令 (进行中)
- 📋 持久化系统 (计划中)


## 📚 文档

- [API 文档](docs/api.md)
- [配置指南](docs/configuration.md)
- [部署指南](docs/deployment.md)
- [开发指南](docs/development.md)

## 🧪 测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test storage::tests

# 运行基准测试
cargo bench
```

## 📊 性能

### 性能对比测试结果

与 Tile38 的性能对比测试（10万条数据，1万次查询，100并发，On my macbook pro M2 16G）：

| 指标 | Geo42 | Tile38 | 性能提升 |
|------|-------|--------|----------|
| 查询成功数 | 10,000 | 10,000 | - |
| **QPS** | **320.18** | **68.89** | **4.65x** |
| **平均延迟** | **264.82ms** | **1,376.88ms** | **5.20x 更快** |
| **中位数延迟** | **254.94ms** | **694.66ms** | **2.72x 更快** |
| **P95延迟** | **451.01ms** | **2,252.80ms** | **5.00x 更快** |
| 最小延迟 | 18.76ms | 7.85ms | - |
| 最大延迟 | 839.51ms | 41,865.64ms | - |

### 测试配置

```python
benchmark.run_benchmark(
    data_count=100000,    # 10万条数据
    query_count=10000,    # 1万次查询
    max_workers=100       # 100 并发
)
```

### 性能亮点

- 🚀 **QPS 提升 4.65倍**: Geo42 达到 320+ QPS，远超 Tile38 的 68.89 QPS
- ⚡ **延迟降低 5.2倍**: 平均查询延迟仅为 Tile38 的 1/5
- � **稳定性更好**: P95延迟控制在 451ms，而 Tile38 超过 2.2s
- 💪 **高并发表现**: 在 100 并发下仍保持优异性能

## 📄 许可证

本项目基于 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [Tile38](https://tile38.com/) - 对标来源
- [Rust 社区](https://www.rust-lang.org/community) - 优秀的工具和库
- 所有贡献者和支持者

---

**⭐ 如果这个项目对你有帮助，请给我们一个 Star！**
