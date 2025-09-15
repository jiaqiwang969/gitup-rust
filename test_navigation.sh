#!/bin/bash

echo "GitUp TUI 导航测试"
echo "=================="
echo ""
echo "测试导航链: Commits → Status → Diff → Status → Commits"
echo ""
echo "操作步骤："
echo "1. 启动后在Commits标签"
echo "2. 按 j/k 选择一个commit"
echo "3. 按 Enter - 应该跳转到Status标签显示该commit的文件"
echo "4. 在Status标签按 Enter - 应该跳转到Diff标签显示文件diff"
echo "5. 在Diff标签按 Esc - 应该返回到Status标签"
echo "6. 在Status标签按 Esc - 应该返回到Commits标签"
echo ""
echo "其他快捷键："
echo "- 在Commits按 o: 查看commit完整diff (不切换标签)"
echo "- 在Commits按 O: 查看commit完整diff (切换到Diff标签)"
echo ""
echo "按Enter启动TUI..."
read

./target/release/gitup-tui .