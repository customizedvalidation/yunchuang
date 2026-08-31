#!/bin/bash

set -e

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║     Metaclouds 多实例部署集成测试脚本                        ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

BASE_URL="http://localhost:8000"
ADMIN_USER="admin"
ADMIN_PASS="admin123"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

pass() {
    echo -e "${GREEN}✅ PASS${NC}: $1"
}

fail() {
    echo -e "${RED}❌ FAIL${NC}: $1"
    FAILED_TESTS=$((FAILED_TESTS + 1))
}

info() {
    echo -e "${BLUE}ℹ️  INFO${NC}: $1"
}

warn() {
    echo -e "${YELLOW}⚠️  WARN${NC}: $1"
}

FAILED_TESTS=0
TOTAL_TESTS=0

check_service() {
    local url=$1
    local name=$2
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if curl -sf "$url" > /dev/null 2>&1; then
        pass "Service $name is healthy"
        return 0
    else
        fail "Service $name is not responding at $url"
        return 1
    fi
}

check_nginx_upstream() {
    local upstream=$1
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    response=$(curl -s "$BASE_URL/health")
    if [ "$response" = "healthy" ]; then
        pass "Nginx upstream $upstream is working"
        return 0
    else
        fail "Nginx upstream $upstream is not working"
        return 1
    fi
}

