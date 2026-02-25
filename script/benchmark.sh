#!/usr/bin/env bash
#
# CodeCoder 性能基准测试脚本
# Performance Benchmark Script for CodeCoder
#
# 用于执行 NFR-04 性能验证基准测试并生成报告
#
# 用法:
#   ./script/benchmark.sh           - 运行完整基准测试套件
#   ./script/benchmark.sh startup   - 仅运行启动时间测试
#   ./script/benchmark.sh plan      - 仅运行 Plan 扫描测试
#   ./script/benchmark.sh api       - 仅运行 API 延迟测试
#   ./script/benchmark.sh report    - 生成性能报告 (Markdown)
#   ./script/benchmark.sh help      - 显示帮助信息
#
# NFR-04 性能目标 (from docs/standards/goals.md):
#   - ZeroBot 后台唤醒时间 ≤ 0.5秒
#   - Plan 模式扫描 (10万行) ≤ 15秒
#   - Gateway 内存 < 5MB
#

set -eo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 项目根目录
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CCODE_DIR="${PROJECT_ROOT}/packages/ccode"
REPORT_DIR="${PROJECT_ROOT}/docs/reports"
REPORT_FILE="${REPORT_DIR}/performance-$(date +%Y%m%d).md"

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

# 检查依赖
check_dependencies() {
    log_info "检查依赖..."

    if ! command -v bun &> /dev/null; then
        log_error "Bun 未安装。请安装 Bun 1.3+: https://bun.sh"
        exit 1
    fi

    local bun_version=$(bun --version)
    log_info "Bun 版本: ${bun_version}"
}

# 检查服务状态
check_services() {
    log_info "检查服务状态..."

    # 检查 API 服务
    if curl -s "http://localhost:4400/health" > /dev/null 2>&1; then
        log_success "API Server (4400) 运行中"
        return 0
    else
        log_warning "API Server (4400) 未运行 - API 延迟测试将跳过"
        return 1
    fi
}

# 运行完整基准测试
run_all_benchmarks() {
    log_info "运行完整基准测试套件..."
    echo ""

    cd "${CCODE_DIR}"
    bun run bench/index.ts

    local exit_code=$?

    echo ""
    if [ $exit_code -eq 0 ]; then
        log_success "所有基准测试通过"
    else
        log_error "部分基准测试失败"
    fi

    return $exit_code
}

# 运行启动时间测试
run_startup_benchmarks() {
    log_info "运行启动时间基准测试..."
    echo ""

    cd "${CCODE_DIR}"
    bun run -e '
import { runStartupBenchmarks } from "./bench/startup.bench"
runStartupBenchmarks().then(results => {
    console.log(JSON.stringify(results, null, 2))
})
'
}

# 运行 Plan 扫描测试
run_plan_benchmarks() {
    log_info "运行 Plan 扫描基准测试..."
    echo ""

    cd "${CCODE_DIR}"
    bun run -e '
import { runPlanScanBenchmarks } from "./bench/plan-scan.bench"
runPlanScanBenchmarks().then(results => {
    console.log(JSON.stringify(results, null, 2))
})
'
}

# 运行 API 延迟测试
run_api_benchmarks() {
    log_info "运行 API 延迟基准测试..."
    echo ""

    if ! check_services; then
        log_warning "启动 API 服务后重试: ./ops.sh start api"
        return 1
    fi

    cd "${CCODE_DIR}"
    bun run -e '
import { runApiLatencyBenchmarks } from "./bench/api-latency.bench"
runApiLatencyBenchmarks().then(results => {
    console.log(JSON.stringify(results, null, 2))
})
'
}

# 生成性能报告
generate_report() {
    log_info "生成性能报告..."

    # 确保报告目录存在
    mkdir -p "${REPORT_DIR}"

    cd "${CCODE_DIR}"

    # 运行基准测试并捕获输出
    local output
    output=$(bun run bench/index.ts 2>&1) || true

    # 提取报告部分
    local report_content
    report_content=$(echo "$output" | sed -n '/=== CodeCoder Performance Report ===/,$p')

    # 生成 Markdown 报告
    cat > "${REPORT_FILE}" << EOF
# CodeCoder 性能验证报告

**生成时间**: $(date '+%Y-%m-%d %H:%M:%S')

## NFR-04 性能目标 (from docs/standards/goals.md)

| 指标 | 目标 |
|------|------|
| ZeroBot 后台唤醒时间 | ≤ 0.5秒 |
| Plan 模式扫描 (10万行) | ≤ 15秒 |
| Gateway 内存 | < 5MB |

## 测试结果

\`\`\`
${report_content}
\`\`\`

## 测试环境

- **平台**: $(uname -s) $(uname -r)
- **架构**: $(uname -m)
- **CPU**: $(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo "N/A") 核心
- **内存**: $(sysctl -n hw.memsize 2>/dev/null | awk '{print int($1/1024/1024/1024)"GB"}' || free -h 2>/dev/null | awk '/^Mem:/{print $2}' || echo "N/A")
- **Bun 版本**: $(bun --version)

## 验收标准

根据 \`docs/standards/goals.md\` NFR-04 要求:

1. ✅/❌ ZeroBot 后台唤醒时间 ≤ 0.5秒
2. ✅/❌ Plan 模式扫描 (10万行) ≤ 15秒
3. ✅/❌ Gateway 内存 < 5MB (如已部署)

## 备注

- 如部分测试显示 "Not Built" 或 "Server Offline"，表示对应服务未运行/未编译
- 运行 \`./ops.sh build rust\` 构建 Rust 服务
- 运行 \`./ops.sh start all\` 启动所有服务后重新测试

---
*此报告由 script/benchmark.sh 自动生成*
EOF

    log_success "报告已生成: ${REPORT_FILE}"
    echo ""
    cat "${REPORT_FILE}"
}

# 显示帮助
show_help() {
    cat << EOF
CodeCoder 性能基准测试脚本

用法:
  ./script/benchmark.sh [command]

命令:
  (无参数)   运行完整基准测试套件
  startup    仅运行启动时间测试
  plan       仅运行 Plan 扫描测试
  api        仅运行 API 延迟测试
  report     运行测试并生成 Markdown 报告
  help       显示此帮助信息

示例:
  ./script/benchmark.sh              # 运行所有测试
  ./script/benchmark.sh startup      # 仅测试启动时间
  ./script/benchmark.sh report       # 生成完整报告

NFR-04 性能目标:
  - ZeroBot 后台唤醒时间 ≤ 0.5秒
  - Plan 模式扫描 (10万行) ≤ 15秒
  - Gateway 内存 < 5MB

EOF
}

# 主函数
main() {
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║   CodeCoder Performance Benchmark      ║${NC}"
    echo -e "${CYAN}║   NFR-04 Validation Suite              ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
    echo ""

    check_dependencies
    echo ""

    case "${1:-all}" in
        startup)
            run_startup_benchmarks
            ;;
        plan)
            run_plan_benchmarks
            ;;
        api)
            run_api_benchmarks
            ;;
        report)
            generate_report
            ;;
        help|--help|-h)
            show_help
            ;;
        all|*)
            run_all_benchmarks
            ;;
    esac
}

main "$@"
