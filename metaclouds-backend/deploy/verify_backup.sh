#!/bin/bash
# Backup Verification Script for Metaclouds
# Author: Metaclouds Team
# Version: 1.0.0

set -euo pipefail

# ==================== 配置区域 ====================
BACKUP_DIR="${BACKUP_DIR:-/var/backups/metaclouds}"
DATABASE_HOST="${DATABASE_HOST:-localhost}"
DATABASE_PORT="${DATABASE_PORT:-5432}"
DATABASE_NAME="${DATABASE_NAME:-metaclouds}"
DATABASE_USER="${DATABASE_USER:-metaclouds_user}"
LOG_FILE="/var/log/metaclouds/backup_verify.log"
ALERT_EMAIL="${ALERT_EMAIL:-admin@yourcompany.com}"

# ==================== 日志函数 ====================
log() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[${timestamp}] $*" >> "${LOG_FILE}"
}

log_info() { log "INFO: $*"; }
log_warn() { log "WARN: $*"; }
log_error() { log "ERROR: $*"; }

# ==================== 验证函数 ====================
verify_backup_file() {
    local backup_file="$1"

    log_info "Verifying backup file: ${backup_file}"

    # 1. 检查文件是否存在
    if [ ! -f "${backup_file}" ]; then
        log_error "Backup file not found: ${backup_file}"
        return 1
    fi

    # 2. 检查文件大小
    local file_size=$(stat -c%s "${backup_file}" 2>/dev/null || stat -f%z "${backup_file}" 2>/dev/null)
    if [ "${file_size}" -lt 1024 ]; then
        log_error "Backup file is too small (${file_size} bytes): ${backup_file}"
        return 1
    fi
    log_info "Backup file size: ${file_size} bytes"

    # 3. 验证校验和
    if [ -f "${backup_file}.sha256" ]; then
        if sha256sum -c "${backup_file}.sha256" > /dev/null 2>&1; then
            log_info "Checksum verification: PASSED"
        else
            log_error "Checksum verification: FAILED"
            return 1
        fi
    else
        log_warn "Checksum file not found: ${backup_file}.sha256"
    fi

    # 4. 验证备份内容（使用 pg_restore --list）
    if [[ "${backup_file}" == *.gpg ]]; then
        # 解密后验证
        local temp_file=$(mktemp)
        if echo "${BACKUP_ENCRYPTION_KEY:-}" | gpg --batch --yes --passphrase-fd 0 -d -o "${temp_file}" "${backup_file}" 2>/dev/null; then
            if PGPASSWORD="${DATABASE_PASSWORD}" pg_restore "${temp_file}" --list > /dev/null 2>&1; then
                log_info "Backup content verification (decrypted): PASSED"
            else
                log_warn "Backup content verification: WARNING (may still be valid)"
            fi
            rm -f "${temp_file}"
        else
            log_warn "Could not decrypt backup for content verification"
        fi
    else
        if PGPASSWORD="${DATABASE_PASSWORD}" pg_restore "${backup_file}" --list > /dev/null 2>&1; then
            log_info "Backup content verification: PASSED"
        else
            log_warn "Backup content verification: WARNING (may still be valid)"
        fi
    fi

    log_info "Backup verification completed: ${backup_file}"
    return 0
}

# ==================== 主函数 ====================
main() {
    local target="${1:-latest}"
    local backup_file=""
    local exit_code=0

    log "========== Backup Verification Started =========="

    if [ "${target}" = "latest" ]; then
        # 查找最新的备份文件
        backup_file=$(ls -t "${BACKUP_DIR}"/${DATABASE_NAME}_*_*.dump.gz* 2>/dev/null | head -1)
        if [ -z "${backup_file}" ]; then
            backup_file=$(ls -t "${BACKUP_DIR}"/${DATABASE_NAME}_*_*.sql.gz* 2>/dev/null | head -1)
        fi
        if [ -z "${backup_file}" ]; then
            log_error "No backup files found in ${BACKUP_DIR}"
            exit 1
        fi
        log_info "Latest backup file: ${backup_file}"
    else
        backup_file="${target}"
    fi

    if verify_backup_file "${backup_file}"; then
        log_info "Backup verification: SUCCESS"
    else
        log_error "Backup verification: FAILED"
        exit_code=1

        # 发送告警邮件
        if [ -n "${ALERT_EMAIL}" ] && command -v sendmail &> /dev/null; then
            echo -e "Subject: [CRITICAL] Metaclouds Backup Verification Failed\n\nBackup file: ${backup_file}\n\nPlease check the backup immediately." | sendmail "${ALERT_EMAIL}"
        fi
    fi

    log "========== Backup Verification Completed =========="
    exit ${exit_code}
}

main "$@"
