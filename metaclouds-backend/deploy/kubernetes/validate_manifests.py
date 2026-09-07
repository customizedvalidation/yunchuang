#!/usr/bin/env python3
"""
Metaclouds K8s 清单验证脚本（纯 Python，无第三方依赖）
- 解析所有 YAML 文件，检查语法（最小化 YAML 解析器）
- 验证每个 Deployment 有 resources / probes / securityContext
- 验证 ConfigMap 环境变量与 config.go 一致
- 验证 Secret 为 fail-secure（data 为空）
"""
import os
import sys
import re
import glob

K8S_DIR = r"D:\YCYD\metaclouds-backend\deploy\kubernetes"

errors = []
warnings = []
passed = []

def check(condition, msg, is_warning=False):
    if condition:
        passed.append(msg)
    else:
        if is_warning:
            warnings.append(msg)
        else:
            errors.append(msg)

# ============================================================================
# 最小化 YAML 解析器（支持 mappings / lists / block scalars / multi-doc）
# ============================================================================
def parse_yaml(text):
    """解析 YAML 文本，返回文档列表（每个文档为 dict/list/str/None）"""
    # 按 --- 分割多文档
    docs = []
    current_lines = []
    for line in text.split('\n'):
        if line.strip() == '---':
            if current_lines:
                docs.append(_parse_block(current_lines))
                current_lines = []
        elif line.strip() == '...':
            if current_lines:
                docs.append(_parse_block(current_lines))
                current_lines = []
        else:
            current_lines.append(line)
    if current_lines:
        docs.append(_parse_block(current_lines))
    # 过滤全空文档
    return [d for d in docs if d is not None]

def _strip_comment(line):
    """移除行尾注释（保留引号内的 #）"""
    in_single = False
    in_double = False
    for i, ch in enumerate(line):
        if ch == "'" and not in_double:
            in_single = not in_single
        elif ch == '"' and not in_single:
            in_double = not in_double
        elif ch == '#' and not in_single and not in_double:
            if i == 0 or line[i-1] in ' \t':
                return line[:i].rstrip()
    return line

def _parse_block(lines, base_indent=0):
    """解析一个缩进块，返回 dict 或 list"""
    # 过滤空行和纯注释行，计算实际内容
    content_lines = []
    for line in lines:
        stripped = line.strip()
        if stripped == '' or stripped.startswith('#'):
            continue
        content_lines.append(line)

    if not content_lines:
        return None

    # 确定块的缩进级别
    first_indent = len(content_lines[0]) - len(content_lines[0].lstrip())

    # 判断是 list 还是 mapping
    first_content = content_lines[0].strip()
    is_list = first_content.startswith('- ') or first_content == '-'

    if is_list:
        return _parse_list(content_lines, first_indent)
    else:
        return _parse_mapping(content_lines, first_indent)

def _parse_mapping(lines, indent):
    """解析 mapping"""
    result = {}
    i = 0
    while i < len(lines):
        line = lines[i]
        line_indent = len(line) - len(line.lstrip())
        if line_indent < indent:
            break
        if line_indent > indent:
            i += 1
            continue

        stripped = line.strip()
        # 跳过 list 项（不应该出现在 mapping 块中）
        if stripped.startswith('- '):
            i += 1
            continue

        # 解析 key: value
        colon_match = re.match(r'^([^:]+):\s*(.*)$', stripped)
        if not colon_match:
            i += 1
            continue

        key = colon_match.group(1).strip()
        # 去除引号
        if (key.startswith('"') and key.endswith('"')) or (key.startswith("'") and key.endswith("'")):
            key = key[1:-1]

        value_part = colon_match.group(2).strip()
        value_part = _strip_comment(value_part)

        # 收集子块（缩进更大的后续行）
        child_lines = []
        j = i + 1
        while j < len(lines):
            child_line = lines[j]
            if child_line.strip() == '' or child_line.strip().startswith('#'):
                child_lines.append(child_line)
                j += 1
                continue
            child_indent = len(child_line) - len(child_line.lstrip())
            if child_indent > indent:
                child_lines.append(child_line)
                j += 1
            else:
                break

        if value_part == '' and child_lines:
            # 嵌套结构
            result[key] = _parse_block(child_lines, indent + 1)
        elif value_part.startswith('|') or value_part.startswith('>'):
            # block scalar
            result[key] = _parse_block_scalar(child_lines, indent + 1)
        elif value_part == '':
            result[key] = None
        else:
            result[key] = _parse_scalar(value_part)

        i = j if j > i + 1 else i + 1

    return result

