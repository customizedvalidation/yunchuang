#!/usr/bin/env python3
"""凭据轮换工具。

用密码学安全随机源重新生成 .env 系列文件中的敏感值，原地替换并保留
注释、键顺序与行尾风格。输出一份轮换清单，供运维同步到真实的
数据库 / Redis / Secret Manager。

用法:
    python scripts/rotate_credentials.py --repo . --out <轮换清单路径>
    python scripts/rotate_credentials.py --repo . --out <路径> --dry-run
"""

from __future__ import annotations

import argparse
import base64
import re
import secrets
import string
import sys
from datetime import datetime, timezone
from pathlib import Path

# URL / 连接串安全字符集：排除 @ : / ? # [ ] % 等保留字符，
# 避免写进 postgres://user:pass@host 形式的连接串后被误解析。
CONNSTR_SAFE = string.ascii_letters + string.digits + "-_.~"

# 人工登录口令允许更宽的符号集以满足复杂度策略，但仍排除引号、
# 反斜杠与空格，避免 shell / YAML / .env 解析歧义。
PASSWORD_SYMBOLS = "!@#$%^&*-_=+"

# 每个文件需要轮换的键。值为生成器类型。
ROTATION_PLAN: dict[str, dict[str, str]] = {
    ".env": {
        "JWT_SECRET": "jwt",
        "DEFAULT_ADMIN_PASSWORD": "password",
        "DEFAULT_USER_PASSWORD": "password",
    },
    ".env.development": {
        "JWT_SECRET": "jwt",
        "DATABASE_PASSWORD": "connstr",
    },
    ".env.staging": {
        "JWT_SECRET": "jwt",
        "DATABASE_PASSWORD": "connstr",
    },
    ".env.production": {
        "JWT_SECRET": "jwt",
        "DATABASE_PASSWORD": "connstr",
        "REDIS_PASSWORD": "connstr",
        "DEFAULT_ADMIN_PASSWORD": "password",
        "DEFAULT_USER_PASSWORD": "password",
    },
    "docker/.env.production": {
        "JWT_SECRET": "jwt",
        "REDIS_PASSWORD": "connstr",
    },
}

# 若键在文件中缺失，是否补写（用于 staging 缺少 JWT_SECRET 这类情况）。
APPEND_IF_MISSING = {
    ".env.staging": ["JWT_SECRET"],
}


def gen_jwt_secret() -> str:
    """64 字节熵，base64url 无填充，约 86 字符。"""
    return base64.urlsafe_b64encode(secrets.token_bytes(64)).decode().rstrip("=")


def gen_connstr_password(length: int = 32) -> str:
    return "".join(secrets.choice(CONNSTR_SAFE) for _ in range(length))


def gen_login_password(length: int = 24) -> str:
    """保证同时含大写、小写、数字、符号，满足常见密码复杂度策略。"""
    if length < 8:
        raise ValueError("length must be >= 8")
    pools = [
        string.ascii_uppercase,
        string.ascii_lowercase,
        string.digits,
        PASSWORD_SYMBOLS,
    ]
    chars = [secrets.choice(pool) for pool in pools]
    everything = "".join(pools)
    chars += [secrets.choice(everything) for _ in range(length - len(pools))]
    # secrets.SystemRandom().shuffle 使用 os.urandom，避免 random 的可预测性
    secrets.SystemRandom().shuffle(chars)
    return "".join(chars)


GENERATORS = {
    "jwt": gen_jwt_secret,
    "connstr": gen_connstr_password,
    "password": gen_login_password,
}


def detect_newline(raw: bytes) -> str:
    """保持原文件行尾风格，避免整文件 diff 噪声。"""
    if b"\r\n" in raw:
        return "\r\n"
    return "\n"


def mask(value: str) -> str:
    if len(value) <= 8:
        return "*" * len(value)
    return f"{value[:3]}…{value[-2:]} (len={len(value)})"


