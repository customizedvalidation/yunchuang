# -*- coding: utf-8 -*-
"""
Metaclouds 386 台服务器全维度模拟数据灌入脚本
目标：7 大维度（Linux/Windows/Unix/MacOS/信创OS/NVIDIA GPU/信创GPU）
通过 Rust 后端 API（:8001）批量创建 5 集群 + 386 节点 + 390 GPU + 告警。
用法: python seed_sim_386_servers.py [base_url]
"""
import json
import random
import sys
import time
import urllib.request
import urllib.error

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:8001"
USER = "admin"
PASS = "Admin@123456"

# ---------- 1. 会话与认证 ----------
# 注意：不使用 CookieJar——携带 access_token Cookie 的写操作会触发 CSRF 双提交校验。
# 纯 Bearer 通道（无 cookie）时 CSRF 中间件跳过，写操作正常。
opener = urllib.request.build_opener()
opener.addheaders = [("User-Agent", "metaclouds-seed/1.0")]

def api(method, path, body=None, token=None, timeout=30):
    url = BASE + path
    data = None
    headers = {"Content-Type": "application/json; charset=utf-8", "Accept": "application/json"}
    if body is not None:
        data = json.dumps(body, ensure_ascii=False).encode("utf-8")
    if token:
        headers["Authorization"] = "Bearer " + token
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with opener.open(req, timeout=timeout) as resp:
            raw = resp.read().decode("utf-8")
            return resp.status, json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8", "ignore")
        try:
            return e.code, json.loads(raw)
        except Exception:
            return e.code, {"error": raw[:300]}

# 登录
code, login = api("POST", "/api/v1/auth/login", {"username": USER, "password": PASS})
if code != 200 or not login.get("success"):
    print("LOGIN FAILED:", code, login)
    sys.exit(1)
token = login["data"]["token"]
print(f"[1/6] 登录成功 admin role={login['data']['user']['role']}")

def expect_ok(code, payload, what):
    if code >= 400 or not payload.get("success", True):
        print(f"  !! {what}: HTTP {code} {str(payload)[:200]}")
        return None
    d = payload.get("data")
    if isinstance(d, dict):
        return d.get("id")
    return None

# ---------- 2. 集群 ----------
clusters = [
    ("metaclouds-prod", "生产主集群"),
    ("ai-training-cluster", "AI 训练集群"),
    ("inference-cluster", "推理服务集群"),
    ("domestic-x-cluster", "信创算力集群"),
    ("edge-cluster", "边缘节点集群"),
]
cluster_ids = {}
for name, desc in clusters:
    code, r = api("POST", "/api/v1/clusters", {"name": name, "description": desc}, token)
    cid = expect_ok(code, r, f"创建集群 {name}")
    if cid is None:
        # 可能已存在，查列表
        code, r = api("GET", "/api/v1/clusters", token=token)
        for c in r.get("data", []):
            if c.get("name") == name:
                cid = c["id"]
                break
    cluster_ids[name] = cid
    print(f"[2/6] 集群 {name} -> id={cid}")

# ---------- 3. 节点定义（386 台，7 大 OS 维度） ----------
OS_FAMILY = []  # (family, name, count, arch, cluster, role)
# Linux 150
linux_defs = [("Debian", 25, "x86_64"), ("RedHat", 25, "x86_64"), ("CentOS", 25, "x86_64"),
              ("Ubuntu", 35, "x86_64"), ("Fedora", 20, "x86_64"), ("SUSE", 20, "x86_64")]
# Windows 48
win_defs = [("Windows Server 2012", 12, "x86_64"), ("Windows Server 2016", 12, "x86_64"),
            ("Windows Server 2019", 12, "x86_64"), ("Windows Server 2022", 12, "x86_64")]
# Unix 32
unix_defs = [("Solaris", 8, "sparc"), ("FreeBSD", 8, "x86_64"), ("OpenBSD", 8, "x86_64"), ("AIX", 8, "powerpc")]
# MacOS 12
mac_defs = [("MacOS Amd64", 7, "amd64"), ("MacOS Arm64", 5, "arm64")]
# 信创 OS 144
xc_defs = [("ARM", 10, "aarch64"), ("Android", 8, "aarch64"), ("Riscv64", 6, "riscv64"), ("S390x", 4, "s390x"),
           ("树莓派", 6, "aarch64"), ("deepin", 10, "x86_64"), ("群晖", 4, "x86_64"), ("凝思", 8, "x86_64"),
           ("飞腾CPU", 10, "aarch64"), ("海光CPU", 10, "x86_64"), ("兆芯CPU", 8, "x86_64"), ("鲲鹏OS", 10, "aarch64"),
           ("龙蜥Anolis", 10, "x86_64"), ("EulerOS欧拉", 8, "x86_64"), ("BC-Linux", 6, "x86_64"),
           ("KeyarchOS", 6, "x86_64"), ("麒麟", 10, "x86_64"), ("统信UOS", 6, "x86_64"), ("龙芯MIPS", 4, "mips64")]