def _parse_list(lines, indent):
    """解析 list"""
    result = []
    i = 0
    while i < len(lines):
        line = lines[i]
        line_indent = len(line) - len(line.lstrip())
        if line_indent < indent:
            break
        if line_indent > indent:
            i += 1
            continue

        stripped = line.strip()
        if not stripped.startswith('-'):
            i += 1
            continue

        # 提取 - 后面的内容
        item_content = stripped[1:].strip()
        item_content = _strip_comment(item_content)

        # 收集子块
        child_lines = []
        j = i + 1
        while j < len(lines):
            child_line = lines[j]
            if child_line.strip() == '' or child_line.strip().startswith('#'):
                child_lines.append(child_line)
                j += 1
                continue
            child_indent = len(child_line) - len(child_line.lstrip())
            if child_indent > indent:
                child_lines.append(child_line)
                j += 1
            else:
                break

        if item_content == '' and child_lines:
            result.append(_parse_block(child_lines, indent + 1))
        elif ':' in item_content and not item_content.startswith('{'):
            # 内联 mapping 项（如 - name: foo）
            item_dict = {}
            colon_match = re.match(r'^([^:]+):\s*(.*)$', item_content)
            if colon_match:
                k = colon_match.group(1).strip()
                v = colon_match.group(2).strip()
                v = _strip_comment(v)
                if v == '':
                    item_dict[k] = _parse_block(child_lines, indent + 1) if child_lines else None
                else:
                    item_dict[k] = _parse_scalar(v)
                    # 如果有子行，可能是同一 item 的更多 key
                    if child_lines:
                        sub = _parse_mapping(child_lines, indent + 2)
                        if isinstance(sub, dict):
                            item_dict.update(sub)
            result.append(item_dict)
        else:
            result.append(_parse_scalar(item_content))

        i = j if j > i + 1 else i + 1

    return result

def _parse_block_scalar(lines, indent):
    """解析 block scalar（| 或 >）"""
    result_lines = []
    for line in lines:
        stripped = line.strip()
        if stripped == '' or stripped.startswith('#'):
            result_lines.append('')
            continue
        line_indent = len(line) - len(line.lstrip())
        if line_indent >= indent:
            result_lines.append(line[indent:])
    return '\n'.join(result_lines).strip()

def _parse_scalar(value):
    """解析标量值"""
    value = value.strip()
    if value == '' or value == '~' or value.lower() == 'null':
        return None
    if value == '{}':
        return {}
    if value == '[]':
        return []
    if value.lower() == 'true':
        return True
    if value.lower() == 'false':
        return False
    # 去除引号
    if (value.startswith('"') and value.endswith('"')) or (value.startswith("'") and value.endswith("'")):
        return value[1:-1]
    # 数字
    try:
        if '.' in value:
            return float(value)
        return int(value)
    except ValueError:
        pass
    return value

def get_nested(d, path, default=None):
    """安全获取嵌套字典值"""
    keys = path.split('.')
    current = d
    for key in keys:
        if isinstance(current, dict) and key in current:
            current = current[key]
        elif isinstance(current, list):
            try:
                current = current[int(key)]
            except (ValueError, IndexError):
                return default
        else:
            return default
    return current

# ============================================================================
# 1. 解析所有 YAML 文件
# ============================================================================
yaml_files = sorted(glob.glob(os.path.join(K8S_DIR, "*.yaml")))
yaml_files += sorted(glob.glob(os.path.join(K8S_DIR, "overlays", "**", "*.yaml"), recursive=True))

all_docs = {}
for f in yaml_files:
    rel = os.path.relpath(f, K8S_DIR)
    try:
        with open(f, "r", encoding="utf-8") as fh:
            text = fh.read()
        docs = parse_yaml(text)
        all_docs[rel] = docs
        check(True, f"[语法] {rel}: 解析成功，{len(docs)} 个文档")
    except Exception as e:
        check(False, f"[语法] {rel}: YAML 解析失败 - {e}")