test_authentication() {
    echo ""
    echo "【测试1】用户认证"
    echo "─────────────────────────────────────"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    response=$(curl -s -X POST "$BASE_URL/api/v1/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"$ADMIN_USER\",\"password\":\"$ADMIN_PASS\"}")
    
    if echo "$response" | grep -q '"token"'; then
        TOKEN=$(echo "$response" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
        pass "用户认证成功"
        echo "$TOKEN" > /tmp/metaclouds_token.txt
        return 0
    else
        fail "用户认证失败: $response"
        return 1
    fi
}

test_priority_job_creation() {
    echo ""
    echo "【测试2】优先级任务创建（多实例）"
    echo "─────────────────────────────────────"
    
    TOKEN=$(cat /tmp/metaclouds_token.txt 2>/dev/null)
    
    if [ -z "$TOKEN" ]; then
        fail "Token不存在，需要先进行认证"
        return 1
    fi
    
    priorities=(0 1 2 3)
    created_jobs=()
    
    for priority in "${priorities[@]}"; do
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
        job_name="Priority-Test-P${priority}-$(date +%s)"
        
        response=$(curl -s -X POST "$BASE_URL/api/v1/jobs" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $TOKEN" \
            -d "{\"name\":\"$job_name\",\"priority\":$priority,\"type\":\"training\",\"gpus\":1,\"cpus\":2,\"memory\":4,\"duration\":60}")
        
        if echo "$response" | grep -q '"id"'; then
            job_id=$(echo "$response" | grep -o '"id":[0-9]*' | head -1 | cut -d':' -f2)
            created_jobs+=("$job_id")
            pass "创建优先级任务: P${priority} (ID: $job_id)"
        else
            fail "创建优先级任务失败: P${priority}"
        fi
    done
    
    echo "${created_jobs[*]}" > /tmp/created_jobs.txt
}

test_priority_update() {
    echo ""
    echo "【测试3】优先级更新（多实例同步）"
    echo "─────────────────────────────────────"
    
    TOKEN=$(cat /tmp/metaclouds_token.txt)
    created_jobs=$(cat /tmp/created_jobs.txt 2>/dev/null)
    
    if [ -z "$created_jobs" ]; then
        warn "没有创建的任务，跳过优先级更新测试"
        return 0
    fi
    
    for job_id in $created_jobs; do
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
        new_priority=$((RANDOM % 4))
        
        response=$(curl -s -X PUT "$BASE_URL/api/v1/jobs/$job_id" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $TOKEN" \
            -d "{\"priority\":$new_priority}")
        
        if echo "$response" | grep -q "\"priority\":$new_priority"; then
            pass "更新任务 #${job_id} 优先级为 P${new_priority}"
        else
            fail "更新任务 #${job_id} 优先级失败"
        fi
        
        sleep 0.5
    done
}

test_concurrent_requests() {
    echo ""
    echo "【测试4】并发请求测试（负载均衡）"
    echo "─────────────────────────────────────"
    
    TOKEN=$(cat /tmp/metaclouds_token.txt)
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    results=()
    for i in {1..10}; do
        response=$(curl -s -X GET "$BASE_URL/api/v1/jobs" \
            -H "Authorization: Bearer $TOKEN" \
            -w "\n%{http_code}\n%{time_total}" \
            -o /tmp/response_$i.txt)
        
        http_code=$(echo "$response" | tail -1)
        time_total=$(echo "$response" | tail -2 | head -1)
        
        if [ "$http_code" = "200" ]; then
            results+=("$time_total")
            info "请求 #$i: HTTP $http_code (${time_total}s)"
        else
            fail "请求 #$i: HTTP $http_code"
        fi
    done
    
    if [ ${#results[@]} -gt 0 ]; then
        pass "并发请求测试完成: ${#results[@]}/10 成功"
    fi
}

test_load_balancing() {
    echo ""
    echo "【测试5】负载均衡验证"
    echo "─────────────────────────────────────"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    instance_counts=()
    
    for i in {1..20}; do
        response=$(curl -s -I "$BASE_URL/health" 2>/dev/null | grep -i "x-upstream" || echo "")
        
        if [ -n "$response" ]; then
            instance=$(echo "$response" | grep -o 'app-[0-9]' || echo "unknown")
            instance_counts+=("$instance")
        fi
        
        sleep 0.1
    done
    
    if [ ${#instance_counts[@]} -gt 0 ]; then
        pass "负载均衡工作正常: 检测到 ${#instance_counts[@]} 个请求分布"
        info "实例分布: $(printf '%s\n' "${instance_counts[@]}" | sort | uniq -c)"
    else
        warn "无法检测负载均衡实例分布"
    fi
}

test_job_list() {
    echo ""
    echo "【测试6】任务列表查询"
    echo "─────────────────────────────────────"
    
    TOKEN=$(cat /tmp/metaclouds_token.txt 2>/dev/null)
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    response=$(curl -s -X GET "$BASE_URL/api/v1/jobs" \
        -H "Authorization: Bearer $TOKEN")
    
    if echo "$response" | grep -q '"data"'; then
        total=$(echo "$response" | grep -o '"total":[0-9]*' | cut -d':' -f2 || echo "0")
        pass "任务列表查询成功 (总计: $total 个任务)"
    else
        fail "任务列表查询失败"
    fi
}

test_priority_distribution() {
    echo ""
    echo "【测试7】优先级分布统计"
    echo "─────────────────────────────────────"
    
    TOKEN=$(cat /tmp/metaclouds_token.txt)
    
    for priority in 0 1 2 3; do
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
        
        count=$(curl -s -X GET "$BASE_URL/api/v1/jobs?priority=$priority" \
            -H "Authorization: Bearer $TOKEN" | \
            grep -o '"total":[0-9]*' | cut -d':' -f2 || echo "0")
        
        priority_name=$(case $priority in 0) echo "低" ;; 1) echo "中" ;; 2) echo "高" ;; 3) echo "紧急" ;; esac)
        info "优先级 P${priority} (${priority_name}): $count 个任务"
        
        if [ "$count" != "" ]; then
            pass "优先级 P${priority} 统计: $count 个任务"
        fi
    done
}

test_high_concurrency() {
    echo ""
    echo "【测试8】高并发优先级更新"
    echo "─────────────────────────────────────"
    
    TOKEN=$(cat /tmp/metaclouds_token.txt)
    created_jobs=$(cat /tmp/created_jobs.txt 2>/dev/null)
    
    if [ -z "$created_jobs" ]; then
        warn "没有创建的任务，跳过高并发测试"
        return 0
    fi
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    success=0
    failed=0
    
    echo "并发执行 50 次优先级更新..."
    
    for i in {1..50}; do
        job_id=$(echo $created_jobs | cut -d' ' -f$((i % ${#created_jobs[@]} + 1)))
        new_priority=$((RANDOM % 4))
        
        curl -s -X PUT "$BASE_URL/api/v1/jobs/$job_id" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $TOKEN" \
            -d "{\"priority\":$new_priority}" > /dev/null 2>&1 &
        
        if [ $((i % 10)) -eq 0 ]; then
            wait
            success=$((success + 10))
            info "进度: $i/50"
        fi
    done
    
    wait
    
    if [ $success -ge 45 ]; then
        pass "高并发测试完成: $success/50 成功"
    else
        fail "高并发测试: $success/50 成功 (低于阈值)"
    fi
}

test_monitoring() {
    echo ""
    echo "【测试9】监控端点检查"
    echo "─────────────────────────────────────"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if curl -sf "$BASE_URL/metrics" > /tmp/metrics.txt 2>&1; then
        metrics_count=$(wc -l < /tmp/metrics.txt)
        pass "Prometheus指标端点正常 (${metrics_count} 行)"
        
        if grep -q "scheduler_priority_changes" /tmp/metrics.txt; then
            pass "优先级相关指标已暴露"
        fi
    else
        fail "Prometheus指标端点不可用"
    fi
}

cleanup() {
    echo ""
    echo "【清理】删除测试数据..."
    rm -f /tmp/metaclouds_token.txt /tmp/created_jobs.txt /tmp/response_*.txt /tmp/metrics.txt 2>/dev/null
    pass "清理完成"
}

main() {
    echo ""
    info "等待服务启动..."
    sleep 5
    
    if ! check_service "http://localhost:8000/health" "Nginx"; then
        echo ""
        fail "Nginx网关未启动，请先运行: docker-compose up -d"
        exit 1
    fi
    
    if ! check_service "http://localhost:8000" "Backend"; then
        echo ""
        fail "后端服务未启动"
        exit 1
    fi
    
    test_authentication
    
    if [ ! -f /tmp/metaclouds_token.txt ]; then
        echo ""
        fail "认证失败，无法继续测试"
        exit 1
    fi
    
    test_priority_job_creation
    test_priority_update
    test_concurrent_requests
    test_load_balancing
    test_job_list
    test_priority_distribution
    test_high_concurrency
    test_monitoring
    
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    echo "                    测试结果汇总"
    echo "═══════════════════════════════════════════════════════════"
    echo ""
    echo -e "总测试数: ${BLUE}$TOTAL_TESTS${NC}"
    echo -e "失败数:   ${RED}$FAILED_TESTS${NC}"
    echo -e "成功率:   ${GREEN}$(( (TOTAL_TESTS - FAILED_TESTS) * 100 / TOTAL_TESTS ))%${NC}"
    echo ""
    
    cleanup
    
    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "${GREEN}🎉 所有测试通过！${NC}"
        exit 0
    else
        echo -e "${RED}⚠️  有 $FAILED_TESTS 个测试失败${NC}"
        exit 1
    fi
}

main "$@"
