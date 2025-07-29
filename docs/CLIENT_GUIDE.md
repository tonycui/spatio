# Geo42 客户端使用指南

> geo42-cli 是 Geo42 地理空间数据库的官方命令行客户端

## 📖 概述

geo42-cli 提供了与 Geo42 服务器交互的便捷方式，支持所有数据库操作命令。客户端兼容 RESP 协议，提供两种使用模式：命令行模式和交互模式。

## 🚀 安装和启动

### 安装客户端

```bash
# 从源码构建
cd geo42
cargo build --release

# 客户端位置
./target/release/geo42-cli
```

### 启动服务器

在使用客户端之前，需要先启动 Geo42 服务器：

```bash
# 启动服务器（默认端口 9851）
cargo run --bin geo42-server

# 或使用发布版本
./target/release/geo42-server
```

## 💻 使用模式

### 1. 命令行模式

直接执行单个命令：

```bash
geo42-cli [OPTIONS] [COMMAND]...
```

**示例：**
```bash
geo42-cli PING
geo42-cli SET fleet truck1 '{"type":"Point","coordinates":[116.3,39.9]}'
geo42-cli GET fleet truck1
```

### 2. 交互模式

进入交互式 REPL 环境：

```bash
geo42-cli --interactive
# 或
geo42-cli -i
```

**交互模式示例：**
```
$ geo42-cli -i
Connected to geo42 server at 127.0.0.1:9851
geo42> PING
PONG
geo42> SET fleet truck1 {"type":"Point","coordinates":[116.3,39.9]}
OK
geo42> GET fleet truck1
{"type":"Point","coordinates":[116.3,39.9]}
geo42> exit
```

## ⚙️ 命令行选项

### 连接选项

```bash
--host <HOST>        服务器主机名 [默认: 127.0.0.1]
-p, --port <PORT>    服务器端口 [默认: 9851]
-i, --interactive    进入交互模式
-h, --help          显示帮助信息
-V, --version       显示版本信息
```

### 使用示例

```bash
# 连接到远程服务器
geo42-cli --host 192.168.1.100 --port 9851 PING

# 连接到远程服务器的交互模式
geo42-cli --host production.example.com -p 9851 -i

# 显示版本信息
geo42-cli --version
```

## 📍 命令参考

### 连接和测试命令

#### PING
测试与服务器的连接。

**语法：**
```bash
PING
```

**示例：**
```bash
geo42-cli PING
# 输出: PONG
```

**用途：**
- 检查服务器是否在线
- 测试网络连接
- 健康检查

---

#### HELLO
执行协议握手，获取服务器信息。

**语法：**
```bash
HELLO
```

**示例：**
```bash
geo42-cli HELLO
# 输出: Hello from geo42 server!
```

**用途：**
- 协议版本确认
- 服务器信息获取
- 连接初始化

---

### 数据操作命令

#### SET
存储地理空间对象到指定的 collection。

**语法：**
```bash
SET <collection> <id> <geojson>
```

**参数：**
- `collection`: 集合名称（类似数据库表名）
- `id`: 对象的唯一标识符
- `geojson`: 符合 GeoJSON 标准的地理空间数据

**支持的 GeoJSON 类型：**
- Point（点）
- LineString（线串）
- Polygon（多边形）
- MultiPoint（多点）
- MultiLineString（多线串）
- MultiPolygon（多多边形）
- GeometryCollection（几何集合）
- Feature（要素）
- FeatureCollection（要素集合）

**示例：**

**存储点数据：**
```bash
geo42-cli SET fleet truck1 '{
  "type": "Point",
  "coordinates": [116.3974, 39.9093]
}'
# 输出: OK
```

**存储线串：**
```bash
geo42-cli SET routes route1 '{
  "type": "LineString",
  "coordinates": [
    [116.3974, 39.9093],
    [116.4074, 39.9193],
    [116.4174, 39.9293]
  ]
}'
# 输出: OK
```

**存储多边形：**
```bash
geo42-cli SET zones beijing_cbd '{
  "type": "Polygon",
  "coordinates": [[
    [116.3974, 39.9093],
    [116.4074, 39.9093],
    [116.4074, 39.9193],
    [116.3974, 39.9193],
    [116.3974, 39.9093]
  ]]
}'
# 输出: OK
```

**存储带属性的要素：**
```bash
geo42-cli SET pois restaurant1 '{
  "type": "Feature",
  "geometry": {
    "type": "Point",
    "coordinates": [116.3974, 39.9093]
  },
  "properties": {
    "name": "北京烤鸭店",
    "category": "restaurant",
    "rating": 4.5
  }
}'
# 输出: OK
```

**错误示例：**
```bash
# 参数数量错误
geo42-cli SET fleet
# 输出: ERR wrong number of arguments for 'SET' command. Expected 3, got 1

# 无效的 GeoJSON
geo42-cli SET fleet truck1 '{"invalid": "data"}'
# 输出: ERR invalid GeoJSON: missing 'type' field

# JSON 格式错误
geo42-cli SET fleet truck1 'invalid json'
# 输出: ERR invalid GeoJSON: expected value at line 1 column 1
```

---

#### GET
从指定 collection 获取地理空间对象。

**语法：**
```bash
GET <collection> <id>
```

