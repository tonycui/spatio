#!/bin/bash

echo "=== R-tree 可视化系统第一阶段测试 ==="
echo

# 检查Rust项目编译
echo "1. 检查Rust项目编译状态..."
if cargo check; then
    echo "✓ Rust项目编译成功"
else
    echo "✗ Rust项目编译失败"
    exit 1
fi
echo

# 运行测试
echo "2. 运行R-tree核心功能测试..."
if cargo test --lib; then
    echo "✓ 核心功能测试通过"
else
    echo "✗ 核心功能测试失败"
    exit 1
fi
echo

# 测试JSON导出功能
echo "3. 测试JSON导出功能..."
if cargo test test_json_export; then
    echo "✓ JSON导出功能测试通过"
else
    echo "✗ JSON导出功能测试失败"
    exit 1
fi
echo

# 生成示例数据
echo "4. 生成可视化示例数据..."
if cargo run --example generate_json; then
    echo "✓ 示例数据生成成功"
else
    echo "✗ 示例数据生成失败"
    exit 1
fi
echo

# 检查生成的文件
echo "5. 检查生成的JSON文件..."
files=("simple_tree.json" "complex_tree.json" "clustered_data.json")
for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        size=$(wc -c < "$file")
        echo "✓ $file 存在 (${size} 字节)"
    else
        echo "✗ $file 不存在"
        exit 1
    fi
done
echo

# 验证JSON格式
echo "6. 验证JSON格式..."
for file in "${files[@]}"; do
    if python3 -m json.tool "$file" > /dev/null 2>&1; then
        echo "✓ $file JSON格式正确"
    else
        echo "✗ $file JSON格式错误"
        exit 1
    fi
done
echo

# 检查前端文件
echo "7. 检查前端文件..."
frontend_files=("visualization/web/index.html" "visualization/web/style.css" "visualization/web/rtree-visualizer.js")
for file in "${frontend_files[@]}"; do
    if [ -f "$file" ]; then
        echo "✓ $file 存在"
    else
        echo "✗ $file 不存在"
        exit 1
    fi
done
echo

echo "=== 第一阶段系统测试完成 ==="
echo "所有核心功能均正常工作！"
echo
echo "下一步操作指南："
echo "1. 打开 visualization/web/index.html 在浏览器中"
echo "2. 复制任意 *.json 文件的内容到输入框"
echo "3. 点击'加载数据'按钮查看可视化效果"
echo "4. 使用可视化选项控制显示内容"
echo "5. 鼠标滚轮可以缩放，移动鼠标查看坐标"
echo
