#!/bin/bash

# ==============================================================================
# Metaclouds 数据库迁移运行脚本
# ==============================================================================
# 封装 golang-migrate 常用操作，提供生产环境保护和便捷的命令行接口。
#
# 用法：
#   ./scripts/run_migrations.sh <command> [options]
#
# 命令：
#   up              执行所有待执行的迁移（升级到最新版本）
#   down [n]        回滚 n 个版本（默认 1）
#   goto <version>  迁移到指定版本
#   version         查看当前迁移版本
#   force <version> 强制设置版本（不执行 SQL，仅修改版本记录）
#   create <name>   创建新的迁移文件对
#   help            显示帮助信息
#
# 选项：
#   -d, --database-url <url>  数据库连接 URL（也可通过 DATABASE_URL 环境变量设置）
#   -p, --path <path>          迁移文件目录（默认 ./migrations）
#   -y, --yes                  跳过确认提示（自动化场景使用）
#
# 示例：
#   # 升级到最新版本
#   DATABASE_URL="postgres://user:pass@localhost:5432/metaclouds?sslmode=disable" ./scripts/run_migrations.sh up
#
#   # 回滚 1 个版本（生产环境需确认）
#   ./scripts/run_migrations.sh down 1
#
#   # 查看当前版本
#   ./scripts/run_migrations.sh version
#
#   # 创建新迁移
#   ./scripts/run_migrations.sh create add_new_feature
# ==============================================================================

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 脚本所在目录（用于解析相对路径）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$(dirname "$SCRIPT_DIR")"

# 默认配置
MIGRATIONS_PATH="${BACKEND_DIR}/migrations"
DATABASE_URL="${DATABASE_URL:-}"
AUTO_YES=false

# ==============================================================================
# 工具函数
# ==============================================================================

info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# 显示帮助信息
show_help() {
    cat << EOF
Metaclouds 数据库迁移运行脚本

用法：
  $(basename "$0") <command> [options]

命令：
  up              执行所有待执行的迁移（升级到最新版本）
  down [n]        回滚 n 个版本（默认 1，生产环境需确认）
  goto <version>  迁移到指定版本号（如 000002）
  version         查看当前迁移版本
  force <version> 强制设置版本（不执行 SQL，仅修改版本记录，危险操作）
  create <name>   创建新的迁移文件对（up/down）
  help            显示本帮助信息

选项：
  -d, --database-url <url>  数据库连接 URL
                             也可通过 DATABASE_URL 环境变量设置
  -p, --path <path>          迁移文件目录（默认: ./migrations）
  -y, --yes                  跳过所有确认提示（自动化/CI 场景使用）

环境变量：
  DATABASE_URL    PostgreSQL 连接字符串
                  格式: postgres://user:password@host:port/dbname?sslmode=require

示例：
  # 升级到最新版本
  DATABASE_URL="postgres://user:pass@localhost:5432/metaclouds" $0 up

  # 回滚 1 个版本
  $0 down 1

  # 查看当前版本
  $0 version

  # 创建新迁移文件
  $0 create add_user_preferences
EOF
}

# 检查 migrate 工具是否安装
check_migrate_installed() {
    if ! command -v migrate &> /dev/null; then
        error "golang-migrate 工具未安装！"
        echo ""
        echo "安装命令："
        echo "  go install -tags 'postgres' github.com/golang-migrate/migrate/v4/cmd/migrate@latest"
        echo ""
        echo "安装后请确保 \$(go env GOPATH)/bin 在 PATH 中："
        echo "  export PATH=\$PATH:\$(go env GOPATH)/bin"
        echo ""
        echo "其他安装方式请参考：migrations/README.md"
        exit 1
    fi
}

# 检查数据库连接 URL 是否设置
check_database_url() {
    if [ -z "$DATABASE_URL" ]; then
        error "数据库连接 URL 未设置！"
        echo ""
        echo "请通过以下方式之一设置："
        echo "  1. 环境变量: export DATABASE_URL=\"postgres://user:pass@host:5432/dbname?sslmode=require\""
        echo "  2. 命令行参数: $0 <command> -d \"postgres://user:pass@host:5432/dbname\""
        exit 1
    fi
}

# 检查迁移目录是否存在
check_migrations_path() {
    if [ ! -d "$MIGRATIONS_PATH" ]; then
        error "迁移目录不存在: $MIGRATIONS_PATH"
        exit 1
    fi

    # 检查是否有迁移文件
    local up_count
    up_count=$(find "$MIGRATIONS_PATH" -maxdepth 1 -name "*.up.sql" | wc -l)
    if [ "$up_count" -eq 0 ]; then
        warning "迁移目录中没有找到 .up.sql 文件"
    fi
}

# 确认提示（除非 --yes）
confirm() {
    local message="$1"
    if [ "$AUTO_YES" = true ]; then
        return 0
    fi
    echo -e "${YELLOW}$message${NC}"
    read -r -p "输入 'yes' 继续，其他任意键取消: " response
    if [ "$response" != "yes" ]; then
        info "操作已取消"
        exit 0
    fi
}

# 检测是否为生产环境（基于数据库 URL 中的主机名或数据库名）
is_production() {
    if echo "$DATABASE_URL" | grep -qiE '(prod|production|metaclouds-prod)'; then
        return 0
    fi
    return 1
}

# 执行 migrate 命令
run_migrate() {
    local cmd_args=("$@")
    info "执行: migrate -path \"$MIGRATIONS_PATH\" -database \"***\" ${cmd_args[*]}"
    migrate -path "$MIGRATIONS_PATH" -database "$DATABASE_URL" "${cmd_args[@]}"
}

# ==============================================================================
# 命令实现
# ==============================================================================

