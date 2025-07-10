# R-tree 空间索引实现与可视化

一个基于 Rust 的 R-tree 空间索引数据结构实现，严格遵循 Gut84.pdf 论文标准，包含完整的可视化系统。

## 🌟 项目特点

- **严格的学术实现**：完全按照 Gut84.pdf 论文实现二次分裂算法
- **完整的功能支持**：插入、搜索、删除、节点分裂、树压缩
- **可视化系统**：直观展示 R-tree 的空间分布和树结构
- **丰富的测试数据**：多种分布模式的示例数据集
- **高质量代码**：32个单元测试，100% 测试通过率

## 🏗️ 项目结构

```
rtree/
├── src/                         # 🦀 Rust 核心实现
│   ├── lib.rs                   # 库入口
│   ├── rtree.rs                 # R-tree 主结构
│   ├── node.rs                  # 节点结构定义
│   ├── rectangle.rs             # 矩形几何操作
│   └── algorithms.rs            # 核心算法实现
├── examples/                    # 📊 示例程序
│   └── generate_json.rs         # JSON 数据生成器
├── visualization/               # 🎨 可视化系统
│   ├── example_data/            # 📄 测试数据集
│   │   ├── simple_tree.json     # 简单网格分布 (20点)
│   │   ├── uniform_data.json    # 均匀分布 (20点)
│   │   ├── complex_tree.json    # 复杂聚集分布 (24点)
│   │   └── clustered_data.json  # 聚类+噪声分布 (28点)
│   └── web/                     # 🌐 前端界面
│       ├── index.html           # 主页面
│       ├── style.css            # 样式文件
│       └── rtree-visualizer.js  # 可视化引擎
├── docs/                        # 📚 项目文档
│   └── context_md/              # 开发历程文档
└── README.md                    # 📖 项目说明
```

## 🚀 快速开始

### 1. 环境要求

- Rust 1.70+ 
- 现代浏览器（Chrome、Firefox、Safari、Edge）

### 2. 运行测试

```bash
# 克隆项目
git clone <repository-url>
cd rtree

# 运行所有测试
cargo test

# 运行测试并显示输出
cargo test -- --nocapture
```

### 3. 生成可视化数据

```bash
# 生成示例数据
cargo run --example generate_json

# 检查生成的数据文件
ls visualization/example_data/
```

## 🎨 可视化系统使用

### 方式1：直接打开（推荐新手）

1. **打开界面**：
   ```bash
   # 直接用浏览器打开
   open visualization/web/index.html
   # 或者双击该文件
   ```

2. **加载数据**：
   - 复制 `visualization/example_data/` 中任意 JSON 文件的内容
   - 粘贴到网页的"JSON数据输入"文本框
   - 点击"加载数据"按钮

3. **功能限制**：
   - ✅ 基础可视化功能完全正常
   - ✅ 手动数据输入正常
   - ❌ 示例数据按钮无法使用（浏览器安全限制）

### 方式2：HTTP服务器（完整功能）

1. **启动服务器**（选择一种方式）：

   **Python（推荐）**：
   ```bash
   cd visualization/web
   python3 -m http.server 8000
   ```

   **Node.js**：
   ```bash
   npm install -g http-server
   cd visualization/web
   http-server -p 8000
   ```

   **Rust**：
   ```bash
   cargo install basic-http-server
   cd visualization/web
   basic-http-server .
   ```

2. **打开界面**：
   ```bash
   # 浏览器访问
   open http://localhost:8000
   ```

3. **完整功能**：
   - ✅ 所有可视化功能
   - ✅ 示例数据一键加载
   - ✅ 完整的交互体验

### 🎮 界面操作指南

#### 数据加载
- **示例数据按钮**：一键加载预置的测试数据
- **手动输入**：复制粘贴 JSON 数据到输入框

#### 视图控制
- **鼠标滚轮**：缩放视图
- **鼠标拖拽**：平移视图
- **重置视图按钮**：恢复到最佳视图

#### 键盘快捷键
- **R键**：重置视图
- **+/-键**：放大/缩小
- **方向键**：精确平移视图

#### 显示选项
- **显示数据点**：切换数据点的显示
- **显示MBR**：切换最小边界矩形的显示
- **显示树结构**：切换右侧树结构图

## 📊 示例数据集说明

| 数据集 | 数据点数 | 分布模式 | 适用场景 |
|--------|----------|----------|----------|
| `simple_tree.json` | 20个 | 网格分布 | 初学者，理解基本分割 |
| `uniform_data.json` | 20个 | 均匀分布 | 观察规整的树结构 |
| `complex_tree.json` | 24个 | 区域聚集 | 理解多层索引关系 |
| `clustered_data.json` | 28个 | 聚集+噪声 | 研究真实数据模式 |

## 🔧 开发指南

### 添加新功能

1. **核心算法**：在 `src/algorithms.rs` 中实现
2. **数据结构**：修改 `src/node.rs` 或 `src/rtree.rs`
3. **可视化**：扩展 `visualization/web/rtree-visualizer.js`

### 生成新的测试数据

```rust
// 编辑 examples/generate_json.rs
fn generate_custom_data() -> RTree {
    let mut rtree = RTree::new(4);
    // 添加您的数据点
    rtree.insert(Rectangle::new(x, y, x+w, y+h), data);
    rtree
}
```

### 运行特定测试

```bash
# 运行特定测试
cargo test test_insert

# 运行condense_tree相关测试
cargo test condense_tree

# 显示测试输出
cargo test test_insert -- --nocapture
```

## 📈 性能特征

- **插入复杂度**：O(log n) 平均情况
- **搜索复杂度**：O(log n) 点查询，O(m) 范围查询
- **删除复杂度**：O(log n) 平均情况
- **空间复杂度**：O(n)
- **适用规模**：当前实现适合 < 10,000 节点的中小规模应用

## 🎓 算法实现细节

### 核心算法
- **插入算法**：严格遵循论文 Algorithm Insert
- **分裂算法**：二次分裂 (Quadratic Split)
  - PickSeeds：选择最差的种子对
  - PickNext：选择最适合分配的条目
- **删除算法**：包含节点下溢处理和树压缩
- **搜索算法**：递归搜索，支持范围查询

### 测试覆盖
- **32个单元测试**，覆盖所有核心功能
- **专项测试**：condense_tree、分裂算法、MBR更新
- **边界情况测试**：空树、单节点、下溢等

## 🤝 贡献指南

1. Fork 本项目
2. 创建功能分支：`git checkout -b feature/new-feature`
3. 提交更改：`git commit -am 'Add new feature'`
4. 推送分支：`git push origin feature/new-feature`
5. 提交 Pull Request

## 📄 许可证

[MIT License](LICENSE)

## 🙏 致谢

- 基于 Gut84.pdf 论文的经典 R-tree 算法
- 感谢开源社区的贡献和支持

---

**快速体验**：直接打开 `visualization/web/index.html`，复制 `visualization/example_data/simple_tree.json` 的内容进行体验！
