# GitUp Rust TUI - Vim操作指南

## 启动TUI
```bash
./target/release/gitup tui [path]
```

## Vim模式

### Normal模式（默认）
主要的导航和操作模式

### Insert模式
按 `i` 进入，用于正常的键盘操作

### Visual模式
按 `v` 进入，用于批量选择和操作

### Command模式
按 `:` 进入，执行命令

### Search模式
按 `/` 进入，搜索内容

## 导航操作

### 基础移动
- `h` / `←` - 切换到左边的标签页
- `l` / `→` - 切换到右边的标签页
- `j` / `↓` - 向下移动（列表中）或向下滚动（diff中）
- `k` / `↑` - 向上移动（列表中）或向上滚动（diff中）

### 快速导航
- `gg` - 跳转到列表顶部
- `G` - 跳转到列表底部
- `M` - 跳转到列表中间
- `H` - 切换到第一个标签页（Commits）
- `L` - 切换到最后一个标签页（Diff）
- `5G` - 跳转到第5行（数字+G）
- `5j` - 向下移动5行（数字+j）
- `5k` - 向上移动5行（数字+k）

### 标签页切换
- `1` - 切换到Commits标签
- `2` - 切换到Branches标签
- `3` - 切换到Status标签
- `4` - 切换到Diff标签

### 页面滚动（在Diff标签中）
- `Ctrl+d` - 向下滚动半页
- `Ctrl+u` - 向上滚动半页
- `Ctrl+f` - 向下滚动一页
- `Ctrl+b` - 向上滚动一页

## Git操作

### 在Commits标签页
- `Enter` - 查看选中commit的文件列表（切换到Status标签）
- `o` - 查看选中commit的完整diff（停留在当前标签）
- `O` - 查看选中commit的完整diff（切换到Diff标签）

### 在Branches标签页
- `c` 或 `Enter` - checkout选中的分支

### 在Status标签页
- `s` - stage选中的文件（仅在工作目录模式）
- `u` - unstage选中的文件（仅在工作目录模式）
- `a` - stage所有文件（仅在工作目录模式）
- `A` - unstage所有文件（仅在工作目录模式）
- `Enter` - 查看文件的diff并切换到Diff标签
- `Esc` - 返回到Commits标签（如果在查看commit文件）
- `v` - 进入Visual模式批量选择
  - 在Visual模式下按 `s` 批量stage
- `x` - 放弃文件更改（暂未实现）

### 在Diff标签页
- `Esc` - 返回到之前的标签（通常是Status）

## 导航链

新的导航流程形成了一个清晰的链条：

```
Commits → Status (commit文件) → Diff (特定文件)
   ↑            ↑                      ↓
   └────────────┴──────────────────────┘
         (Esc键返回)
```

1. **Commits标签**: 按Enter查看该commit的文件变更列表
2. **Status标签**: 显示commit的文件或工作目录，按Enter查看具体diff
3. **Diff标签**: 显示文件的具体变更，按Esc返回
4. **智能返回**: Esc键会智能返回到上一个标签

## 命令模式（:）

输入 `:` 进入命令模式，支持以下命令：

- `:q` 或 `:quit` - 退出
- `:w <message>` 或 `:write <message>` - 提交staged的更改
- `:wq` - 提交并退出
- `:e` 或 `:edit` - 刷新
- `:branch <name>` - 创建新分支
- `:checkout <branch>` 或 `:co <branch>` - 切换分支

## 搜索模式（/）

输入 `/` 进入搜索模式：
- 在Commits标签：搜索commit消息或作者
- 在Branches标签：搜索分支名
- 在Status标签：搜索文件路径
- 按 `Enter` 执行搜索
- 按 `Esc` 取消搜索

## 其他操作

- `r` - 刷新仓库状态
- `q` - 退出（Normal模式下）
- `Esc` - 多功能键：
  - 在Diff标签页：返回到Commits标签
  - 在其他标签页：清除计数/清除消息
  - 在其他模式下：返回Normal模式

## 改进说明

### Enter键功能改进
- **Commits标签**: 按Enter会加载diff并自动切换到Diff标签查看
- **Status标签**: 按Enter会加载选中文件的diff并切换到Diff标签（新功能）
- **Branches标签**: 按Enter checkout选中的分支
- **导航时自动加载**: 在Commits或Diff标签中使用j/k导航时会自动加载对应的diff

### Esc键智能返回
- **在Diff标签**: 按Esc返回到之前的标签（通常是Commits或Status）
- **在其他标签**: 清除计数和消息
- **在其他模式**: 返回Normal模式

### 新增快捷键
- `o` - 打开/查看（不切换标签）
- `O` - 打开并切换到新标签
- `H` - 快速切换到第一个标签
- `L` - 快速切换到最后一个标签
- `M` - 跳转到列表中间
- `a` - Stage所有文件
- `A` - Unstage所有文件

## 示例工作流

### 1. 快速查看commit diff
```
# 在Commits标签
1
# 选择commit
jj
# 查看diff（自动切换到Diff标签）
Enter
# 按Esc快速返回Commits标签
Esc
# 继续浏览其他commit
j
Enter
# 再次按Esc返回
Esc
```

### 2. 批量管理文件
```
# 切换到Status标签
3
# Stage所有文件
a
# 或者unstage所有
A
```

### 4. 查看文件更改
```
# 切换到Status标签
3
# 选择修改的文件
j
# 查看文件diff（自动切换到Diff标签）
Enter
# 按Esc返回Status标签
Esc
# Stage该文件
s
```

## 状态栏说明

状态栏显示：
- 左侧：当前Vim模式（NORMAL/INSERT/VISUAL/COMMAND/SEARCH）和计数
- 右侧：
  - Command模式：显示输入的命令
  - Search模式：显示搜索内容
  - 其他模式：显示上下文相关的帮助信息

## 特点

1. **完整的Vim模式支持** - Normal、Insert、Visual、Command、Search
2. **智能Enter键** - 根据上下文执行最合适的操作
3. **自动加载diff** - 导航时自动更新diff内容
4. **批量操作** - 支持批量stage/unstage
5. **计数前缀** - 支持 `5j`、`10G` 等操作
6. **命令行操作** - 通过 `:` 执行Git命令
7. **增量搜索** - 通过 `/` 搜索内容
8. **模式指示器** - 清晰显示当前模式和操作状态