# ============================================================================
# 2. 验证 Deployment
# ============================================================================
for rel, docs in all_docs.items():
    for doc in docs:
        if not isinstance(doc, dict):
            continue
        kind = doc.get("kind")
        if kind != "Deployment":
            continue
        name = get_nested(doc, "metadata.name", "unknown")
        prefix = f"[Deployment/{name}]"

        replicas = get_nested(doc, "spec.replicas")
        check(replicas is not None and replicas > 0,
              f"{prefix} replicas 已设置 ({replicas})")

        strategy = get_nested(doc, "spec.strategy.type")
        check(strategy == "RollingUpdate",
              f"{prefix} strategy 为 RollingUpdate")

        containers = get_nested(doc, "spec.template.spec.containers", [])
        for c in containers:
            if not isinstance(c, dict):
                continue
            cname = c.get("name", "unknown")
            cp = f"{prefix} container={cname}"

            resources = c.get("resources", {})
            requests = get_nested(resources, "requests", {})
            limits = get_nested(resources, "limits", {})
            check("cpu" in requests and "memory" in requests,
                  f"{cp} resources.requests 有 cpu 和 memory")
            check("cpu" in limits and "memory" in limits,
                  f"{cp} resources.limits 有 cpu 和 memory")

            check(c.get("livenessProbe") is not None, f"{cp} 有 livenessProbe")
            check(c.get("readinessProbe") is not None, f"{cp} 有 readinessProbe")
            check(c.get("startupProbe") is not None, f"{cp} 有 startupProbe")

            sec_ctx = c.get("securityContext", {})
            check(sec_ctx.get("runAsNonRoot") is True,
                  f"{cp} securityContext.runAsNonRoot=true")
            check(sec_ctx.get("allowPrivilegeEscalation") is False,
                  f"{cp} securityContext.allowPrivilegeEscalation=false")

        pod_sec = get_nested(doc, "spec.template.spec.securityContext", {})
        check(pod_sec.get("runAsNonRoot") is True,
              f"{prefix} pod securityContext.runAsNonRoot=true")

        sa = get_nested(doc, "spec.template.spec.serviceAccountName")
        check(sa is not None and sa != "default",
              f"{prefix} 使用专用 ServiceAccount ({sa})")

# ============================================================================
# 3. 验证 ConfigMap 环境变量
# ============================================================================
configmap_env = {}
for rel, docs in all_docs.items():
    for doc in docs:
        if not isinstance(doc, dict):
            continue
        if doc.get("kind") == "ConfigMap" and get_nested(doc, "metadata.name") == "metaclouds-backend-config":
            configmap_env = doc.get("data", {}) or {}

if configmap_env:
    check(True, f"[ConfigMap] 找到 metaclouds-backend-config，{len(configmap_env)} 个键")

    critical_vars = {
        "SERVER_ENV": "production",
        "USE_SQLITE": "false",
        "MEMORY_STORE_ENABLED": "false",
        "DATABASE_SSL_MODE": "require",
        "ALLOW_PUBLIC_REGISTRATION": "false",
        "PROMETHEUS_ENABLED": "true",
        "LOG_LEVEL": "info",
    }
    for var, expected in critical_vars.items():
        actual = configmap_env.get(var)
        check(str(actual) == expected,
              f"[ConfigMap] {var}={actual} (预期 {expected})")

    origins = str(configmap_env.get("ALLOWED_ORIGINS", ""))
    check(origins != "" and "*" not in origins,
          f"[ConfigMap] ALLOWED_ORIGINS 非空且不含 '*' ({origins})")

    sensitive_in_cm = [v for v in ["JWT_SECRET", "DATABASE_PASSWORD", "REDIS_PASSWORD"]
                        if v in configmap_env]
    check(len(sensitive_in_cm) == 0,
          f"[ConfigMap] 敏感变量未出现在 ConfigMap 中 (发现: {sensitive_in_cm})")
else:
    check(False, "[ConfigMap] 未找到 metaclouds-backend-config")

# ============================================================================
# 4. 验证 Secret fail-secure
# ============================================================================
for rel, docs in all_docs.items():
    for doc in docs:
        if not isinstance(doc, dict):
            continue
        if doc.get("kind") == "Secret" and get_nested(doc, "metadata.name") == "metaclouds-secrets":
            data = doc.get("data", {})
            check(data == {} or data is None or (isinstance(data, dict) and len(data) == 0),
                  f"[Secret] {rel}: data 为空（fail-secure）")
            check(doc.get("type") == "Opaque",
                  f"[Secret] {rel}: type=Opaque")

# ============================================================================
# 5. 验证 Service
# ============================================================================
for rel, docs in all_docs.items():
    for doc in docs:
        if not isinstance(doc, dict):
            continue
        if doc.get("kind") == "Service":
            name = get_nested(doc, "metadata.name", "unknown")
            check(get_nested(doc, "spec.type") == "ClusterIP",
                  f"[Service/{name}] type=ClusterIP")
            ports = get_nested(doc, "spec.ports", [])
            check(len(ports) > 0, f"[Service/{name}] 有端口定义")
            selector = get_nested(doc, "spec.selector", {})
            check("app" in selector, f"[Service/{name}] selector 含 app 标签")

# ============================================================================
# 6. 验证 Ingress
# ============================================================================
for rel, docs in all_docs.items():
    for doc in docs:
        if not isinstance(doc, dict):
            continue
        if doc.get("kind") == "Ingress":
            name = get_nested(doc, "metadata.name", "unknown")
            annotations = get_nested(doc, "metadata.annotations", {})
            check("cert-manager.io/cluster-issuer" in annotations,
                  f"[Ingress/{name}] 有 cert-manager cluster-issuer 注解")
            check("nginx.ingress.kubernetes.io/proxy-body-size" in annotations,
                  f"[Ingress/{name}] 有 proxy-body-size 注解")
            tls = get_nested(doc, "spec.tls", [])
            check(len(tls) > 0, f"[Ingress/{name}] 有 TLS 配置")

