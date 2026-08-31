@echo off
REM ==============================================
REM 端口工具函数库 (Windows)
REM 用于处理端口占用、进程管理等通用功能
REM ==============================================

goto :EOF

REM ==============================================
REM 从 .env 文件读取端口配置
REM 用法: call :port_utils_get_port_from_env [env_file] [output_var
REM ==============================================
:port_utils_get_port_from_env
setlocal
set "env_file=%~1
if "%env_file%"=="" set "env_file=.env"
set "port=8000"
if exist "%env_file%" (
    for /f "tokens=1,2 delims==" %%a in ('type "%env_file%" ^| findstr /r "^SERVER_PORT="') do (
        set "port=%%b"
    )
)
endlocal & set "%~2=%port%"
goto :EOF

REM ==============================================
REM 检测端口是否被占用
REM 用法: call :port_utils_is_port_in_use port
REM 返回: 0=被占用, 1=空闲
REM ==============================================
:port_utils_is_port_in_use
setlocal
set "port=%~1"
netstat -ano | findstr /r ":!port! " | findstr /i "LISTENING" >nul
if errorlevel 1 (
    endlocal & exit /b 1
) else (
    endlocal & exit /b 0
)
goto :EOF

REM ==============================================
REM 获取占用端口的 PID
REM 用法: call :port_utils_get_pid_by_port port output_var
REM ==============================================
:port_utils_get_pid_by_port
setlocal
set "port=%~1"
set "pid="
for /f "tokens=5" %%a in ('netstat -ano ^| findstr /r ":!port! " ^| findstr /i "LISTENING"') do (
    set "pid=%%a"
)
endlocal & set "%~2=%pid%"
goto :EOF

REM ==============================================
REM 强制终止指定 PID 的进程
REM 用法: call :port_utils_kill_pid pid
REM 返回: 0=成功, 1=失败
REM ==============================================
:port_utils_kill_pid
setlocal
set "pid=%~1"
if "%pid%"=="" exit /b 1
taskkill /F /PID %pid% >nul 2>&1
if errorlevel 1 (
    endlocal & exit /b 1
) else (
    endlocal & exit /b 0
)
goto :EOF

REM ==============================================
REM 终止占用端口的进程
REM 用法: call :port_utils_kill_process_by_port port [silent]
REM 返回: 0=成功, 1=失败
REM ==============================================
:port_utils_kill_process_by_port
setlocal enabledelayedexpansion
set "port=%~1"
set "silent=%~2"
if "%silent%"=="" set "silent=false"

call :port_utils_get_pid_by_port "%port%" pid
if "!pid!"=="" (
    if /i not "!silent!"=="false" (
        echo [WARNING] No process found using port %port%
    )
    endlocal & exit /b 1
)

if /i not "!silent!"=="false" (
    echo [WARNING] Killing process !pid! using port %port%
)

call :port_utils_kill_pid "!pid!"
if errorlevel 1 (
    if /i not "!silent!"=="false" (
        echo [ERROR] Failed to kill process !pid!
    )
    endlocal & exit /b 1
)

timeout /t 2 /nobreak >nul
call :port_utils_is_port_in_use "%port%"
if not errorlevel 1 (
    if /i not "!silent!"=="false" (
        echo [ERROR] Failed to release port %port%. Please kill the process manually.
    )
    endlocal & exit /b 1
)

if /i not "!silent!"=="false" (
    echo [SUCCESS] Port %port% released successfully
)
endlocal & exit /b 0
goto :EOF

REM ==============================================
REM 更新 .env 文件中的端口配置
REM 用法: call :port_utils_update_env_port new_port [env_file]
REM ==============================================
:port_utils_update_env_port
setlocal enabledelayedexpansion
set "new_port=%~1"
set "env_file=%~2"
if "%env_file%"=="" set "env_file=.env"

if not exist "%env_file%" (
    echo [ERROR] Environment file %env_file% not found
    endlocal & exit /b 1
)

(
    for /f "tokens=*" %%a in ('type "%env_file%"') do (
        set "line=%%a"
        echo !line! | findstr /r "^SERVER_PORT=" >nul
        if errorlevel 1 (
            echo !line!
        ) else (
            echo SERVER_PORT=!new_port!
        )
    )
) > "%env_file%.tmp"
move /Y "%env_file%.tmp" "%env_file%" >nul
endlocal & exit /b 0
goto :EOF

REM ==============================================
REM 交互式处理端口占用
REM 用法: call :port_utils_handle_port_conflict_interactive port [kill_on_prompt]
REM 返回: 0=成功, 1=失败, 2=用户取消
REM ==============================================
:port_utils_handle_port_conflict_interactive
setlocal enabledelayedexpansion
set "port=%~1"
set "kill_on_prompt=%~2
if "%kill_on_prompt%"=="" set "kill_on_prompt=false"

call :port_utils_is_port_in_use "%port%"
if errorlevel 1 (
    endlocal & exit /b 0
)

echo.
echo [WARNING] Port %port% is already in use!

if /i "%kill_on_prompt%"=="true" (
    call :port_utils_kill_process_by_port "%port%"
    if errorlevel 1 (
        endlocal & exit /b 1
    ) else (
        endlocal & exit /b 0
    )
)

echo.
echo Options:
echo 1) Kill the existing process and start anyway
echo 2) Change SERVER_PORT in .env file
echo 3) Cancel deployment
echo.
set /p choice="Please select an option [1-3]: "

