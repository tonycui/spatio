# Geo42

> 🌍 A modern geospatial database built with Rust

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

Geo42 是一个高性能的地理空间数据库，旨在替代 Tile38 并提供更优秀的性能和开发体验。

## ✨ 特性

- 🚀 **高性能**: 基于 Rust 和异步架构
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

## 🤝 贡献

我们欢迎所有形式的贡献！

1. Fork 这个项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启 Pull Request

查看 [贡献指南](CONTRIBUTING.md) 了解详细信息。

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

# benchmark
benchmark.run_benchmark(
        data_count=100000,    # 10万条数据
        query_count=10000,    # 1万次查询
        max_workers=100      # 100 并发（先用较小值测试）
    )

============================================================
geo42 vs tile38 高并发性能对比结果
============================================================
指标              geo42           tile38          对比             
------------------------------------------------------------
查询成功数           10000           10000          
QPS             320.18          68.89           4.65           x
平均延迟(ms)        264.82          1376.88         5.20           x
中位数(ms)         254.94          694.66          2.72           x
P95延迟(ms)       451.01          2252.80         5.00           x
最小延迟(ms)        18.76           7.85           
最大延迟(ms)        839.51          41865.64       

============================================================

## 📊 性能

*基准测试结果即将发布...*

## 📄 许可证

本项目基于 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [Tile38](https://tile38.com/) - 灵感来源
- [Rust 社区](https://www.rust-lang.org/community) - 优秀的工具和库
- 所有贡献者和支持者

---

**⭐ 如果这个项目对你有帮助，请给我们一个 Star！**