nodes_plan = []  # (hostname, family, os_name, arch, cluster_id, role, status)
seq = {}
def next_seq(key):
    seq[key] = seq.get(key, 0) + 1
    return seq[key]

def add_plan(family, os_name, count, arch, cluster, role):
    for i in range(1, count + 1):
        key = os_name.replace(" ", "").lower()[:10]
        nodes_plan.append((f"node-{key}-{next_seq(key):03d}", family, os_name, arch, cluster, role))

for name, cnt, arch in linux_defs:
    add_plan("Linux", name, cnt, arch, "metaclouds-prod" if name in ("CentOS", "Ubuntu", "Debian") else "ai-training-cluster", "compute")
for name, cnt, arch in win_defs:
    add_plan("Windows", name, cnt, arch, "metaclouds-prod", "compute")
for name, cnt, arch in unix_defs:
    add_plan("Unix", name, cnt, arch, "edge-cluster", "edge")
for name, cnt, arch in mac_defs:
    add_plan("MacOS", name, cnt, arch, "edge-cluster", "edge")
for name, cnt, arch in xc_defs:
    add_plan("信创", name, cnt, arch, "domestic-x-cluster", "compute")

print(f"[3/6] 节点计划总数 = {len(nodes_plan)}")
assert len(nodes_plan) == 386, f"节点计划 {len(nodes_plan)} != 386"

status_pool = ["online"] * 90 + ["maintenance"] * 6 + ["offline"] * 4
node_ids = {}  # hostname -> id
random.seed(42)
for hostname, family, os_name, arch, cluster, role in nodes_plan:
    status = random.choice(status_pool)
    cpu_cores = random.choice([16, 32, 64, 96, 128, 192])
    memory_gb = random.choice([128, 256, 512, 768, 1024, 2048])
    labels = {"os_family": family, "os_name": os_name, "os_version": "2026 LTS", "arch": arch,
              "region": "cn-east", "rack": f"rack-{random.randint(1, 30):02d}"}
    body = {"hostname": hostname, "cluster_id": cluster_ids[cluster], "ip": f"10.{random.randint(1,250)}.{random.randint(0,250)}.{random.randint(2,250)}",
            "role": role, "cpu_cores": cpu_cores, "memory_gb": memory_gb,
            "gpu_count": 0, "gpu_model": "", "status": status, "labels": labels}
    code, r = api("POST", "/api/v1/topology/nodes", body, token)
    cid = expect_ok(code, r, f"创建节点 {hostname}")
    if cid is not None:
        node_ids[hostname] = cid

print(f"[3/6] 节点创建完成: {len(node_ids)}/386")

# ---------- 4. GPU 定义（390 张） ----------
nvidia_defs = [("L20", 40), ("L40", 30), ("A100", 30), ("H100", 30), ("H800", 20),
               ("H20", 25), ("RTX 5090", 20), ("B200", 15), ("B300", 15), ("GB300", 15)]
xc_gpu_defs = [("海光", "BW2000", 20), ("华为", "910C", 20), ("寒武纪", "MLU690", 15), ("阿里", "PPU PG1", 10),
               ("昆仑芯", "P800", 15), ("沐曦", "C600", 15), ("天数", "天垓200", 10), ("摩尔线程", "S5000", 15),
               ("燧源", "L600", 15), ("华为", "950D", 15)]

# 挂载目标：NVIDIA -> ai-training/inference/prod 集群的 compute 节点；信创 -> domestic-x 的 compute 节点
def target_nodes(cluster_names, needed):
    pool = [h for h, _f, _o, _a, c, role in nodes_plan
            if c in cluster_names and role == "compute" and h in node_ids]
    out = []
    for i in range(needed):
        out.append(pool[i % len(pool)] if pool else "node-missing")
    return out

nvidia_targets = target_nodes(["ai-training-cluster", "inference-cluster", "metaclouds-prod"], 240)
xc_targets = target_nodes(["domestic-x-cluster"], 150)