# 升级到最新版本
cmd_up() {
    info "开始执行数据库迁移（升级到最新版本）..."

    # 显示待执行的迁移
    local current_version
    current_version=$(migrate -path "$MIGRATIONS_PATH" -database "$DATABASE_URL" version 2>/dev/null || echo "0 (未初始化)")
    info "当前版本: $current_version"

    # 生产环境额外确认
    if is_production; then
        warning "检测到生产环境数据库！"
        confirm "确认要在生产环境执行迁移升级吗？"
    fi

    run_migrate up
    success "迁移升级完成！"

    # 显示新版本
    local new_version
    new_version=$(migrate -path "$MIGRATIONS_PATH" -database "$DATABASE_URL" version 2>/dev/null || echo "未知")
    info "当前版本: $new_version"
}

# 回滚版本
cmd_down() {
    local steps="${1:-1}"

    # 验证 steps 是正整数
    if ! echo "$steps" | grep -qE '^[0-9]+$'; then
        error "回滚步数必须是正整数，当前值: $steps"
        exit 1
    fi

    if [ "$steps" -eq 0 ]; then
        info "回滚步数为 0，无需操作"
        exit 0
    fi

    warning "即将回滚 $steps 个迁移版本！"

    # 显示当前版本
    local current_version
    current_version=$(migrate -path "$MIGRATIONS_PATH" -database "$DATABASE_URL" version 2>/dev/null || echo "未知")
    info "当前版本: $current_version"

    # 生产环境强制确认（即使 --yes 也需要二次确认，除非是 CI 环境）
    if is_production; then
        error "生产环境回滚操作被阻止！"
        echo ""
        echo "生产环境数据库回滚是高风险操作，可能导致数据丢失。"
        echo "如需执行，请："
        echo "  1. 先完整备份数据库"
        echo "  2. 由 DBA 手动执行回滚 SQL"
        echo "  3. 或在非生产环境验证后，联系运维团队执行"
        exit 1
    fi

    # 非生产环境也需要确认
    confirm "确认要回滚 $steps 个版本吗？此操作可能导致数据丢失！"

    run_migrate down "$steps"
    success "回滚完成！"

    local new_version
    new_version=$(migrate -path "$MIGRATIONS_PATH" -database "$DATABASE_URL" version 2>/dev/null || echo "未知")
    info "当前版本: $new_version"
}

# 迁移到指定版本
cmd_goto() {
    local version="$1"

    if [ -z "$version" ]; then
        error "请指定目标版本号，如: 000002"
        exit 1
    fi

    info "迁移到指定版本: $version"

    local current_version
    current_version=$(migrate -path "$MIGRATIONS_PATH" -database "$DATABASE_URL" version 2>/dev/null || echo "未知")
    info "当前版本: $current_version"

    if is_production; then
        warning "检测到生产环境数据库！"
        confirm "确认要在生产环境迁移到版本 $version 吗？"
    fi

    run_migrate goto "$version"
    success "已迁移到版本 $version"
}

# 查看当前版本
cmd_version() {
    info "当前数据库迁移版本："
    run_migrate version
}

# 强制设置版本
cmd_force() {
    local version="$1"

    if [ -z "$version" ]; then
        error "请指定要强制设置的版本号"
        exit 1
    fi

    warning "强制设置版本操作不会执行任何 SQL！"
    warning "仅修改 schema_migrations 表中的版本记录。"
    warning "使用前必须确保数据库实际状态与目标版本一致！"

    confirm "确认要强制设置版本为 $version 吗？请确保已手动修复数据库状态！"

    run_migrate force "$version"
    success "版本已强制设置为 $version"
    info "请验证数据库状态是否符合预期"
}

# 创建新迁移
cmd_create() {
    local name="$1"

    if [ -z "$name" ]; then
        error "请指定迁移名称，如: add_user_preferences"
        exit 1
    fi

    # 验证名称格式（小写字母、数字、下划线）
    if ! echo "$name" | grep -qE '^[a-z][a-z0-9_]*$'; then
        error "迁移名称必须以小写字母开头，仅包含小写字母、数字和下划线"
        exit 1
    fi

    info "创建新迁移文件: $name"
    migrate create -ext sql -dir "$MIGRATIONS_PATH" -seq "$name"
    success "迁移文件创建完成！"
    info "请编辑生成的 .up.sql 和 .down.sql 文件"
}

# ==============================================================================
# 主入口
# ==============================================================================

main() {
    if [ $# -eq 0 ]; then
        show_help
        exit 0
    fi

    local command="$1"
    shift

    # 解析选项
    while [ $# -gt 0 ]; do
        case "$1" in
            -d|--database-url)
                DATABASE_URL="$2"
                shift 2
                ;;
            -p|--path)
                MIGRATIONS_PATH="$2"
                shift 2
                ;;
            -y|--yes)
                AUTO_YES=true
                shift
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                # 非选项参数，作为命令参数保留
                break
                ;;
        esac
    done

    # 对于 create 命令，不需要数据库连接
    if [ "$command" != "create" ] && [ "$command" != "help" ]; then
        check_migrate_installed
        check_database_url
        check_migrations_path
    else
        check_migrate_installed
    fi

    # 执行命令
    case "$command" in
        up)
            cmd_up
            ;;
        down)
            cmd_down "${1:-1}"
            ;;
        goto)
            cmd_goto "${1:-}"
            ;;
        version)
            cmd_version
            ;;
        force)
            cmd_force "${1:-}"
            ;;
        create)
            cmd_create "${1:-}"
            ;;
        help|-h|--help)
            show_help
            ;;
        *)
            error "未知命令: $command"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

main "$@"
