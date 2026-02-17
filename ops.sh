#!/usr/bin/env bash
#
# CodeCoder 服务运维脚本
# 用于管理项目所有服务的启动、停止、状态查看
#
# 用法:
#   ./ops.sh start [service]   - 启动服务 (all|api|web)
#   ./ops.sh stop [service]    - 停止服务 (all|api|web)
#   ./ops.sh restart [service] - 重启服务 (all|api|web)
#   ./ops.sh status            - 查看所有服务状态
#   ./ops.sh logs [service]    - 查看服务日志
#   ./ops.sh help              - 显示帮助信息
#

set -eo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 项目根目录
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_DIR="${PROJECT_ROOT}/.pids"
LOG_DIR="${PROJECT_ROOT}/.logs"

# 服务列表
ALL_SERVICES="api web zerobot"

# 服务配置函数
get_service_port() {
    case "$1" in
        api) echo "4400" ;;
        web) echo "4401" ;;
        zerobot) echo "4402" ;;
        *) echo "" ;;
    esac
}

get_service_name() {
    case "$1" in
        api) echo "CodeCoder API Server" ;;
        web) echo "Web Frontend (Vite)" ;;
        zerobot) echo "ZeroBot Daemon" ;;
        *) echo "" ;;
    esac
}

is_valid_service() {
    case "$1" in
        api|web|zerobot) return 0 ;;
        *) return 1 ;;
    esac
}

# 初始化目录
init_dirs() {
    mkdir -p "${PID_DIR}"
    mkdir -p "${LOG_DIR}"
}

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 获取服务 PID 文件路径
get_pid_file() {
    echo "${PID_DIR}/$1.pid"
}

# 获取服务日志文件路径
get_log_file() {
    echo "${LOG_DIR}/$1.log"
}

# 检查服务是否运行
is_running() {
    local service="$1"
    local pid_file
    pid_file=$(get_pid_file "${service}")

    if [ -f "${pid_file}" ]; then
        local pid
        pid=$(cat "${pid_file}")
        if kill -0 "${pid}" 2>/dev/null; then
            return 0
        fi
    fi
    return 1
}

# 获取运行中的 PID
get_pid() {
    local service="$1"
    local pid_file
    pid_file=$(get_pid_file "${service}")

    if [ -f "${pid_file}" ]; then
        cat "${pid_file}"
    fi
}

# 检查端口是否被占用
check_port() {
    local port="$1"
    if lsof -i :"${port}" >/dev/null 2>&1; then
        return 0
    fi
    return 1
}

# 启动单个服务
start_service() {
    local service="$1"
    local service_name
    service_name=$(get_service_name "${service}")
    local port
    port=$(get_service_port "${service}")
    local log_file
    log_file=$(get_log_file "${service}")
    local pid_file
    pid_file=$(get_pid_file "${service}")

    log_info "正在启动 ${service_name}..."

    # 检查是否已经运行
    if is_running "${service}"; then
        local pid
        pid=$(get_pid "${service}")
        log_warn "${service_name} 已经在运行 (PID: ${pid})"
        return 0
    fi

    # 检查端口是否被占用
    if check_port "${port}"; then
        log_error "端口 ${port} 已被占用，无法启动 ${service_name}"
        return 1
    fi

    # 启动服务
    case "${service}" in
        api)
            cd "${PROJECT_ROOT}"
            nohup bun run --cwd packages/ccode --conditions=browser src/index.ts serve --port 4400 \
                > "${log_file}" 2>&1 &
            ;;
        web)
            cd "${PROJECT_ROOT}/packages/web"
            nohup bun run dev --port 4401 > "${log_file}" 2>&1 &
            ;;
        zerobot)
            cd "${PROJECT_ROOT}/services/zero-bot"
            nohup cargo run --release -- daemon --port 4402 > "${log_file}" 2>&1 &
            ;;
        *)
            log_error "未知服务: ${service}"
            return 1
            ;;
    esac

    local pid=$!
    echo "${pid}" > "${pid_file}"

    # 等待服务启动
    sleep 2

    if is_running "${service}"; then
        log_success "${service_name} 启动成功 (PID: ${pid}, Port: ${port})"
    else
        log_error "${service_name} 启动失败，请检查日志: ${log_file}"
        rm -f "${pid_file}"
        return 1
    fi
}

# 停止单个服务
stop_service() {
    local service="$1"
    local service_name
    service_name=$(get_service_name "${service}")
    local pid_file
    pid_file=$(get_pid_file "${service}")

    log_info "正在停止 ${service_name}..."

    if ! is_running "${service}"; then
        log_warn "${service_name} 未在运行"
        rm -f "${pid_file}"
        return 0
    fi

    local pid
    pid=$(get_pid "${service}")

    # 尝试优雅停止
    kill "${pid}" 2>/dev/null || true

    # 等待进程退出
    local count=0
    while kill -0 "${pid}" 2>/dev/null && [ ${count} -lt 10 ]; do
        sleep 1
        count=$((count + 1))
    done

    # 如果还没停止，强制终止
    if kill -0 "${pid}" 2>/dev/null; then
        log_warn "服务未响应，强制终止..."
        kill -9 "${pid}" 2>/dev/null || true
    fi

    rm -f "${pid_file}"
    log_success "${service_name} 已停止"
}

# 重启单个服务
restart_service() {
    local service="$1"
    stop_service "${service}"
    sleep 1
    start_service "${service}"
}

# 启动所有服务
start_all() {
    log_info "启动所有服务..."
    echo ""
    for service in ${ALL_SERVICES}; do
        start_service "${service}"
    done
}

