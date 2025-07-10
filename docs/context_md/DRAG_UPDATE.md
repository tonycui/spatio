# R-tree 可视化系统更新 - 拖拽和导航功能

## 更新内容

### 🆕 新增功能

#### 1. 拖拽平移功能
- **鼠标拖拽**：支持左键拖拽平移视图
- **视觉反馈**：拖拽时鼠标光标变为抓取状态
- **流畅体验**：实时更新视图，无延迟

#### 2. 键盘快捷键
- **R键**：重置视图到最佳状态
- **+/-键**：放大/缩小视图
- **方向键**：上下左右平移视图
- **智能处理**：在输入框中不响应快捷键

#### 3. 视图控制面板
- **重置视图按钮**：一键回到最佳视图状态
- **操作提示**：详细的交互操作说明
- **状态显示**：实时显示缩放级别

#### 4. 增强状态栏
- **坐标显示**：实时显示鼠标位置的世界坐标
- **缩放级别**：显示当前缩放百分比
- **状态信息**：操作状态和提示信息

### 🔧 技术改进

#### 拖拽实现
```javascript
// 添加拖拽状态管理
this.isDragging = false;
this.lastMouseX = 0;
this.lastMouseY = 0;

// 鼠标事件绑定
this.canvas.addEventListener('mousedown', (e) => {
    if (e.button === 0) { // 左键
        this.isDragging = true;
        this.lastMouseX = e.clientX;
        this.lastMouseY = e.clientY;
        this.canvas.style.cursor = 'grabbing';
    }
});

// 拖拽处理
if (this.isDragging) {
    const deltaX = e.clientX - this.lastMouseX;
    const deltaY = e.clientY - this.lastMouseY;
    
    this.offsetX += deltaX;
    this.offsetY += deltaY;
    
    this.lastMouseX = e.clientX;
    this.lastMouseY = e.clientY;
    
    this.render();
}
```

#### 键盘导航
```javascript
// 键盘事件处理
handleKeyDown(e) {
    // 避免在输入框中响应
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
        return;
    }
    
    switch (e.key.toLowerCase()) {
        case 'r': this.resetView(); break;
        case '+': this.zoomIn(); break;
        case '-': this.zoomOut(); break;
        case 'arrowup': this.panUp(); break;
        // ... 其他快捷键
    }
}
```

### 🎯 用户体验提升

#### 1. 解决了原问题
- ✅ **拖拽平移**：彻底解决了缩放后无法移动视图的问题
- ✅ **多种导航方式**：鼠标、键盘、按钮多种操作方式
- ✅ **快速重置**：一键回到最佳视图状态

#### 2. 交互体验优化
- **视觉反馈**：光标状态、缩放级别显示
- **操作提示**：详细的使用说明
- **智能处理**：避免快捷键与输入冲突

#### 3. 状态信息丰富
- **实时坐标**：鼠标位置的世界坐标
- **缩放级别**：当前缩放百分比
- **操作状态**：系统状态和提示

### 📱 完整操作方式

#### 鼠标操作
- **滚轮**：缩放视图
- **左键拖拽**：平移视图
- **鼠标移动**：显示坐标

#### 键盘操作
- **R键**：重置视图
- **+/-键**：放大/缩小
- **方向键**：平移视图

#### 按钮操作
- **重置视图**：回到最佳视图
- **加载数据**：导入JSON数据
- **可视化选项**：控制显示内容

### 🚀 使用方法

1. **加载数据**：
   - 点击"加载示例数据"快速体验
   - 或粘贴JSON数据到输入框

2. **导航视图**：
   - 滚轮缩放到合适级别
   - 拖拽鼠标平移查看不同区域
   - 使用方向键精确平移

3. **重置视图**：
   - 点击"重置视图"按钮
   - 或按R键快速重置

4. **查看信息**：
   - 状态栏显示当前缩放级别
   - 鼠标移动查看世界坐标
   - 树结构面板显示详细信息

### 🎉 问题解决

原问题：**"前端界面有问题，无法左右移动，所以在放大的时候，容易被移除界面"**

解决方案：
- ✅ 添加完整的拖拽平移功能
- ✅ 支持键盘方向键平移
- ✅ 提供重置视图功能
- ✅ 增加多种导航方式
- ✅ 实时状态反馈

现在用户可以自由地缩放和平移视图，不再有内容被移出界面无法查看的问题！
