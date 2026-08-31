@echo off
REM ==============================================
REM 示例脚本 - 如何使用端口工具函数库 (Windows)
REM ==============================================

setlocal enabledelayedexpansion

echo === 端口工具函数库使用示例 ===
echo.

REM 1. 引入工具库
echo [步骤 1] 引入工具库...
set "SCRIPT_DIR=%~dp0"
set "PORT_UTILS_SCRIPT=%SCRIPT_DIR%port_utils.bat"

if exist "%PORT_UTILS_SCRIPT%" (
    call "%PORT_UTILS_SCRIPT%"
    echo ^<工具库引入成功^>
) else (
    echo ^<工具库未找到: %PORT_UTILS_SCRIPT%^>
    exit /b 1
)
echo.

REM 2. 测试端口检测
echo [步骤 2] 检测常用端口...
for %%p in (22 80 443 8000 8080 3306) do (
    call :port_utils_is_port_in_use %%p
    if not errorlevel 1 (
        echo ^<端口 %%p 已被占用^>
    ) else (
        echo ^<端口 %%p 可用^>
    )
)
echo.

REM 3. 获取示例端口信息
echo [步骤 3] 尝试获取端口 8000 的进程信息...
call :port_utils_is_port_in_use 8000
if not errorlevel 1 (
    echo 发现端口 8000 已被占用，获取进程信息...
    call :port_utils_get_process_info 8000
)
echo.

REM 4. 查找可用端口
echo [步骤 4] 查找可用端口...
call :port_utils_find_available_port FREE_PORT
echo 找到可用端口: %FREE_PORT%
echo.

REM 5. 从示例 .env 文件读取端口
echo [步骤 5] 读取端口配置...
set "TEMP_ENV=%TEMP%\example_env_%RANDOM%"
echo SERVER_PORT=8000 > "%TEMP_ENV%"
call :port_utils_get_port_from_env "%TEMP_ENV%" ENV_PORT
echo 从 %TEMP_ENV% 读取的端口: %ENV_PORT%
del /Q "%TEMP_ENV%"
echo.

echo === 示例结束 ===
echo 使用提示：
echo   - 在你的脚本中 call port_utils.bat
echo   - 然后调用如 :port_utils_ensure_port_available 等函数

endlocal