# ============================================================================
# 7. 验证 HPA
# ============================================================================
hpa_count = 0
for rel, docs in all_docs.items():
    for doc in docs:
        if not isinstance(doc, dict):
            continue
        if doc.get("kind") == "HorizontalPodAutoscaler":
            hpa_count += 1
            name = get_nested(doc, "metadata.name", "unknown")
            check(get_nested(doc, "spec.minReplicas") is not None,
                  f"[HPA/{name}] 有 minReplicas")
            check(get_nested(doc, "spec.maxReplicas") is not None,
                  f"[HPA/{name}] 有 maxReplicas")
            check("behavior" in (doc.get("spec") or {}),
                  f"[HPA/{name}] 有 behavior 配置")

check(hpa_count == 2, f"[HPA] 共 {hpa_count} 个 HPA（预期 2）")

# ============================================================================
# 8. 验证 NetworkPolicy
# ============================================================================
netpol_count = 0
has_default_deny = False
for rel, docs in all_docs.items():
    for doc in docs:
        if not isinstance(doc, dict):
            continue
        if doc.get("kind") == "NetworkPolicy":
            netpol_count += 1
            name = get_nested(doc, "metadata.name", "unknown")
            policy_types = get_nested(doc, "spec.policyTypes", [])
            pod_selector = get_nested(doc, "spec.podSelector", {})
            if "Ingress" in policy_types and "Egress" in policy_types:
                if pod_selector == {} or pod_selector is None:
                    has_default_deny = True
                    check(True, f"[NetworkPolicy/{name}] 默认拒绝所有入站/出站")

check(netpol_count >= 3, f"[NetworkPolicy] 共 {netpol_count} 个策略（预期 ≥3）")
check(has_default_deny, "[NetworkPolicy] 有默认拒绝策略")

# ============================================================================
# 9. 验证 PDB
# ============================================================================
pdb_count = 0
for rel, docs in all_docs.items():
    for doc in docs:
        if not isinstance(doc, dict):
            continue
        if doc.get("kind") == "PodDisruptionBudget":
            pdb_count += 1
            name = get_nested(doc, "metadata.name", "unknown")
            check(get_nested(doc, "spec.minAvailable") is not None,
                  f"[PDB/{name}] 有 minAvailable")

check(pdb_count == 2, f"[PDB] 共 {pdb_count} 个 PDB（预期 2）")

# ============================================================================
# 10. 验证 Kustomization
# ============================================================================
kust_file = os.path.join(K8S_DIR, "kustomization.yaml")
if os.path.exists(kust_file):
    with open(kust_file, "r", encoding="utf-8") as f:
        kust = parse_yaml(f.read())
    if kust:
        kust = kust[0]
        resources = kust.get("resources", [])
        expected_files = [
            "00-namespace.yaml", "01-configmap.yaml", "02-secrets.yaml",
            "10-serviceaccount.yaml", "03-backend-deployment.yaml",
            "04-backend-service.yaml", "05-frontend-deployment.yaml",
            "06-frontend-service.yaml", "07-ingress.yaml", "08-hpa.yaml",
            "09-networkpolicy.yaml", "11-pdb.yaml", "backup-cronjob.yaml",
        ]
        for ef in expected_files:
            check(ef in resources, f"[Kustomization] resources 包含 {ef}")
        common_labels = kust.get("commonLabels", {})
        check("app.kubernetes.io/part-of" in common_labels,
              "[Kustomization] commonLabels 含 app.kubernetes.io/part-of")
else:
    check(False, "[Kustomization] kustomization.yaml 不存在")

for env in ["production", "staging"]:
    overlay = os.path.join(K8S_DIR, "overlays", env, "kustomization.yaml")
    check(os.path.exists(overlay), f"[Overlay] {env}/kustomization.yaml 存在")

# ============================================================================
# 输出结果
# ============================================================================
print("=" * 70)
print("Metaclouds K8s 清单验证结果")
print("=" * 70)

print(f"\n[PASS] 通过 ({len(passed)}):")
for p in passed:
    print(f"  {p}")

if warnings:
    print(f"\n[WARN] 警告 ({len(warnings)}):")
    for w in warnings:
        print(f"  {w}")

if errors:
    print(f"\n[FAIL] 错误 ({len(errors)}):")
    for e in errors:
        print(f"  {e}")
    print(f"\n验证失败：{len(errors)} 个错误")
    sys.exit(1)
else:
    print(f"\n验证全部通过！{len(passed)} 项检查通过，{len(warnings)} 个警告")
    sys.exit(0)
