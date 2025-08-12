# Spatio

> 🌍 A modern geospatial database built with Rust

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https- 💪 **Excellent Concurrency**: Maintains superior performance under 100 concurrent load

## 📄 Licensedge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

Spatio is a high-performance geospatial indexing service based on RTree, designed to provide the best performance and user experience.

## ✨ Features

- 🚀 **High Performance**: Currently the best-performing spatial indexing service based on RTree
- 🔒 **Memory Safety**: Memory safety guaranteed by Rust's type system  
- ⚡ **High Concurrency**: Native async support
- 🌐 **Protocol Compatible**: Supports RESP protocol (Redis compatible)
- 📍 **Spatial Indexing**: Integrated R-tree spatial indexing
- 🛠️ **Developer Friendly**: Clear error messages and modern tooling

## 🚀 Quick Start

### Installation

```bash
# Build from source
git clone https://github.com/your-org/spatio.git
cd spatio
cargo build --release
```

### Start Server

```bash
# Start Spatio server
cargo run --bin spatio-server
# Server will start on 127.0.0.1:9851
```

### Use Client

```bash
# Interactive mode
cargo run --bin spatio-cli -- --interactive

# Direct command execution
cargo run --bin spatio-cli -- PING
cargo run --bin spatio-cli -- SET fleet truck1 '{"type":"Point","coordinates":[116.3,39.9]}'
cargo run --bin spatio-cli -- GET fleet truck1
```

## 📖 Basic Usage

### Store Geospatial Data

```bash
# Store a point
SET fleet truck1 {"type":"Point","coordinates":[116.3974,39.9093]}

# Store a polygon
SET boundaries beijing {
  "type": "Polygon",
  "coordinates": [[
    [116.0,39.5], [117.0,39.5], 
    [117.0,40.5], [116.0,40.5], 
    [116.0,39.5]
  ]]
}
```

### Query Data

```bash
# Get object
GET fleet truck1

# Test connection
PING
```

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐
│   Spatio-CLI     │    │   Your App      │
└─────────┬───────┘    └─────────┬───────┘
          │                      │
          └──────────┬───────────┘
                     │ RESP Protocol
          ┌──────────▼───────────┐
          │    Spatio Server      │
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

## 🛣️ Development Roadmap

Check our detailed [Roadmap](ROADMAP.md) for project plans and progress.

### Current Status

- ✅ Basic storage system
- ✅ RESP protocol support  
- ✅ R-tree spatial indexing
- ✅ SET/GET commands
- ✅ Spatial query commands


## 📚 Documentation

- [API Documentation](docs/api.md)
- [Configuration Guide](docs/configuration.md)
- [Deployment Guide](docs/deployment.md)
- [Development Guide](docs/development.md)

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific module tests
cargo test storage::tests

# Run benchmarks
cargo bench
```

## 📊 Performance

### Performance Comparison Results

Performance comparison with Tile38 (100K data, 10K queries, 100 concurrent, on MacBook Pro M2 16G):

| Metric | Spatio | Tile38 | Performance Gain |
|--------|-------|--------|------------------|
| Successful Queries | 10,000 | 10,000 | - |
| **QPS** | **320.18** | **68.89** | **4.65x** |
| **Average Latency** | **264.82ms** | **1,376.88ms** | **5.20x faster** |
| **Median Latency** | **254.94ms** | **694.66ms** | **2.72x faster** |
| **P95 Latency** | **451.01ms** | **2,252.80ms** | **5.00x faster** |
| Min Latency | 18.76ms | 7.85ms | - |
| Max Latency | 839.51ms | 41,865.64ms | - |

### Test Configuration

```python
benchmark.run_benchmark(
    data_count=100000,    # 100K data points
    query_count=10000,    # 10K queries
    max_workers=100       # 100 concurrent workers
)
```

### Performance Highlights

- 🚀 **4.65x Higher QPS**: Spatio achieves 320+ QPS, far exceeding Tile38's 68.89 QPS
- ⚡ **5.2x Lower Latency**: Average query latency is only 1/5 of Tile38's
- 📈 **Better Stability**: P95 latency controlled under 451ms, while Tile38 exceeds 2.2s
- 💪 **Excellent Concurrency**: Maintains superior performance under 100 concurrent load

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Tile38](https://tile38.com/) - Inspiration source
- [Rust Community](https://www.rust-lang.org/community) - Excellent tools and libraries
- All contributors and supporters

---

**⭐ If this project helps you, please give us a Star!**
