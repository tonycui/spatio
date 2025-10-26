# AOF 持久化实现计划

## 概述

基于 JSON Lines 格式实现 AOF (Append-Only File) 持久化系统，支持写入、恢复和配置化的同步策略。

**预计工作量**: 8-12 天

---

## Phase 1: 核心数据结构（1-2天）✅ 已完成

### 目标
定义 AOF 系统的基础数据结构和配置。

### 任务清单
- [x] 创建 `rtree/algorithms/aof.rs` 文件
- [x] 实现 `AofCommand` 枚举（支持 INSERT/DELETE/DROP）
- [x] 实现 `AofSyncPolicy` 枚举（Always/EverySecond/No）
- [x] 实现 `AofConfig` 结构体
- [x] 实现 `AofError` 错误类型
- [x] 添加基础序列化/反序列化测试

### 验收标准
- ✅ 所有数据结构编译通过
- ✅ `AofCommand` 可以序列化为 JSON
- ✅ JSON 反序列化测试通过
- ✅ 所有 10 个单元测试通过

### 核心结构
```rust
// AofCommand: INSERT, DELETE, DROP
// AofSyncPolicy: Always, EverySecond, No
// AofConfig: file_path, sync_policy, enabled
// AofError: Io, Json, InvalidCommand, etc.
```

---

## Phase 2: AOF Writer 实现（2-3天）

### 目标
实现 AOF 写入器，支持 JSON Lines 格式和三种同步策略。

### 任务清单
- [ ] 实现 `AofWriter` 结构体
- [ ] 实现 `append()` 方法（写入一行 JSON）
- [ ] 实现 `sync_if_needed()` 方法（根据策略同步）
- [ ] 实现 `flush()` 方法（手动刷新）
- [ ] 在 `Drop` trait 中自动刷新
- [ ] 添加单元测试（基本写入、JSON Lines 格式、同步策略）

### 验收标准
- ✅ 可以正确写入 AOF 文件
- ✅ JSON Lines 格式正确（每行一条 JSON）
- ✅ 三种同步策略都能正常工作
- ✅ 所有测试通过

### 关键实现
```rust
// AofWriter::new() - 打开文件（追加模式）
// AofWriter::append() - 序列化 + 写入 + 同步
// 同步策略:
//   - Always: 立即 flush + sync_data
//   - EverySecond: 每秒 flush + sync_data
//   - No: 每 1MB flush（不 fsync）
```

---

## Phase 3: AOF Reader 实现（2天）

### 目标
实现 AOF 读取器，支持逐行读取和容错恢复。

### 任务清单
- [ ] 实现 `AofReader` 结构体
- [ ] 实现 `open()` 方法（打开 AOF 文件）
- [ ] 实现 `read_next()` 方法（读取下一条命令）
- [ ] 实现 `recover_all()` 方法（恢复所有命令，容错）
- [ ] 实现 `RecoveryResult` 结构体（恢复报告）
- [ ] 添加测试（基本读取、容错恢复、损坏文件处理）

### 验收标准
- ✅ 可以正确读取 AOF 文件
- ✅ 支持容错恢复（跳过损坏的行）
- ✅ 提供详细的恢复报告（成功率、错误列表）
- ✅ 所有测试通过

### 关键实现
```rust
// AofReader::open() - 打开文件
// AofReader::read_next() - 逐行读取 JSON
// AofReader::recover_all() - 容错恢复
// RecoveryResult - 包含 commands, errors, success_rate
```

---

## Phase 4: 集成到 Storage（2-3天）

### 目标
将 AOF 系统集成到现有的 `Storage` 层，实现自动记录和启动恢复。

### 任务清单
- [ ] 修改 `Storage` 结构体（添加 `aof_writer` 字段）
- [ ] 实现 `Storage::with_aof_config()` 构造函数
- [ ] 修改 `insert()` 方法（先记录 AOF，再执行插入）
- [ ] 修改 `delete()` 方法（记录 AOF）
- [ ] 修改 `drop()` 方法（记录 AOF）
- [ ] 实现 `Storage::recover_from_aof()` 方法（启动时恢复）
- [ ] 在 `rtree/algorithms/mod.rs` 中导出 AOF 模块
- [ ] 添加集成测试

