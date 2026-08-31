#!/bin/bash
# PostgreSQL Database Backup Script for Metaclouds
# Author: Metaclouds Team
# Version: 1.1.0

set -euo pipefail

# ==================== 依赖检查 ====================
check_dependencies() {
    local missing=()
    
    for cmd in pg_dump pg_isready pg_restore sha256sum; do
        if ! command -v "$cmd" &> /dev/null; then
            missing+=("$cmd")
        fi
    done
    
    if [ ${#missing[@]} -gt 0 ]; then
        echo "ERROR: Missing required commands: ${missing[*]}" >&2
        echo "Please install postgresql-client package" >&2
        return 1
    fi
    
    return 0
}

# 检查依赖（可选，脚本不会因此失败）
check_dependencies || true

# ==================== 兼容性别名 ====================
get_file_size() {
    local file="$1"
    if stat -c%s "$file" &> /dev/null; then
        stat -c%s "$file"
    else
        stat -f%z "$file" 2>/dev/null || echo "0"
    fi
}

# ==================== 配置区域 ====================
# 修改以下配置以适应您的环境
BACKUP_DIR="${BACKUP_DIR:-/var/backups/metaclouds}"
DATABASE_HOST="${DATABASE_HOST:-localhost}"
DATABASE_PORT="${DATABASE_PORT:-5432}"
DATABASE_NAME="${DATABASE_NAME:-metaclouds}"
DATABASE_USER="${DATABASE_USER:-metaclouds_user}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
S3_BUCKET="${S3_BUCKET:-}"
S3_PREFIX="${S3_PREFIX:-metaclouds/backups}"
LOG_FILE="/var/log/metaclouds/backup.log"

# 保留的备份数量
HOURLY_KEEP=24
DAILY_KEEP=7
WEEKLY_KEEP=4
MONTHLY_KEEP=12

# ==================== 颜色输出 ====================
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# ==================== 日志函数 ====================
log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo -e "[${timestamp}] [${level}] ${message}" | tee -a "${LOG_FILE}"
}

log_info() { log "INFO" "$*"; }
log_warn() { log "WARN" "$*"; }
log_error() { log "ERROR" "$*" >&2; }

# ==================== 备份函数 ====================
create_backup() {
    local backup_type="$1"
    local timestamp=$(date '+%Y%m%d_%H%M%S')
    local backup_file="${BACKUP_DIR}/${DATABASE_NAME}_${backup_type}_${timestamp}.dump.gz"
    local temp_file="${BACKUP_DIR}/${DATABASE_NAME}_${backup_type}_${timestamp}.dump.gz"
    local encrypted_file="${BACKUP_DIR}/${DATABASE_NAME}_${backup_type}_${timestamp}.dump.gz.gpg"

    log_info "Starting ${backup_type} backup..."

    # 创建备份目录
    mkdir -p "${BACKUP_DIR}"

    # 检查数据库连接
    if ! PGPASSWORD="${DATABASE_PASSWORD}" pg_isready -h "${DATABASE_HOST}" -p "${DATABASE_PORT}" -U "${DATABASE_USER}"; then
        log_error "Database is not ready for backup"
        return 1
    fi

    # 执行备份
    if PGPASSWORD="${DATABASE_PASSWORD}" pg_dump \
        -h "${DATABASE_HOST}" \
        -p "${DATABASE_PORT}" \
        -U "${DATABASE_USER}" \
        -d "${DATABASE_NAME}" \
        -Fc \
        -Z 6 \
        -f "${temp_file}"; then

        log_info "Backup created successfully: ${temp_file}"

        # 加密备份文件（如果设置了密码）
        if [ -n "${BACKUP_ENCRYPTION_KEY:-}" ]; then
            if command -v gpg &> /dev/null; then
                echo "${BACKUP_ENCRYPTION_KEY}" | gpg --batch --yes --passphrase-fd 0 -c -o "${encrypted_file}" "${temp_file}"
                rm -f "${temp_file}"
                backup_file="${encrypted_file}"
                log_info "Backup encrypted and saved to ${backup_file}"
            else
                log_warn "GPG not found, saving unencrypted backup"
            fi
        fi

        # 计算校验和
        sha256sum "${backup_file}" > "${backup_file}.sha256"
        log_info "Checksum saved to ${backup_file}.sha256"

        # 上传到 S3（如果配置了）
        if [ -n "${S3_BUCKET}" ]; then
            upload_to_s3 "${backup_file}"
            upload_to_s3 "${backup_file}.sha256"
        fi

        return 0
    else
        log_error "Backup failed!"
        return 1
    fi
}

upload_to_s3() {
    local file="$1"
    local s3_path="s3://${S3_BUCKET}/${S3_PREFIX}/$(basename ${file})"

    log_info "Uploading ${file} to ${s3_path}..."

    if command -v aws &> /dev/null; then
        aws s3 cp "${file}" "${s3_path}" --storage-class STANDARD_IA
        log_info "Upload to S3 completed"
    else
        log_warn "AWS CLI not found, skipping S3 upload"
    fi
}

cleanup_old_backups() {
    log_info "Cleaning up old backups (retention: ${RETENTION_DAYS} days)..."

    # 删除过期备份
    find "${BACKUP_DIR}" -name "${DATABASE_NAME}_*.sql.gz*" -type f -mtime +${RETENTION_DAYS} -delete
    find "${BACKUP_DIR}" -name "${DATABASE_NAME}_*.sha256" -type f -mtime +${RETENTION_DAYS} -delete

    # 清理 S3 上的旧备份
    if [ -n "${S3_BUCKET}" ] && command -v aws &> /dev/null; then
        local cutoff_date=$(date -d "${RETENTION_DAYS} days ago" +%Y-%m-%d)
        aws s3api list-objects \
            --bucket "${S3_BUCKET}" \
            --prefix "${S3_PREFIX}/" \
            --query "Contents[?LastModified<'${cutoff_date}'].{Key: Key}" \
            --output text | while read key; do
                [ -n "$key" ] && aws s3 rm "s3://${S3_BUCKET}/${key}"
            done
        log_info "S3 cleanup completed"
    fi

    log_info "Cleanup completed"
}

verify_backup() {
    local backup_file="$1"

    log_info "Verifying backup ${backup_file}..."

    # 验证文件存在
    if [ ! -f "${backup_file}" ]; then
        log_error "Backup file not found: ${backup_file}"
        return 1
    fi

    # 验证校验和
    if [ -f "${backup_file}.sha256" ]; then
        if sha256sum -c "${backup_file}.sha256" > /dev/null 2>&1; then
            log_info "Checksum verification passed"
        else
            log_error "Checksum verification failed!"
            return 1
        fi
    fi

    # 验证备份内容（使用 pg_restore --list）
    if PGPASSWORD="${DATABASE_PASSWORD}" pg_restore "${backup_file}" --list > /dev/null 2>&1; then
        log_info "Backup content verification passed"
    else
        log_warn "Backup content verification failed (backup may still be valid)"
    fi

    log_info "Backup size: $(du -h ${backup_file} | cut -f1)"
    return 0
}

# ==================== 恢复函数 ====================
restore_backup() {
    local backup_file="$1"

    if [ -z "${backup_file}" ]; then
        echo "Usage: $0 restore <backup_file>"
        exit 1
    fi

    log_info "Restoring from ${backup_file}..."

    # 确认操作
    read -p "This will overwrite the current database. Are you sure? (yes/no): " confirm
    if [ "${confirm}" != "yes" ]; then
        log_info "Restore cancelled"
        exit 0
    fi

    # 解密备份（如果需要）
    local temp_file="${backup_file%.gpg}"
    if [[ "${backup_file}" == *.gpg ]]; then
        echo "${BACKUP_ENCRYPTION_KEY}" | gpg --batch --yes --passphrase-fd 0 -d -o "${temp_file}" "${backup_file}"
    else
        temp_file="${backup_file}"
    fi

    # 执行恢复
    PGPASSWORD="${DATABASE_PASSWORD}" pg_restore \
        -h "${DATABASE_HOST}" \
        -p "${DATABASE_PORT}" \
        -U "${DATABASE_USER}" \
        -d "${DATABASE_NAME}" \
        --clean \
        --if-exists \
        --no-owner \
        --no-acl \
        "${temp_file}"

    # 清理临时文件
    if [[ "${backup_file}" == *.gpg ]]; then
        rm -f "${temp_file}"
    fi

    log_info "Restore completed successfully"
}

# ==================== 列出备份 ====================
list_backups() {
    log_info "Available backups in ${BACKUP_DIR}:"
    echo ""
    ls -lh "${BACKUP_DIR}"/${DATABASE_NAME}_*.sql.gz* 2>/dev/null | awk '{print $9, $5}' | while read file size; do
        echo "  $(basename ${file}) - ${size}"
    done
    echo ""

    if [ -n "${S3_BUCKET}" ] && command -v aws &> /dev/null; then
        log_info "Backups in S3 (${S3_BUCKET}/${S3_PREFIX}):"
        echo ""
        aws s3 ls "s3://${S3_BUCKET}/${S3_PREFIX}/" 2>/dev/null | while read line; do
            echo "  ${line}" | awk '{print $4, $3}'
        done
        echo ""
    fi
}

# ==================== 主函数 ====================
main() {
    local command="${1:-backup}"

    case "${command}" in
        backup)
            create_backup "full"
            cleanup_old_backups
            ;;
        hourly)
            create_backup "hourly"
            ;;
        daily)
            create_backup "daily"
            cleanup_old_backups
            ;;
        weekly)
            create_backup "weekly"
            cleanup_old_backups
            ;;
        restore)
            restore_backup "${2:-}"
            ;;
        list)
            list_backups
            ;;
        verify)
            verify_backup "${2:-}"
            ;;
        *)
            echo "Usage: $0 {backup|hourly|daily|weekly|restore|list|verify}"
            echo ""
            echo "Commands:"
            echo "  backup    - Create a full backup"
            echo "  hourly    - Create an hourly backup"
            echo "  daily     - Create a daily backup"
            echo "  weekly    - Create a weekly backup"
            echo "  restore   - Restore from backup"
            echo "  list      - List available backups"
            echo "  verify    - Verify a backup file"
            exit 1
            ;;
    esac
}

# ==================== 健康检查 ====================
health_check() {
    PGPASSWORD="${DATABASE_PASSWORD}" pg_isready \
        -h "${DATABASE_HOST}" \
        -p "${DATABASE_PORT}" \
        -U "${DATABASE_USER}" \
        -d "${DATABASE_NAME}"
}

# ==================== 入口点 ====================
main "$@"