def rotate_file(
    path: Path,
    keys: dict[str, str],
    append_missing: list[str],
    dry_run: bool,
) -> tuple[dict[str, tuple[str, str]], list[str]]:
    """返回 ({key: (旧值掩码, 新值)}, 警告列表)。"""
    if not path.exists():
        return {}, [f"文件不存在，跳过: {path}"]

    raw = path.read_bytes()
    newline = detect_newline(raw)
    text = raw.decode("utf-8")
    lines = text.split(newline)

    changes: dict[str, tuple[str, str]] = {}
    warnings: list[str] = []
    seen: set[str] = set()

    for idx, line in enumerate(lines):
        stripped = line.lstrip()
        if not stripped or stripped.startswith("#"):
            continue
        m = re.match(r"^(\s*)([A-Za-z_][A-Za-z0-9_]*)(\s*=\s*)(.*)$", line)
        if not m:
            continue
        indent, key, sep, old_value = m.groups()
        if key not in keys:
            continue
        if key in seen:
            warnings.append(f"{path.name}: 键 {key} 重复出现，仅第一处被轮换")
            continue
        seen.add(key)

        new_value = GENERATORS[keys[key]]()
        lines[idx] = f"{indent}{key}{sep}{new_value}"
        changes[key] = (mask(old_value), new_value)

    missing = [k for k in append_missing if k not in seen]
    if missing:
        # 补写缺失键，附带说明注释
        if lines and lines[-1] == "":
            lines.pop()
        lines.append("")
        lines.append("# 由 scripts/rotate_credentials.py 补写：此前缺失导致回退到默认值")
        for key in missing:
            new_value = GENERATORS[keys[key]]()
            lines.append(f"{key}={new_value}")
            changes[key] = ("(此前缺失)", new_value)
        lines.append("")

    if not dry_run and changes:
        path.write_bytes(newline.join(lines).encode("utf-8"))

    return changes, warnings


def main() -> int:
    parser = argparse.ArgumentParser(description="轮换 .env 系列文件中的敏感凭据")
    parser.add_argument("--repo", default=".", help="仓库根目录")
    parser.add_argument("--out", required=True, help="轮换清单输出路径（含明文新凭据）")
    parser.add_argument("--dry-run", action="store_true", help="只预览不写入")
    args = parser.parse_args()

    repo = Path(args.repo).resolve()
    if not repo.is_dir():
        print(f"错误: 仓库目录不存在: {repo}", file=sys.stderr)
        return 1

    all_changes: dict[str, dict[str, tuple[str, str]]] = {}
    all_warnings: list[str] = []

    for rel, keys in ROTATION_PLAN.items():
        path = repo / rel
        changes, warnings = rotate_file(
            path, keys, APPEND_IF_MISSING.get(rel, []), args.dry_run
        )
        if changes:
            all_changes[rel] = changes
        all_warnings.extend(warnings)

    stamp = datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds")
    report = [
        "# 凭据轮换清单",
        "",
        f"生成时间: {stamp}",
        f"仓库: {repo}",
        f"模式: {'DRY-RUN（未写入）' if args.dry_run else '已写入工作区文件'}",
        "",
        "> 此文件含明文新凭据。请立即同步到真实的数据库 / Redis /",
        "> Secret Manager，完成后安全删除本文件。切勿提交到版本控制。",
        "",
    ]

    total = 0
    for rel, changes in all_changes.items():
        report.append(f"## {rel}")
        report.append("")
        for key, (old_masked, new_value) in changes.items():
            total += 1
            report.append(f"- **{key}**")
            report.append(f"  - 旧值: `{old_masked}`")
            report.append(f"  - 新值: `{new_value}`")
        report.append("")

    if all_warnings:
        report.append("## 警告")
        report.append("")
        report.extend(f"- {w}" for w in all_warnings)
        report.append("")

    report.append("## 必须同步的外部系统")
    report.append("")
    report.append("轮换只改了配置文件，真实服务端口令**不会自动生效**，需人工同步：")
    report.append("")
    report.append("1. PostgreSQL: `ALTER USER metaclouds_user WITH PASSWORD '<新 DATABASE_PASSWORD>';`")
    report.append("2. Redis: 更新 `requirepass` 并重载配置")
    report.append("3. Secret Manager / K8s Secret: 更新对应条目后滚动重启工作负载")
    report.append("4. 已签发的 JWT 会因 JWT_SECRET 变更而全部失效，用户需重新登录")
    report.append("5. 用新的 DEFAULT_ADMIN_PASSWORD 验证登录后，建议立即改为个人口令")
    report.append("")

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text("\n".join(report), encoding="utf-8")

    print(f"轮换完成: {total} 项凭据，覆盖 {len(all_changes)} 个文件")
    for rel, changes in all_changes.items():
        print(f"  {rel}: {', '.join(changes)}")
    for w in all_warnings:
        print(f"  警告: {w}")
    print(f"清单已写入: {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
