/**
 * R-tree 可视化器
 * 用于解析和可视化从Rust R-tree导出的JSON数据
 */
class RTreeVisualizer {
    constructor() {
        this.treeData = null;
        this.canvas = null;
        this.ctx = null;
        this.scale = 1;
        this.offsetX = 0;
        this.offsetY = 0;
        
        // 拖拽状态
        this.isDragging = false;
        this.lastMouseX = 0;
        this.lastMouseY = 0;
        
        this.colors = {
            index_node: '#7c3aed',
            leaf_node: '#059669',
            data_point: '#dc2626',
            mbr_border: '#6b7280',
            background: '#ffffff'
        };
        
        this.init();
    }

    /**
     * 初始化可视化器
     */
    init() {
        this.canvas = document.getElementById('spatial-canvas');
        this.ctx = this.canvas.getContext('2d');
        
        // 绑定事件监听器
        this.bindEventListeners();
        
        // 设置初始状态
        this.updateStatus('就绪');
        this.clearCanvas();
    }

    /**
     * 绑定事件监听器
     */
    bindEventListeners() {
        // 加载数据按钮
        document.getElementById('load-data-btn').addEventListener('click', () => {
            this.loadDataFromInput();
        });

        // 示例数据按钮
        document.querySelectorAll('.example-buttons .btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const filePath = btn.getAttribute('data-file');
                this.loadExampleData(filePath);
            });
        });

        // 重置视图按钮
        document.getElementById('reset-view-btn').addEventListener('click', () => {
            this.resetView();
        });

        // 可视化选项复选框
        const checkboxes = [
            'show-mbr', 'show-data-points', 
            'show-node-labels', 'show-tree-structure'
        ];
        
        checkboxes.forEach(id => {
            document.getElementById(id).addEventListener('change', () => {
                if (this.treeData) {
                    this.render();
                }
            });
        });

        // 画布鼠标事件
        this.canvas.addEventListener('mousemove', (e) => {
            this.handleMouseMove(e);
        });

        this.canvas.addEventListener('wheel', (e) => {
            this.handleWheel(e);
        });

        // 拖拽事件
        this.canvas.addEventListener('mousedown', (e) => {
            this.handleMouseDown(e);
        });

        this.canvas.addEventListener('mouseup', (e) => {
            this.handleMouseUp(e);
        });

        this.canvas.addEventListener('mouseleave', (e) => {
            this.handleMouseUp(e); // 鼠标离开画布时结束拖拽
        });

        // 防止右键菜单
        this.canvas.addEventListener('contextmenu', (e) => {
            e.preventDefault();
        });

        // 设置画布光标样式
        this.canvas.style.cursor = 'grab';

        // 键盘事件
        document.addEventListener('keydown', (e) => {
            this.handleKeyDown(e);
        });
    }

    /**
     * 从输入框加载JSON数据
     */
    loadDataFromInput() {
        const input = document.getElementById('json-input');
        const jsonText = input.value.trim();

        if (!jsonText) {
            this.updateStatus('错误：请输入JSON数据');
            return;
        }

        try {
            const data = JSON.parse(jsonText);
            this.loadTreeData(data);
        } catch (error) {
            this.updateStatus(`错误：JSON解析失败 - ${error.message}`);
            console.error('JSON解析错误:', error);
        }
    }

    /**
     * 加载树数据
     */
    loadTreeData(data) {
        try {
            // 验证数据结构
            if (!this.validateTreeData(data)) {
                this.updateStatus('错误：无效的树数据结构');
                return;
            }

            this.treeData = data;
            this.updateTreeInfo();
            this.updateTreeStructureView();
            this.calculateOptimalView();
            this.render();
            this.updateStatus('数据加载成功');
        } catch (error) {
            this.updateStatus(`错误：数据加载失败 - ${error.message}`);
            console.error('数据加载错误:', error);
        }
    }

    /**
     * 加载示例数据文件
     */
    async loadExampleData(filePath) {
        try {
            this.updateStatus('加载示例数据...');
            
            const response = await fetch(filePath);
            if (!response.ok) {
                throw new Error(`无法加载文件: ${response.status}`);
            }
            
            const jsonText = await response.text();
            const data = JSON.parse(jsonText);
            
            // 更新输入框内容
            document.getElementById('json-input').value = jsonText;
            
            // 加载数据
            this.loadData(data);
            
        } catch (error) {
            this.updateStatus(`错误：示例数据加载失败 - ${error.message}`);
            console.error('示例数据加载错误:', error);
        }
    }

    /**
     * 验证树数据结构
     */
    validateTreeData(data) {
        return data && 
               typeof data === 'object' && 
               data.config && 
               typeof data.config.max_entries === 'number' &&
               typeof data.config.min_entries === 'number';
    }

    /**
     * 更新树信息显示
     */
    updateTreeInfo() {
        if (!this.treeData) return;

        const config = this.treeData.config;
        const stats = this.calculateTreeStats();

        document.getElementById('max-entries').textContent = config.max_entries;
        document.getElementById('min-entries').textContent = config.min_entries;
        document.getElementById('tree-depth').textContent = stats.depth;
        document.getElementById('total-nodes').textContent = stats.totalNodes;
        document.getElementById('total-data').textContent = stats.totalData;
    }

    /**
     * 计算树统计信息
     */
    calculateTreeStats() {
        if (!this.treeData || !this.treeData.root) {
            return { depth: 0, totalNodes: 0, totalData: 0 };
        }

        let totalNodes = 0;
        let totalData = 0;
        let maxDepth = 0;

        const traverse = (node, depth = 0) => {
            totalNodes++;
            maxDepth = Math.max(maxDepth, depth);

            if (node.node_type === 'Leaf') {
                totalData += node.data_entries.length;
            } else {
                node.child_nodes.forEach(child => {
                    traverse(child, depth + 1);
                });
            }
        };

        traverse(this.treeData.root);
        
        return {
            depth: maxDepth + 1,
            totalNodes,
            totalData
        };
    }

    /**
     * 更新树结构视图
     */
    updateTreeStructureView() {
        const container = document.getElementById('tree-structure');
        
        if (!this.treeData || !this.treeData.root) {
            container.innerHTML = '<div class="placeholder">无树数据</div>';
            return;
        }

        const html = this.generateTreeHTML(this.treeData.root, 0);
        container.innerHTML = html;
    }

    /**
     * 生成树结构HTML
     */
    generateTreeHTML(node, level) {
        const indent = '  '.repeat(level);
        const nodeClass = node.node_type === 'Leaf' ? 'leaf-node' : 'index-node';
        const nodeType = node.node_type === 'Leaf' ? '叶子节点' : '索引节点';
        
        let html = `<div class="tree-node ${nodeClass}">`;
        html += `<div class="node-header">${indent}${nodeType} (级别 ${node.level})</div>`;
        html += `<div class="node-details">MBR: [${node.mbr.min[0].toFixed(1)}, ${node.mbr.min[1].toFixed(1)}] - [${node.mbr.max[0].toFixed(1)}, ${node.mbr.max[1].toFixed(1)}]</div>`;

        if (node.node_type === 'Leaf' && node.data_entries.length > 0) {
            html += '<div class="node-details">数据条目:</div>';
            node.data_entries.forEach(entry => {
                html += `<div class="data-entry">${indent}  数据 ${entry.data}: [${entry.mbr.min[0]}, ${entry.mbr.min[1]}] - [${entry.mbr.max[0]}, ${entry.mbr.max[1]}]</div>`;
            });
        }
        
        html += '</div>';

        if (node.child_nodes && node.child_nodes.length > 0) {
            node.child_nodes.forEach(child => {
                html += this.generateTreeHTML(child, level + 1);
            });
        }

        return html;
    }

    /**
     * 计算最佳视图
     */
    calculateOptimalView() {
        if (!this.treeData || !this.treeData.root) return;

        const bounds = this.calculateBounds();
        const canvasWidth = this.canvas.width;
        const canvasHeight = this.canvas.height;
        
        // 计算缩放比例和偏移量，确保所有数据都可见
        const scaleX = (canvasWidth - 100) / (bounds.maxX - bounds.minX);
        const scaleY = (canvasHeight - 100) / (bounds.maxY - bounds.minY);
        
        this.scale = Math.min(scaleX, scaleY, 1); // 不要放大超过1:1
        this.offsetX = 50 - bounds.minX * this.scale;
        this.offsetY = 50 - bounds.minY * this.scale;
    }

    /**
     * 计算数据边界
     */
    calculateBounds() {
        if (!this.treeData || !this.treeData.root) {
            return { minX: 0, minY: 0, maxX: 100, maxY: 100 };
        }

        const root = this.treeData.root;
        return {
            minX: root.mbr.min[0],
            minY: root.mbr.min[1],
            maxX: root.mbr.max[0],
            maxY: root.mbr.max[1]
        };
    }

    /**
     * 渲染整个可视化
     */
    render() {
        if (!this.treeData) return;

        this.clearCanvas();
        
        const showMBR = document.getElementById('show-mbr').checked;
        const showDataPoints = document.getElementById('show-data-points').checked;
        const showLabels = document.getElementById('show-node-labels').checked;

        if (this.treeData.root) {
            this.renderNode(this.treeData.root, showMBR, showDataPoints, showLabels);
        }
    }

    /**
     * 渲染节点
     */
    renderNode(node, showMBR, showDataPoints, showLabels) {
        // 渲染MBR边界
        if (showMBR) {
            this.renderMBR(node.mbr, node.node_type === 'Leaf' ? this.colors.leaf_node : this.colors.index_node);
        }

        // 渲染数据点（仅叶子节点）
        if (node.node_type === 'Leaf' && showDataPoints) {
            node.data_entries.forEach(entry => {
                this.renderDataPoint(entry);
            });
        }

        // 渲染节点标签
        if (showLabels) {
            this.renderNodeLabel(node);
        }

        // 递归渲染子节点
        if (node.child_nodes) {
            node.child_nodes.forEach(child => {
                this.renderNode(child, showMBR, showDataPoints, showLabels);
            });
        }
    }

    /**
     * 渲染MBR边界
     */
    renderMBR(mbr, color) {
        const x1 = this.transformX(mbr.min[0]);
        const y1 = this.transformY(mbr.min[1]);
        const x2 = this.transformX(mbr.max[0]);
        const y2 = this.transformY(mbr.max[1]);

        this.ctx.strokeStyle = color;
        this.ctx.lineWidth = 2;
        this.ctx.setLineDash([]);
        this.ctx.strokeRect(x1, y1, x2 - x1, y2 - y1);

        // 添加半透明填充
        this.ctx.fillStyle = color + '20'; // 添加透明度
        this.ctx.fillRect(x1, y1, x2 - x1, y2 - y1);
    }

    /**
     * 渲染数据点
     */
    renderDataPoint(entry) {
        const centerX = this.transformX((entry.mbr.min[0] + entry.mbr.max[0]) / 2);
        const centerY = this.transformY((entry.mbr.min[1] + entry.mbr.max[1]) / 2);

        // 绘制数据点
        this.ctx.fillStyle = this.colors.data_point;
        this.ctx.beginPath();
        this.ctx.arc(centerX, centerY, 4, 0, 2 * Math.PI);
        this.ctx.fill();

        // 绘制数据标签
        this.ctx.fillStyle = '#000';
        this.ctx.font = '12px Arial';
        this.ctx.textAlign = 'center';
        this.ctx.fillText(entry.data.toString(), centerX, centerY - 8);
    }

    /**
     * 渲染节点标签
     */
    renderNodeLabel(node) {
        const centerX = this.transformX((node.mbr.min[0] + node.mbr.max[0]) / 2);
        const centerY = this.transformY((node.mbr.min[1] + node.mbr.max[1]) / 2);

        const label = node.node_type === 'Leaf' ? `L${node.level}` : `I${node.level}`;
        
        // 背景
        this.ctx.fillStyle = 'rgba(255, 255, 255, 0.8)';
        this.ctx.fillRect(centerX - 15, centerY + 5, 30, 16);

        // 文字
        this.ctx.fillStyle = '#333';
        this.ctx.font = '11px Arial';
        this.ctx.textAlign = 'center';
        this.ctx.fillText(label, centerX, centerY + 16);
    }

    /**
     * 坐标变换：X
     */
    transformX(x) {
        return x * this.scale + this.offsetX;
    }

    /**
     * 坐标变换：Y
     */
    transformY(y) {
        return y * this.scale + this.offsetY;
    }

    /**
     * 反向坐标变换：X
     */
    inverseTransformX(x) {
        return (x - this.offsetX) / this.scale;
    }

    /**
     * 反向坐标变换：Y
     */
    inverseTransformY(y) {
        return (y - this.offsetY) / this.scale;
    }

    /**
     * 清空画布
     */
    clearCanvas() {
        this.ctx.fillStyle = this.colors.background;
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
        
        // 绘制网格
        this.drawGrid();
    }

    /**
     * 绘制网格
     */
    drawGrid() {
        this.ctx.strokeStyle = '#f0f0f0';
        this.ctx.lineWidth = 1;
        this.ctx.setLineDash([]);

        const gridSize = 50;
        
        // 垂直线
        for (let x = 0; x < this.canvas.width; x += gridSize) {
            this.ctx.beginPath();
            this.ctx.moveTo(x, 0);
            this.ctx.lineTo(x, this.canvas.height);
            this.ctx.stroke();
        }

        // 水平线
        for (let y = 0; y < this.canvas.height; y += gridSize) {
            this.ctx.beginPath();
            this.ctx.moveTo(0, y);
            this.ctx.lineTo(this.canvas.width, y);
            this.ctx.stroke();
        }
    }

    /**
     * 处理鼠标滚轮缩放
     */
    handleWheel(e) {
        e.preventDefault();
        
        const rect = this.canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        
        const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
        const newScale = this.scale * zoomFactor;
        
        // 限制缩放范围
        if (newScale < 0.1 || newScale > 5) return;
        
        // 更新偏移量以保持鼠标位置不变
        this.offsetX = x - (x - this.offsetX) * zoomFactor;
        this.offsetY = y - (y - this.offsetY) * zoomFactor;
        this.scale = newScale;
        
        // 更新缩放级别显示
        document.getElementById('zoom-display').textContent = 
            `缩放: ${(this.scale * 100).toFixed(0)}%`;
        
        this.render();
    }

    /**
     * 处理鼠标按下（开始拖拽）
     */
    handleMouseDown(e) {
        if (e.button === 0) { // 左键
            this.isDragging = true;
            this.lastMouseX = e.clientX;
            this.lastMouseY = e.clientY;
            this.canvas.style.cursor = 'grabbing';
        }
    }

    /**
     * 处理鼠标释放（结束拖拽）
     */
    handleMouseUp(e) {
        if (this.isDragging) {
            this.isDragging = false;
            this.canvas.style.cursor = 'grab';
        }
    }

    /**
     * 处理鼠标移动（拖拽时更新视图）
     */
    handleMouseMove(e) {
        const rect = this.canvas.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        
        // 更新坐标显示
        const worldX = this.inverseTransformX(x);
        const worldY = this.inverseTransformY(y);
        document.getElementById('coordinates-display').textContent = 
            `坐标: (${worldX.toFixed(2)}, ${worldY.toFixed(2)})`;

        // 更新缩放级别显示
        document.getElementById('zoom-display').textContent = 
            `缩放: ${(this.scale * 100).toFixed(0)}%`;

        // 处理拖拽
        if (this.isDragging) {
            const deltaX = e.clientX - this.lastMouseX;
            const deltaY = e.clientY - this.lastMouseY;
            
            // 更新偏移量
            this.offsetX += deltaX;
            this.offsetY += deltaY;
            
            // 记录当前鼠标位置
            this.lastMouseX = e.clientX;
            this.lastMouseY = e.clientY;
            
            // 重新渲染
            this.render();
        }
    }

    /**
     * 处理键盘事件
     */
    handleKeyDown(e) {
        // 检查是否在输入框中，避免干扰输入
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') {
            return;
        }

        switch (e.key.toLowerCase()) {
            case 'r':
                e.preventDefault();
                this.resetView();
                break;
            case '+':
            case '=':
                e.preventDefault();
                this.zoomIn();
                break;
            case '-':
                e.preventDefault();
                this.zoomOut();
                break;
            case 'arrowup':
                e.preventDefault();
                this.panUp();
                break;
            case 'arrowdown':
                e.preventDefault();
                this.panDown();
                break;
            case 'arrowleft':
                e.preventDefault();
                this.panLeft();
                break;
            case 'arrowright':
                e.preventDefault();
                this.panRight();
                break;
        }
    }

    /**
     * 放大视图
     */
    zoomIn() {
        const centerX = this.canvas.width / 2;
        const centerY = this.canvas.height / 2;
        const zoomFactor = 1.1;
        const newScale = this.scale * zoomFactor;
        
        if (newScale > 5) return;
        
        this.offsetX = centerX - (centerX - this.offsetX) * zoomFactor;
        this.offsetY = centerY - (centerY - this.offsetY) * zoomFactor;
        this.scale = newScale;
        
        document.getElementById('zoom-display').textContent = 
            `缩放: ${(this.scale * 100).toFixed(0)}%`;
        
        this.render();
    }

    /**
     * 缩小视图
     */
    zoomOut() {
        const centerX = this.canvas.width / 2;
        const centerY = this.canvas.height / 2;
        const zoomFactor = 0.9;
        const newScale = this.scale * zoomFactor;
        
        if (newScale < 0.1) return;
        
        this.offsetX = centerX - (centerX - this.offsetX) * zoomFactor;
        this.offsetY = centerY - (centerY - this.offsetY) * zoomFactor;
        this.scale = newScale;
        
        document.getElementById('zoom-display').textContent = 
            `缩放: ${(this.scale * 100).toFixed(0)}%`;
        
        this.render();
    }

    /**
     * 向上平移
     */
    panUp() {
        this.offsetY += 20;
        this.render();
    }

    /**
     * 向下平移
     */
    panDown() {
        this.offsetY -= 20;
        this.render();
    }

    /**
     * 向左平移
     */
    panLeft() {
        this.offsetX += 20;
        this.render();
    }

    /**
     * 向右平移
     */
    panRight() {
        this.offsetX -= 20;
        this.render();
    }

    /**
     * 重置视图到最佳状态
     */
    resetView() {
        if (!this.treeData) return;
        
        this.calculateOptimalView();
        this.render();
        this.updateStatus('视图已重置');
    }

    /**
     * 更新状态显示
     */
    updateStatus(message) {
        document.getElementById('status-text').textContent = message;
    }
}

