# 端口工具函数库使用文档

## 概述

本库提供了跨平台的端口管理功能，包括端口检测、进程终止、交互式处理等，可被其他脚本轻松复用。

## 文件结构

```
deploy/utils/
├── port_utils.sh      # Linux/macOS 工具库
├── port_utils.bat     # Windows 工具库
└── README.md          # 本文档
```

## Linux/macOS 工具库 (port_utils.sh)

### 引入方式

在你的脚本中添加：

```bash
# 方式一：相对路径（推荐）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PORT_UTILS_SCRIPT="$SCRIPT_DIR/deploy/utils/port_utils.sh"

if [[ -f "$PORT_UTILS_SCRIPT" ]]; then
    source "$PORT_UTILS_SCRIPT"
else
    echo "[ERROR] Port utility script not found"
    exit 1
fi
```

### 可用函数

#### 1. 从 .env 文件读取端口

```bash
# 获取默认 .env 文件的端口
port=$(port_utils_get_port_from_env)

# 指定路径
port=$(port_utils_get_port_from_env "/path/to/.env")
```

#### 2. 检测端口是否被占用

```bash
# 返回 0 如果端口被占用，1 表示空闲
if port_utils_is_port_in_use 8000; then
    echo "Port is in use!"
fi
```

#### 3. 获取占用端口的进程 PID

```bash
pid=$(port_utils_get_pid_by_port 8000)
if [[ -n "$pid" ]]; then
    echo "PID: $pid"
fi
```

#### 4. 获取进程详细信息

```bash
port_utils_get_process_info 8000
```

#### 5. 终止指定 PID 的进程

```bash
if port_utils_kill_pid 12345; then
    echo "Process killed"
fi
```

#### 6. 终止占用端口的进程

```bash
# 正常模式（显示提示）
port_utils_kill_process_by_port 8000

# 静默模式
port_utils_kill_process_by_port 8000 "true"
```

#### 7. 交互式处理端口冲突

```bash
# 交互式询问用户
port_utils_handle_port_conflict_interactive 8000

# 直接杀死进程
port_utils_handle_port_conflict_interactive 8000 "true"
```

#### 8. 更新 .env 文件中的端口

```bash
port_utils_update_env_port 9000 "/path/to/.env"
```

#### 9. 完整的端口确保可用流程（推荐）

```bash
# 完整流程，自动处理冲突
if port_utils_ensure_port_available 8000; then
    echo "Port is ready!"
else
    echo "Failed to get port"
    exit 1
fi

# 自动杀死占用进程
port_utils_ensure_port_available 8000 "true"
```

#### 10. 查找可用端口

```bash
# 查找默认范围（8000-9000）
free_port=$(port_utils_find_available_port)

# 自定义范围
free_port=$(port_utils_find_available_port 8080 8888)
```

### 使用示例

```bash
#!/bin/bash

# 引入工具库
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/deploy/utils/port_utils.sh"

# 场景1：简单的端口检查
if ! port_utils_is_port_in_use 8000; then
    echo "Port is available"
fi

# 场景2：完整的部署流程
SERVER_PORT=$(port_utils_get_port_from_env)
port_utils_ensure_port_available "$SERVER_PORT" "false"

# 场景3：查找随机可用端口
RANDOM_PORT=$(port_utils_find_available_port)
echo "Using port: $RANDOM_PORT"
```

## Windows 工具库 (port_utils.bat)

### 引入方式

在你的 .bat 脚本中添加：

```batch
REM 方式一：相对路径（推荐）
set "SCRIPT_DIR=%~dp0"
set "PORT_UTILS_SCRIPT=%SCRIPT_DIR%deploy\utils\port_utils.bat"

if exist "%PORT_UTILS_SCRIPT%" (
    call "%PORT_UTILS_SCRIPT%"
) else (
    echo [ERROR] Port utility script not found
    exit /b 1
)
```

### 可用函数

#### 1. 从 .env 文件读取端口

```batch
call :port_utils_get_port_from_env ".env" SERVER_PORT
echo Port is: %SERVER_PORT%
```

#### 2. 检测端口是否被占用

```batch
call :port_utils_is_port_in_use 8000
if not errorlevel 1 (
    echo Port is in use!
)
```

#### 3. 获取占用端口的进程 PID

```batch
call :port_utils_get_pid_by_port 8000 PID
if not "%PID%"=="" (
    echo PID: %PID%
)
```

#### 4. 终止指定 PID 的进程

```batch
call :port_utils_kill_pid 12345
if not errorlevel 1 (
    echo Process killed
)
```

#### 5. 终止占用端口的进程

```batch
REM 正常模式（显示提示）
call :port_utils_kill_process_by_port 8000

REM 静默模式
call :port_utils_kill_process_by_port 8000 "true"
```

#### 6. 交互式处理端口冲突

```batch
call :port_utils_handle_port_conflict_interactive 8000
if errorlevel 2 (
    echo User cancelled
    exit /b 0
)
```

#### 7. 更新 .env 文件中的端口

```batch
call :port_utils_update_env_port 9000 ".env"
```

#### 8. 完整的端口确保可用流程（推荐）

```batch
call :port_utils_ensure_port_available 8000
if errorlevel 1 (
    echo Failed!
    exit /b 1
)
if errorlevel 2 (
    echo Cancelled!
    exit /b 0
)

REM 自动杀死占用进程
call :port_utils_ensure_port_available 8000 "true"
```

#### 9. 查找可用端口

```batch
REM 查找默认范围（8000-9000）
call :port_utils_find_available_port RANDOM_PORT
echo Using port: %RANDOM_PORT%

REM 自定义范围
call :port_utils_find_available_port 8080 8888 RANDOM_PORT
```

### 使用示例

```batch
@echo off
setlocal enabledelayedexpansion

REM 引入工具库
set "SCRIPT_DIR=%~dp0"
call "%SCRIPT_DIR%deploy\utils\port_utils.bat"

REM 场景1：简单的端口检查
call :port_utils_is_port_in_use 8000
if errorlevel 1 (
    echo Port is available
)

REM 场景2：完整的部署流程
call :port_utils_get_port_from_env ".env" SERVER_PORT
call :port_utils_ensure_port_available %SERVER_PORT%
if errorlevel 1 (
    exit /b 1
)

REM 场景3：查找随机可用端口
call :port_utils_find_available_port RANDOM_PORT
echo Using port: %RANDOM_PORT%

endlocal
```

## 在 deploy.sh / deploy.bat 中使用

我们已经更新了部署脚本，它们现在会自动使用工具库。你无需做任何额外操作。

### 部署脚本参数

```bash
# Linux/macOS
./deploy.sh -k          # 自动杀死占用端口的进程
./deploy.sh             # 交互式处理端口冲突

# Windows
deploy.bat /k
deploy.bat
```

## 错误码说明

| 错误码 | 含义 |
|--------|------|
| 0      | 成功 |
| 1      | 失败 |
| 2      | 用户取消（仅适用于交互式函数） |

## 常见问题

### Q: 如何调试工具库函数？

A: 可以直接在命令行测试单个函数：

```bash
# Linux/macOS
source deploy/utils/port_utils.sh
port_utils_is_port_in_use 8000
```

### Q: 可以在一个脚本中多次引入工具库吗？

A: 可以，不会有问题。但最好只引入一次以避免重复。

### Q: 工具库支持自定义颜色输出吗？

A: 目前不支持，但你可以在引入后修改颜色变量。

## 更新日志

### v1.0.0 (2026-05-25)

- 初始版本
- 支持 Linux/macOS 和 Windows
- 提供完整的端口管理功能
