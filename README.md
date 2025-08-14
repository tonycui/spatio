# Spatio

> 🌍 A modern geospatial database built with Rust

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

## 📄 License
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
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
git clone https://github.com/tonycui/spatio.git
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
# Insert an irregular polygon (representing a city district)
SET districts id_1 '{"type":"Feature","properties":{"id":"id_1"},"geometry":{"type":"Polygon","coordinates":[[[2.5,1.0],[6.2,0.8],[8.1,3.5],[7.8,6.9],[5.2,8.1],[2.1,7.3],[0.9,4.2],[2.5,1.0]]]}}'


# Find all districts that intersect with the delivery zone
INTERSECTS districts '{"type":"Polygon","coordinates":[[[3.0,2.0],[7.0,1.5],[8.5,5.0],[6.0,7.0],[3.5,6.5],[3.0,2.0]]]}'


# Test connection
PING
```

### Notes on CLI usage and quoting

- spatio-cli interactive mode treats everything after the 3rd token in SET as one argument, so JSON doesn't need extra quotes:

  ```bash
  # spatio-cli (interactive or direct) – JSON can be unquoted
  SET districts id_1 {"type":"Feature","properties":{"id":"id_1"},"geometry":{"type":"Polygon","coordinates":[[[2.5,1.0],[6.2,0.8],[8.1,3.5],[7.8,6.9],[5.2,8.1],[2.1,7.3],[0.9,4.2],[2.5,1.0]]]}} 
  ```

- redis-cli splits by spaces; you must wrap JSON or it will be parsed as multiple args:

  ```bash
  # Recommended: single quotes
  redis-cli -p 9851 SET districts id_1 '{"type":"Feature","properties":{"id":"id_1"},"geometry":{"type":"Polygon","coordinates":[[[2.5,1.0],[6.2,0.8],[8.1,3.5],[7.8,6.9],[5.2,8.1],[2.1,7.3],[0.9,4.2],[2.5,1.0]]]}}'

  # Or double quotes with escapes
  redis-cli -p 9851 SET districts id_1 "{\"type\":\"Feature\",...}"

  # Or pipe the body via -x (no quoting inside)
  echo '{"type":"Feature","properties":{"id":"id_1"},"geometry":{"type":"Polygon","coordinates":[[[2.5,1.0],[6.2,0.8],[8.1,3.5],[7.8,6.9],[5.2,8.1],[2.1,7.3],[0.9,4.2],[2.5,1.0]]]}}' \
    | redis-cli -p 9851 -x SET districts id_1
  ```

- redis-cli displays Bulk Strings with quotes/escapes by default. Use --raw to see plain JSON:

  ```bash
  # With quotes/escapes (default)
  redis-cli -p 9851 GET districts id_1
  # Plain JSON
  redis-cli -p 9851 --raw GET districts id_1
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
python3 benchmark/benchmark_geo42_only_concurrent.py
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
