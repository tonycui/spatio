# Spatio

> 🌍 A modern geospatial database built with Rust

[![Rust Version](https://img.shields.io/badge/rust-1.89-orange.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/docker/v/spaito/spatio?label=docker)](https://hub.docker.com/r/spaito/spatio)
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

### Docker (Recommended) 🐳

The easiest way to run Spatio:

```bash
# Pull and run
docker pull spaito/spatio
docker run -p 9851:9851 spaito/spatio

# Or use docker-compose
docker-compose up -d
```

### From Source

Requirements:
- Rust 1.75+ (developed with 1.89)
- Cargo

```bash
# Clone the repository
git clone https://github.com/tonycui/spatio.git
cd spatio

# Build
cargo build --release

# Run server
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

## � Docker Usage

### Environment Variables

- `RUST_LOG`: Log level (default: `info`, options: `trace`, `debug`, `info`, `warn`, `error`)
- `SPATIO_HOST`: Listen address (default: `0.0.0.0`)
- `SPATIO_PORT`: Listen port (default: `9851`)

### Data Persistence

Use volume mounts to persist data:

```bash
docker run -p 9851:9851 -v spatio-data:/data spaito/spatio
```

### Docker Compose Example

```yaml
version: '3.8'
services:
  spatio:
    image: spaito/spatio:latest
    ports:
      - "9851:9851"
    volumes:
      - spatio-data:/data
    environment:
      - RUST_LOG=info

volumes:
  spatio-data:
```

### Building Docker Image

```bash
# Build the image
docker build -t spaito/spatio:latest .

# Run it
docker run -p 9851:9851 spaito/spatio:latest

# Push to Docker Hub
docker push spaito/spatio:latest
```

## �📖 Basic Usage

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
# Get a specific item
GET fleet truck1

# Delete an item
DELETE fleet truck1

# Insert an irregular polygon (representing a city district)
SET districts id_1 '{"type":"Feature","properties":{"id":"id_1"},"geometry":{"type":"Polygon","coordinates":[[[2.5,1.0],[6.2,0.8],[8.1,3.5],[7.8,6.9],[5.2,8.1],[2.1,7.3],[0.9,4.2],[2.5,1.0]]]}}'

# Find all districts that intersect with the delivery zone
INTERSECTS districts '{"type":"Polygon","coordinates":[[[3.0,2.0],[7.0,1.5],[8.5,5.0],[6.0,7.0],[3.5,6.5],[3.0,2.0]]]}'

# Find nearest neighbors (KNN query)
# Syntax: NEARBY collection POINT lon lat [COUNT k] [RADIUS meters]
# At least one of COUNT or RADIUS must be specified

# Find 10 nearest vehicles
NEARBY fleet POINT 116.4 39.9 COUNT 10

# Find all vehicles within 1000 meters
NEARBY fleet POINT 116.4 39.9 RADIUS 1000

# Find 5 nearest vehicles within 2000 meters
NEARBY fleet POINT 116.4 39.9 COUNT 5 RADIUS 2000

# List all collections
KEYS

# Drop a collection
DROP fleet

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