if "!choice!"=="1" (
    call :port_utils_kill_process_by_port "%port%"
    if errorlevel 1 (
        endlocal & exit /b 1
    ) else (
        endlocal & exit /b 0
    )
) else if "!choice!"=="2" (
    set /p new_port="Enter new port number: "
    if "!new_port!"=="" (
        echo [ERROR] Invalid port number
        endlocal & exit /b 1
    )
    call :port_utils_update_env_port "!new_port!"
    if errorlevel 1 (
        endlocal & exit /b 1
    )
    set "SERVER_PORT=!new_port!"
    echo [SUCCESS] Updated .env file with port !new_port!
    endlocal & exit /b 0
) else if "!choice!"=="3" (
    echo [INFO] Deployment cancelled by user
    endlocal & exit /b 2
) else (
    echo [ERROR] Invalid option selected
    endlocal & exit /b 1
)
goto :EOF

REM ==============================================
REM 完整的端口验证和处理流程
REM 用法: call :port_utils_ensure_port_available port [auto_kill]
REM 返回: 0=成功, 1=失败, 2=用户取消
REM ==============================================
:port_utils_ensure_port_available
setlocal
set "port=%~1"
set "auto_kill=%~2"
if "%auto_kill%"=="" set "auto_kill=false"

echo [INFO] Checking port %port%...

call :port_utils_is_port_in_use "%port%"
if errorlevel 1 (
    echo [SUCCESS] Port %port% is available
    endlocal & exit /b 0
)

if /i "%auto_kill%"=="true" (
    call :port_utils_kill_process_by_port "%port%"
    if errorlevel 1 (
        endlocal & exit /b 1
    ) else (
        endlocal & exit /b 0
    )
)

call :port_utils_handle_port_conflict_interactive "%port%"
set "result=%errorlevel%"
endlocal & exit /b %result%
goto :EOF

REM ==============================================
REM 查找可用的端口
REM 用法: call :port_utils_find_available_port [start_port] [end_port] output_var
REM ==============================================
:port_utils_find_available_port
setlocal enabledelayedexpansion
set "start_port=%~1"
set "end_port=%~2"
set "output_var=%~3"
if "%start_port%"=="" set "start_port=8000"
if "%end_port%"=="" set "end_port=9000"

for /l %%p in (%start_port%,1,%end_port%) do (
    call :port_utils_is_port_in_use "%%p"
    if errorlevel 1 (
        endlocal & set "%output_var%=%%p"
        exit /b 0
    )
)

echo [ERROR] No available port found in range %start_port%-%end_port%
endlocal & exit /b 1
goto :EOF

REM ==============================================
REM 获取占用端口的进程信息
REM 用法: call :port_utils_get_process_info port
REM ==============================================
:port_utils_get_process_info
setlocal
set "port=%~1"
call :port_utils_get_pid_by_port "%port%" pid
if not "%pid%"=="" (
    tasklist /FI "PID eq %pid%" /FO LIST
)
endlocal
goto :EOF
