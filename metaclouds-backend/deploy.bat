@echo off
REM ==============================================
REM Metaclouds Production Deployment Script (Windows)
REM ==============================================
REM Usage: deploy.bat [options]
REM Options:
REM   /h, /help        Show this help message
REM   /e:ENV           Environment: production, staging, development
REM   /b               Only build, don't start
REM   /s               Only start, don't build
REM   /d               Run in background (daemon mode)
REM   /k               Kill existing process using the port before starting
REM ==============================================

setlocal enabledelayedexpansion

REM 引入端口工具函数库
set "SCRIPT_DIR=%~dp0"
set "PORT_UTILS_SCRIPT=%SCRIPT_DIR%deploy\utils\port_utils.bat"

if exist "%PORT_UTILS_SCRIPT%" (
    call "%PORT_UTILS_SCRIPT%"
) else (
    echo [ERROR] Port utility script not found: %PORT_UTILS_SCRIPT%
    exit /b 1
)

set "APP_NAME=metaclouds-backend"
set "APP_VERSION=1.0.0"
set "DEFAULT_ENV=production"
set "BUILD_DIR=."
set "ENV_FILE=.env"
set "ENV=%DEFAULT_ENV%"
set "BUILD=true"
set "START=true"
set "DAEMON=false"
set "KILL=false"

REM Parse arguments
:parse_args
if "%1"=="" goto :end_parse
if /i "%1"=="/h" goto :show_help
if /i "%1"=="/help" goto :show_help
if /i "%1"=="/e" (
    set "ENV=%2"
    shift
    shift
    goto :parse_args
)
if /i "%1"=="/b" (
    set "START=false"
    shift
    goto :parse_args
)
if /i "%1"=="/s" (
    set "BUILD=false"
    shift
    goto :parse_args
)
if /i "%1"=="/d" (
    set "DAEMON=true"
    shift
    goto :parse_args
)
if /i "%1"=="/k" (
    set "KILL=true"
    shift
    goto :parse_args
)
echo Unknown option: %1
exit /b 1

:show_help
echo Usage: %0 [options]
echo Options:
echo   /h, /help        Show this help message
echo   /e:ENV           Environment: production, staging, development
echo   /b               Only build, don't start
echo   /s               Only start, don't build
echo   /d               Run in background (daemon mode)
echo   /k               Kill existing process using the port before starting
exit /b 0

:end_parse

REM Functions
:info
echo [INFO] %~1
goto :EOF

:success
echo [SUCCESS] %~1
goto :EOF

:warning
echo [WARNING] %~1
goto :EOF

:error
echo [ERROR] %~1
exit /b 1

REM Main deployment
call :info "=============================================="
call :info "Metaclouds Production Deployment Script"
call :info "Version: %APP_VERSION%"
call :info "Environment: %ENV%"
call :info "=============================================="

REM Step 1: Validate environment
call :info "Validating environment..."
if not exist ".env.%ENV%" (
    call :error "Environment file .env.%ENV% not found!"
)

REM Step 2: Copy environment configuration
call :info "Copying environment configuration..."
copy ".env.%ENV%" "%ENV_FILE%" /Y >nul
if errorlevel 1 (
    call :error "Failed to copy environment configuration"
)
call :success "Environment configuration copied: %ENV_FILE%"

REM Step 3: Build application
if "%BUILD%"=="true" (
    call :info "Building application..."
    
    REM Check if go is available
    go version >nul 2>&1
    if errorlevel 1 (
        call :error "Go is not installed. Please install Go first."
    )
    
    REM Build
    go build -o "%APP_NAME%.exe" .
    if errorlevel 1 (
        call :error "Build failed"
    )
    call :success "Application built successfully: %APP_NAME%.exe"
)

REM Step 4: Start application
if "%START%"=="true" (
    call :info "Starting application..."
    
    REM Check if application exists
    if not exist "%APP_NAME%.exe" (
        call :error "Application binary not found: %APP_NAME%.exe"
    )
    
    REM Get port from env file
    call :port_utils_get_port_from_env "%ENV_FILE%" SERVER_PORT
    call :info "Server port configured: %SERVER_PORT%"
    
    REM 使用工具库确保端口可用
    call :port_utils_ensure_port_available "%SERVER_PORT%" "%KILL%"
    if errorlevel 1 (
        exit /b 1
    )
    if errorlevel 2 (
        exit /b 0
    )
    
    REM 重新读取端口配置（可能已被用户更改）
    call :port_utils_get_port_from_env "%ENV_FILE%" SERVER_PORT
    
    REM Clean up old PID file if exists
    if exist "metaclouds.pid" (
        del /Q "metaclouds.pid"
    )
    
    if "%DAEMON%"=="true" (
        REM Run in background using start command
        call :info "Starting application in background..."
        start /B "" "%APP_NAME%.exe"
        for /f "tokens=2" %%i in ('tasklist /FI "IMAGENAME eq %APP_NAME%.exe" /FO LIST ^| findstr /r "PID:"') do (
            echo %%i > metaclouds.pid
        )
        call :success "Application started in background"
        if exist "metaclouds.pid" (
            set /p PID=<metaclouds.pid
            call :success "PID: !PID!"
        )
        call :info "Access at: http://localhost:%SERVER_PORT%"
    ) else (
        REM Run in foreground
        call :info "Starting application in foreground (Ctrl+C to stop)..."
        call :info "Access at: http://localhost:%SERVER_PORT%"
        "%APP_NAME%.exe"
    )
)

call :info "=============================================="
call :info "Deployment completed"
call :info "=============================================="

endlocal