// 页面加载完成后初始化可视化器
document.addEventListener('DOMContentLoaded', () => {
    window.visualizer = new RTreeVisualizer();
    
    // 添加示例数据按钮（用于测试）
    const loadExampleBtn = document.createElement('button');
    loadExampleBtn.textContent = '加载示例数据';
    loadExampleBtn.className = 'btn primary';
    loadExampleBtn.style.marginLeft = '10px';
    loadExampleBtn.addEventListener('click', () => {
        const exampleData = {
            "root": {
                "mbr": { "min": [0.0, 0.0], "max": [30.0, 30.0] },
                "node_type": "Leaf",
                "level": 0,
                "data_entries": [
                    { "mbr": { "min": [0.0, 0.0], "max": [10.0, 10.0] }, "data": 1 },
                    { "mbr": { "min": [5.0, 5.0], "max": [15.0, 15.0] }, "data": 2 },
                    { "mbr": { "min": [20.0, 20.0], "max": [30.0, 30.0] }, "data": 3 }
                ],
                "child_nodes": []
            },
            "config": { "max_entries": 4, "min_entries": 2 }
        };
        
        document.getElementById('json-input').value = JSON.stringify(exampleData, null, 2);
        window.visualizer.loadDataFromInput();
    });
    
    document.getElementById('load-data-btn').parentNode.appendChild(loadExampleBtn);
});