gpu_created = 0
def create_gpu(node_name, vendor, model, cluster_name, idx, util, mem_total, mem_used, temp, power, status):
    global gpu_created
    body = {"cluster_id": cluster_ids[cluster_name], "node_name": node_name, "vendor": vendor, "model": model,
            "index": idx, "total_memory_gb": mem_total, "allocatable_memory_gb": mem_total,
            "used_memory_gb": mem_used, "mig_enabled": model in ("H100", "H20", "B200", "GB300"),
            "mig_profiles": "1g.10gb,2g.20gb" if model in ("H100", "H20", "B200", "GB300") else "",
            "driver_version": "570.124.06", "cuda_version": "12.8" if vendor == "nvidia" else "",
            "status": status, "utilization": util, "temperature": temp, "power_draw": power, "details": ""}
    code, r = api("POST", "/api/v1/gpus", body, token)
    cid = expect_ok(code, r, f"创建 GPU {vendor} {model}")
    if cid is not None:
        gpu_created += 1

gi = 0
for model, cnt in nvidia_defs:
    mem = {"L20": 48, "L40": 48, "A100": 80, "H100": 80, "H800": 80, "H20": 96,
           "RTX 5090": 32, "B200": 192, "B300": 288, "GB300": 288}.get(model, 80)
    for k in range(cnt):
        util = random.uniform(5, 98)
        create_gpu(nvidia_targets[gi], "nvidia", model, "ai-training-cluster", k % 8, util,
                   mem, int(mem * util / 100), random.randint(45, 85), random.randint(150, 700),
                   "available" if util < 90 else "allocated")
        gi += 1

mem_map = {"BW2000": 64, "910C": 64, "MLU690": 96, "PPU PG1": 48, "P800": 32, "C600": 64,
           "天垓200": 32, "S5000": 32, "L600": 48, "950D": 64}
xi = 0
for vendor, model, cnt in xc_gpu_defs:
    for k in range(cnt):
        util = random.uniform(5, 95)
        mem = mem_map.get(model, 48)
        create_gpu(xc_targets[xi], vendor, model, "domestic-x-cluster", k % 8, util,
                   mem, int(mem * util / 100), random.randint(45, 88), random.randint(120, 500),
                   "available" if util < 90 else "allocated")
        xi += 1
print(f"[4/6] GPU 创建完成: {gpu_created}/390")

# ---------- 5. 告警 ----------
alerts = [
    ("GPU 高利用率告警", "warning", "gpu", "H100 利用率持续超过 95%"),
    ("节点离线告警", "critical", "node", "compute 节点失联超过 5 分钟"),
    ("显存不足预警", "warning", "gpu", "B200 显存使用率接近 100%"),
    ("集群容量告警", "info", "cluster", "inference-cluster 剩余可调度 GPU 低于 10%"),
    ("高温告警", "critical", "node", "机架 rack-07 温度超过 85°C"),
    ("信创 GPU 驱动告警", "warning", "gpu", "华为 910C 驱动版本存在已知缺陷"),
    ("网络抖动告警", "warning", "network", "RoCE 网络延迟 P99 超过 200ms"),
    ("存储水位告警", "info", "storage", "共享存储使用率超过 80%"),
    ("作业失败率告警", "warning", "job", "训练作业失败率超过 5%"),
    ("配额接近上限", "info", "quota", "租户 GPU 配额使用率超过 90%"),
]
alert_created = 0
for name, sev, kind, msg in alerts:
    body = {"name": name, "severity": sev, "type": kind, "source": "seed-sim",
            "message": msg, "cluster_id": cluster_ids["metaclouds-prod"], "status": "active"}
    code, r = api("POST", "/api/v1/alerts", body, token)
    if expect_ok(code, r, f"创建告警 {name}") is not None:
        alert_created += 1
print(f"[5/6] 告警创建完成: {alert_created}/{len(alerts)}")

# 触发规则评估
code, r = api("POST", "/api/v1/monitoring/alert-rules/evaluate", {}, token)
print(f"[5b] 规则评估: {r.get('data', r)}")

# ---------- 6. 汇总 ----------
code, r = api("GET", "/api/v1/monitoring/metrics", token=token)
if code == 200:
    stats = r.get("data", {})
    print(f"[6/6] Dashboard 统计: {json.dumps(stats, ensure_ascii=False)}")
else:
    print("[6/6] metrics 拉取失败", code, r)

code, r = api("GET", "/api/v1/topology/nodes?page_size=1", token=token)
print(f"[6b] 节点列表可读: {code} {str(r.get('data', {}))[:120]}")
print("\n完成: 5 集群 / 386 节点 / 390 GPU /", alert_created, "告警")