### 验收标准
- ✅ `insert`/`delete`/`drop` 操作自动记录到 AOF
- ✅ 服务启动时能从 AOF 恢复数据
- ✅ AOF 写入失败不影响操作执行（可配置）
- ✅ 集成测试通过

### 关键实现
```rust
// Storage::with_aof_config() - 初始化 AOF Writer
// Storage::insert() - 先 AOF 后执行
// Storage::recover_from_aof() - 启动恢复
// 错误处理: AOF 失败时如何处理？
```

---

## Phase 5: 集成测试和文档（1-2天）

### 目标
完成端到端测试、性能测试和文档。

### 任务清单
- [ ] 端到端测试（写入 → 重启 → 恢复 → 验证）
- [ ] 性能测试（对比三种同步策略）
- [ ] 压力测试（大量数据写入和恢复）
- [ ] 更新 `ROADMAP.md`（标记 AOF 完成）
- [ ] 创建 AOF 配置文档（`docs/aof-configuration.md`）
- [ ] 添加使用示例（`examples/aof_example.rs`）

### 验收标准
- ✅ 端到端测试通过
- ✅ 性能测试完成（记录三种策略的 TPS）
- ✅ 压力测试通过（100万+ 命令）
- ✅ 文档完整清晰

### 测试场景
```rust
// 端到端测试:
//   1. 创建 Storage 并写入 1000 条数据
//   2. 关闭 Storage
//   3. 重新启动并恢复
//   4. 验证 1000 条数据完整

// 性能测试:
//   - Always: 预期 ~1000-5000 TPS
//   - EverySecond: 预期 ~50000-100000 TPS
//   - No: 预期 ~100000+ TPS
```

---

## 验收清单

### 功能完整性
- [ ] AOF Writer 正确写入 JSON Lines 格式
- [ ] AOF Reader 正确恢复数据
- [ ] 三种同步策略都能正常工作
- [ ] 容错恢复机制有效（跳过损坏行）
- [ ] 集成到 Storage 无缝衔接
- [ ] 启动时自动恢复

### 测试覆盖
- [ ] 单元测试全部通过
- [ ] 集成测试全部通过
- [ ] 端到端测试通过
- [ ] 性能测试完成

### 文档完整性
- [ ] 代码注释完整
- [ ] 配置文档完整
- [ ] 使用示例清晰
- [ ] ROADMAP 更新

---

## 技术细节

### JSON Lines 格式示例
```jsonl
{"ts":1698234567890123456,"cmd":"INSERT","collection":"cities","key":"beijing","bbox":[116.0,39.0,117.0,40.0],"geojson":"{\"type\":\"Point\",\"coordinates\":[116.4,39.9]}"}
{"ts":1698234567890123457,"cmd":"DELETE","collection":"cities","key":"beijing","bbox":[116.0,39.0,117.0,40.0]}
{"ts":1698234567890123458,"cmd":"DROP","collection":"cities"}
```

### 同步策略对比

| 策略 | 性能 | 安全性 | 数据丢失风险 | 推荐场景 |
|------|------|--------|-------------|----------|
| Always | 最低 | 最高 | 几乎为 0 | 金融、关键数据 |
| EverySecond | 高 | 高 | 最多 1 秒 | **默认推荐** |
| No | 最高 | 低 | 最多 30 秒 | 可容忍数据丢失 |

### 文件结构
```
data/
├── appendonly.aof      # AOF 日志文件
└── dump.rdb            # RDB 快照（未来实现）
```

---

## 下一步计划

完成 AOF 后的后续工作：

1. **AOF 重写**（可选优化）
   - 当 AOF 文件过大时，生成压缩版本
   - 只保留当前状态的最小命令集

2. **RDB 快照**（后续 Phase）
   - 定期全量快照
   - 快速恢复（相比 AOF）
   - 混合模式：RDB + AOF

3. **监控和指标**
   - AOF 文件大小监控
   - 写入 TPS 统计
   - 恢复时间统计

---

## 参考资料

- Redis AOF 文档: https://redis.io/docs/management/persistence/
- JSON Lines 格式: https://jsonlines.org/
- Rust `serde_json` 文档: https://docs.rs/serde_json/
