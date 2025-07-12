#!/bin/bash

echo "验证重构步骤1和步骤2..."

echo "1. 检查 algorithms 目录是否存在:"
ls -la src/algorithms/

echo "2. 检查 search.rs 文件内容:"
head -10 src/algorithms/search.rs

echo "3. 检查原 algorithms.rs 中是否还有搜索方法:"
if grep -q "pub fn search" src/algorithms.rs; then
    echo "❌ 原文件中仍有搜索方法"
else
    echo "✅ 原文件中搜索方法已移除"
fi

if grep -q "fn search_recursive" src/algorithms.rs; then
    echo "❌ 原文件中仍有递归搜索方法"
else
    echo "✅ 原文件中递归搜索方法已移除"
fi

echo "4. 检查新 search.rs 中是否有搜索方法:"
if grep -q "pub fn search" src/algorithms/search.rs; then
    echo "✅ 新文件中包含搜索方法"
else
    echo "❌ 新文件中缺少搜索方法"
fi

if grep -q "fn search_recursive" src/algorithms/search.rs; then
    echo "✅ 新文件中包含递归搜索方法"
else
    echo "❌ 新文件中缺少递归搜索方法"
fi

echo "重构验证完成!"