# 停止所有服务
stop_all() {
    log_info "停止所有服务..."
    echo ""
    for service in ${ALL_SERVICES}; do
        stop_service "${service}"
    done
}

# 重启所有服务
restart_all() {
    stop_all
    echo ""
    start_all
}

# 显示服务状态
show_status() {
    echo ""
    echo "╔══════════════════════════════════════════════════════════════════╗"
    echo "║                    CodeCoder 服务状态                             ║"
    echo "╠══════════════════════════════════════════════════════════════════╣"
    printf "║ %-25s │ %-10s │ %-8s │ %-6s ║\n" "服务" "状态" "PID" "端口"
    echo "╠══════════════════════════════════════════════════════════════════╣"

    for service in ${ALL_SERVICES}; do
        local service_name
        service_name=$(get_service_name "${service}")
        local port
        port=$(get_service_port "${service}")
        local status
        local pid="-"

        if is_running "${service}"; then
            status="${GREEN}运行中${NC}"
            pid=$(get_pid "${service}")
        else
            status="${RED}已停止${NC}"
        fi

        printf "║ %-25s │ %b%-2s │ %-8s │ %-6s ║\n" "${service_name}" "${status}" "" "${pid}" "${port}"
    done

    echo "╚══════════════════════════════════════════════════════════════════╝"
    echo ""

    # 显示端口占用情况
    echo "端口占用检查:"
    for service in ${ALL_SERVICES}; do
        local port
        port=$(get_service_port "${service}")
        if check_port "${port}"; then
            echo -e "  端口 ${port}: ${GREEN}已占用${NC}"
        else
            echo -e "  端口 ${port}: ${YELLOW}空闲${NC}"
        fi
    done
    echo ""
}

# 查看服务日志
show_logs() {
    local service="$1"
    local log_file
    log_file=$(get_log_file "${service}")
    local service_name
    service_name=$(get_service_name "${service}")

    if [ ! -f "${log_file}" ]; then
        log_error "日志文件不存在: ${log_file}"
        return 1
    fi

    log_info "显示 ${service_name} 日志 (最后 50 行):"
    echo "----------------------------------------"
    tail -n 50 "${log_file}"
}

# 实时查看日志
tail_logs() {
    local service="$1"
    local log_file
    log_file=$(get_log_file "${service}")
    local service_name
    service_name=$(get_service_name "${service}")

    if [ ! -f "${log_file}" ]; then
        log_error "日志文件不存在: ${log_file}"
        return 1
    fi

    log_info "实时跟踪 ${service_name} 日志 (Ctrl+C 退出):"
    echo "----------------------------------------"
    tail -f "${log_file}"
}

# 显示帮助信息
show_help() {
    echo ""
    echo "CodeCoder 服务运维脚本"
    echo ""
    echo "用法: ./ops.sh <命令> [服务名]"
    echo ""
    echo "命令:"
    echo "  start [service]    启动服务"
    echo "  stop [service]     停止服务"
    echo "  restart [service]  重启服务"
    echo "  status             查看所有服务状态"
    echo "  logs <service>     查看服务日志 (最后 50 行)"
    echo "  tail <service>     实时跟踪服务日志"
    echo "  help               显示此帮助信息"
    echo ""
    echo "服务名:"
    echo "  all                所有服务 (默认)"
    echo "  api                CodeCoder API Server (端口 4400)"
    echo "  web                Web Frontend (端口 4401)"
    echo "  zerobot            ZeroBot Daemon (端口 4402)"
    echo ""
    echo "示例:"
    echo "  ./ops.sh start            # 启动所有服务"
    echo "  ./ops.sh start api        # 只启动 API 服务"
    echo "  ./ops.sh stop web         # 只停止 Web 服务"
    echo "  ./ops.sh restart zerobot  # 重启 ZeroBot"
    echo "  ./ops.sh status           # 查看状态"
    echo "  ./ops.sh logs api         # 查看 API 日志"
    echo "  ./ops.sh tail web         # 实时跟踪 Web 日志"
    echo ""
}

# 主函数
main() {
    init_dirs

    local command="${1:-help}"
    local service="${2:-all}"

    case "${command}" in
        start)
            if [ "${service}" = "all" ]; then
                start_all
            elif is_valid_service "${service}"; then
                start_service "${service}"
            else
                log_error "未知服务: ${service}"
                show_help
                exit 1
            fi
            ;;
        stop)
            if [ "${service}" = "all" ]; then
                stop_all
            elif is_valid_service "${service}"; then
                stop_service "${service}"
            else
                log_error "未知服务: ${service}"
                show_help
                exit 1
            fi
            ;;
        restart)
            if [ "${service}" = "all" ]; then
                restart_all
            elif is_valid_service "${service}"; then
                restart_service "${service}"
            else
                log_error "未知服务: ${service}"
                show_help
                exit 1
            fi
            ;;
        status)
            show_status
            ;;
        logs)
            if [ "${service}" = "all" ]; then
                log_error "请指定具体服务名"
                exit 1
            elif is_valid_service "${service}"; then
                show_logs "${service}"
            else
                log_error "未知服务: ${service}"
                exit 1
            fi
            ;;
        tail)
            if [ "${service}" = "all" ]; then
                log_error "请指定具体服务名"
                exit 1
            elif is_valid_service "${service}"; then
                tail_logs "${service}"
            else
                log_error "未知服务: ${service}"
                exit 1
            fi
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            log_error "未知命令: ${command}"
            show_help
            exit 1
            ;;
    esac
}

main "$@"