**参数：**
- `collection`: 集合名称
- `id`: 对象标识符

**示例：**

**获取存在的对象：**
```bash
geo42-cli GET fleet truck1
# 输出: {"type":"Point","coordinates":[116.3974,39.9093]}
```

**获取不存在的对象：**
```bash
geo42-cli GET fleet nonexistent
# 输出: (nil)
```

**错误示例：**
```bash
# 参数数量错误
geo42-cli GET fleet
# 输出: ERR wrong number of arguments for 'GET' command. Expected 2, got 1
```

---

## 🔧 高级用法

### 批量操作

**在交互模式中执行多个命令：**
```bash
geo42-cli -i
geo42> SET fleet truck1 {"type":"Point","coordinates":[116.3,39.9]}
OK
geo42> SET fleet truck2 {"type":"Point","coordinates":[116.4,39.8]}
OK
geo42> GET fleet truck1
{"type":"Point","coordinates":[116.3,39.9]}
geo42> GET fleet truck2
{"type":"Point","coordinates":[116.4,39.8]}
```

### 脚本化使用

**在脚本中使用客户端：**
```bash
#!/bin/bash

# 检查服务器状态
if geo42-cli PING > /dev/null 2>&1; then
    echo "服务器在线"
    
    # 批量导入数据
    geo42-cli SET fleet truck1 '{"type":"Point","coordinates":[116.3,39.9]}'
    geo42-cli SET fleet truck2 '{"type":"Point","coordinates":[116.4,39.8]}'
    
    echo "数据导入完成"
else
    echo "服务器离线"
    exit 1
fi
```

### 环境变量

可以使用环境变量设置默认连接参数：

```bash
export GEO42_HOST=192.168.1.100
export GEO42_PORT=9851

# 现在客户端会使用这些默认值
geo42-cli PING
```

## 📊 输出格式

### 成功响应

| 命令 | 成功输出 | 说明 |
|------|----------|------|
| PING | `PONG` | 连接正常 |
| HELLO | `Hello from geo42 server!` | 服务器问候 |
| SET | `OK` | 数据存储成功 |
| GET | `<geojson>` 或 `(nil)` | 返回数据或空值 |

### 错误响应

所有错误都以 `ERR` 开头，包含详细的错误信息：

```bash
ERR wrong number of arguments for 'SET' command. Expected 3, got 2
ERR invalid GeoJSON: missing 'type' field
ERR invalid collection ID: expected string
ERR failed to store: <具体错误原因>
```

## 🐛 故障排除

### 常见问题

**1. 连接失败**
```bash
geo42-cli PING
# 输出: Error: Connection refused (os error 61)
```
**解决方案：**
- 确认服务器已启动
- 检查主机名和端口是否正确
- 检查防火墙设置

**2. 命令不识别**
```bash
geo42-cli UNKNOWN_COMMAND
# 输出: ERR unknown command 'UNKNOWN_COMMAND'
```
**解决方案：**
- 检查命令拼写
- 参考本文档的命令列表

**3. GeoJSON 格式错误**
```bash
geo42-cli SET fleet truck1 '{"type":"Point"}'
# 输出: ERR invalid GeoJSON: missing coordinates
```
**解决方案：**
- 验证 GeoJSON 格式
- 使用在线 GeoJSON 验证工具
- 确保包含必需的字段

### 调试技巧

**1. 使用详细模式（计划中）**
```bash
geo42-cli --verbose PING
```

**2. 检查连接参数**
```bash
geo42-cli --help
```

**3. 测试简单命令**
```bash
# 先测试 PING，确认连接正常
geo42-cli PING

# 再测试数据操作
geo42-cli SET test simple '{"type":"Point","coordinates":[0,0]}'
```

## 📝 最佳实践

### 1. 命名约定

**Collection 命名：**
- 使用描述性名称：`fleet`, `sensors`, `boundaries`
- 避免特殊字符，使用字母、数字和下划线
- 保持一致的命名风格

**对象 ID 命名：**
- 使用唯一标识符：`truck_001`, `sensor_north_01`
- 避免空格和特殊字符
- 考虑使用 UUID 对于全局唯一性

### 2. GeoJSON 格式

**确保数据完整性：**
```bash
# ✅ 正确的点格式
{"type":"Point","coordinates":[longitude,latitude]}

# ✅ 正确的多边形格式（注意闭合）
{
  "type":"Polygon",
  "coordinates":[[
    [116.3,39.9], [116.4,39.9], 
    [116.4,40.0], [116.3,40.0], 
    [116.3,39.9]
  ]]
}
```

### 3. 性能优化

**批量操作：**
- 在交互模式中执行多个命令
- 避免频繁建立连接

**数据组织：**
- 合理划分 collection
- 避免单个 collection 过大

## 🔄 版本兼容性

| 客户端版本 | 服务器版本 | 兼容性 |
|------------|------------|--------|
| 0.1.x | 0.1.x | ✅ 完全兼容 |

## 📞 获取帮助

- **命令行帮助**: `geo42-cli --help`
- **GitHub Issues**: [项目 Issues 页面]
- **文档**: [完整文档链接]
- **社区**: [社区讨论区]

---

*最后更新: 2025年7月29日*
