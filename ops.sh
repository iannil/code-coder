#!/usr/bin/env bash
#
# CodeCoder 服务运维脚本
# 用于管理项目所有服务的启动、停止、状态查看
#
# 架构说明:
#   - api:          CodeCoder API Server (Bun/TypeScript)
#   - web:          Web Frontend (Vite/React)
#   - zero-daemon:  Zero CLI 组合守护进程 (Rust) - 包含 gateway + channels + scheduler
#   - zero-gateway: 独立网关服务 (Rust) - 认证/路由/配额
#   - zero-channels: 独立频道服务 (Rust) - Telegram/Discord/Slack
#   - zero-workflow: 独立工作流服务 (Rust) - Webhook/Cron/Git
#   - whisper:      Whisper STT Server (Docker)
#
# 用法:
#   ./ops.sh start [service]   - 启动服务 (all|api|web|zero-daemon|...)
#   ./ops.sh stop [service]    - 停止服务
#   ./ops.sh restart [service] - 重启服务
#   ./ops.sh status            - 查看所有服务状态
#   ./ops.sh logs [service]    - 查看服务日志 (支持 all|core|<service>)
#   ./ops.sh tail [service]    - 实时跟踪日志 (支持 all|core|running|<service>)
#   ./ops.sh build [rust]      - 构建服务
#   ./ops.sh help              - 显示帮助信息
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
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_DIR="${PROJECT_ROOT}/.pids"
LOG_DIR="${PROJECT_ROOT}/.logs"
RUST_SERVICES_DIR="${PROJECT_ROOT}/services"
RUST_TARGET_DIR="${RUST_SERVICES_DIR}/target/release"

# Docker 容器名称
WHISPER_CONTAINER="codecoder-whisper"
WHISPER_IMAGE="${WHISPER_IMAGE:-fedirz/faster-whisper-server:latest-cpu}"

# 服务列表 (按启动顺序)
# 核心服务
CORE_SERVICES="api web zero-daemon whisper"
# 独立 Rust 服务 (可选，用于模块化部署)
STANDALONE_RUST_SERVICES="zero-gateway zero-channels zero-workflow"
# 所有服务
ALL_SERVICES="${CORE_SERVICES} ${STANDALONE_RUST_SERVICES}"

# 服务配置函数
get_service_port() {
    case "$1" in
        api) echo "4400" ;;
        web) echo "4401" ;;
        zero-daemon) echo "4402" ;;
        whisper) echo "4403" ;;
        zero-gateway) echo "4410" ;;
        zero-channels) echo "4411" ;;
        zero-workflow) echo "4412" ;;
        *) echo "" ;;
    esac
}

get_service_name() {
    case "$1" in
        api) echo "CodeCoder API Server" ;;
        web) echo "Web Frontend (Vite)" ;;
        zero-daemon) echo "Zero CLI Daemon" ;;
        whisper) echo "Whisper STT Server" ;;
        zero-gateway) echo "Zero Gateway" ;;
        zero-channels) echo "Zero Channels" ;;
        zero-workflow) echo "Zero Workflow" ;;
        *) echo "" ;;
    esac
}

get_service_type() {
    case "$1" in
        api|web) echo "node" ;;
        zero-daemon|zero-gateway|zero-channels|zero-workflow) echo "rust" ;;
        whisper) echo "docker" ;;
        *) echo "" ;;
    esac
}

is_valid_service() {
    case "$1" in
        api|web|zero-daemon|whisper|zero-gateway|zero-channels|zero-workflow) return 0 ;;
        *) return 1 ;;
    esac
}

is_core_service() {
    case "$1" in
        api|web|zero-daemon|whisper) return 0 ;;
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

log_debug() {
    if [ "${DEBUG:-}" = "1" ]; then
        echo -e "${CYAN}[DEBUG]${NC} $1"
    fi
}

# 获取服务 PID 文件路径
get_pid_file() {
    echo "${PID_DIR}/$1.pid"
}

# 获取服务日志文件路径
get_log_file() {
    echo "${LOG_DIR}/$1.log"
}

# 检查 Rust 二进制是否已构建
is_rust_binary_built() {
    local service="$1"
    local binary_name

    case "${service}" in
        zero-daemon) binary_name="zero-cli" ;;
        *) binary_name="${service}" ;;
    esac

    [ -f "${RUST_TARGET_DIR}/${binary_name}" ]
}

# 检查依赖是否安装
check_dependencies() {
    local service="$1"
    local service_type
    service_type=$(get_service_type "${service}")

    case "${service_type}" in
        node)
            if ! command -v bun &> /dev/null; then
                log_error "Bun 未安装"
                echo "  请先安装 Bun: curl -fsSL https://bun.sh/install | bash"
                return 1
            fi
            ;;
        rust)
            if ! command -v cargo &> /dev/null; then
                log_error "Cargo 未安装"
                echo "  请先安装 Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
                return 1
            fi
            ;;
        docker)
            if ! command -v docker &> /dev/null; then
                log_error "Docker 未安装"
                echo "  请先安装 Docker Desktop"
                return 1
            fi
            ;;
    esac
    return 0
}

# 构建 Rust 服务
build_rust_services() {
    log_info "构建 Rust 服务..."
    cd "${RUST_SERVICES_DIR}"

    if command -v cargo &> /dev/null; then
        cargo build --release
        log_success "Rust 服务构建完成"
    else
        log_error "Cargo 未安装，无法构建 Rust 服务"
        return 1
    fi
}

# 检查服务是否运行
is_running() {
    local service="$1"
    local service_type
    service_type=$(get_service_type "${service}")

    # Docker 容器
    if [ "${service_type}" = "docker" ]; then
        if docker ps --format '{{.Names}}' 2>/dev/null | grep -q "^${WHISPER_CONTAINER}$"; then
            return 0
        fi
        return 1
    fi

    # PID 文件方式
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
    local service_type
    service_type=$(get_service_type "${service}")

    # Docker 容器
    if [ "${service_type}" = "docker" ]; then
        docker inspect -f '{{.State.Pid}}' "${WHISPER_CONTAINER}" 2>/dev/null || echo ""
        return
    fi

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
    local service_type
    service_type=$(get_service_type "${service}")

    log_info "正在启动 ${service_name}..."

    # 检查依赖
    if ! check_dependencies "${service}"; then
        return 1
    fi

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

    # 根据服务类型启动
    case "${service}" in
        api)
            cd "${PROJECT_ROOT}"
            nohup bun run --cwd packages/ccode --conditions=browser src/index.ts serve --port "${port}" \
                > "${log_file}" 2>&1 &
            ;;

        web)
            cd "${PROJECT_ROOT}/packages/web"
            nohup bun run dev --port "${port}" > "${log_file}" 2>&1 &
            ;;

        zero-daemon)
            # 检查是否需要构建
            if ! is_rust_binary_built "${service}"; then
                log_warn "Rust 二进制未构建，正在构建..."
                build_rust_services || return 1
            fi

            cd "${RUST_SERVICES_DIR}"
            nohup "${RUST_TARGET_DIR}/zero-cli" daemon --port "${port}" --host 127.0.0.1 \
                > "${log_file}" 2>&1 &
            ;;

        zero-gateway)
            if ! is_rust_binary_built "${service}"; then
                log_warn "Rust 二进制未构建，正在构建..."
                build_rust_services || return 1
            fi

            cd "${RUST_SERVICES_DIR}"
            # 设置环境变量覆盖默认端口
            ZERO_GATEWAY_PORT="${port}" nohup "${RUST_TARGET_DIR}/zero-gateway" \
                > "${log_file}" 2>&1 &
            ;;

        zero-channels)
            if ! is_rust_binary_built "${service}"; then
                log_warn "Rust 二进制未构建，正在构建..."
                build_rust_services || return 1
            fi

            cd "${RUST_SERVICES_DIR}"
            nohup "${RUST_TARGET_DIR}/zero-channels" --port "${port}" \
                > "${log_file}" 2>&1 &
            ;;

        zero-workflow)
            if ! is_rust_binary_built "${service}"; then
                log_warn "Rust 二进制未构建，正在构建..."
                build_rust_services || return 1
            fi

            cd "${RUST_SERVICES_DIR}"
            nohup "${RUST_TARGET_DIR}/zero-workflow" --port "${port}" \
                > "${log_file}" 2>&1 &
            ;;

        whisper)
            # 检查 Docker 是否可用
            if ! command -v docker &> /dev/null; then
                log_error "Docker 未安装"
                echo "  请先安装 Docker Desktop"
                return 1
            fi
            if ! docker info &> /dev/null; then
                log_error "Docker 未运行"
                echo "  请启动 Docker Desktop"
                return 1
            fi

            local whisper_model="${WHISPER_MODEL:-base}"
            local cache_dir="${HOME}/.cache/huggingface"
            mkdir -p "${cache_dir}"

            log_info "拉取 Docker 镜像 ${WHISPER_IMAGE}..."
            docker pull "${WHISPER_IMAGE}" || true

            log_info "启动 Docker 容器..."
            docker run -d \
                --name "${WHISPER_CONTAINER}" \
                --rm \
                -p "${port}:8000" \
                -v "${cache_dir}:/root/.cache/huggingface" \
                -e WHISPER__MODEL="${whisper_model}" \
                "${WHISPER_IMAGE}" \
                > /dev/null 2>&1

            # Docker 容器不使用 PID 文件，直接返回
            sleep 3
            if is_running "whisper"; then
                log_success "${service_name} 启动成功 (Container: ${WHISPER_CONTAINER}, Port: ${port})"
                echo "  模型: ${whisper_model}"
                echo "  镜像: ${WHISPER_IMAGE}"
            else
                log_error "${service_name} 启动失败"
                echo "  查看日志: docker logs ${WHISPER_CONTAINER}"
                return 1
            fi
            return 0
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
    local service_type
    service_type=$(get_service_type "${service}")

    log_info "正在停止 ${service_name}..."

    # Docker 容器
    if [ "${service_type}" = "docker" ]; then
        if ! is_running "${service}"; then
            log_warn "${service_name} 未在运行"
            return 0
        fi
        docker stop "${WHISPER_CONTAINER}" > /dev/null 2>&1 || true
        log_success "${service_name} 已停止"
        return 0
    fi

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

# 启动核心服务
start_core() {
    log_info "启动核心服务..."
    echo ""
    for service in ${CORE_SERVICES}; do
        start_service "${service}"
    done
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
    # 反向停止
    local reversed=""
    for service in ${ALL_SERVICES}; do
        reversed="${service} ${reversed}"
    done
    for service in ${reversed}; do
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
    echo "╔════════════════════════════════════════════════════════════════════════╗"
    echo "║                        CodeCoder 服务状态                               ║"
    echo "╠════════════════════════════════════════════════════════════════════════╣"
    printf "║ %-25s │ %-10s │ %-8s │ %-6s │ %-6s ║\n" "服务" "状态" "PID" "端口" "类型"
    echo "╠════════════════════════════════════════════════════════════════════════╣"

    echo -e "║ ${CYAN}核心服务${NC}                                                              ║"

    for service in ${CORE_SERVICES}; do
        local service_name
        service_name=$(get_service_name "${service}")
        local port
        port=$(get_service_port "${service}")
        local service_type
        service_type=$(get_service_type "${service}")
        local status
        local pid="-"

        if is_running "${service}"; then
            status="${GREEN}运行中${NC}"
            if [ "${service_type}" = "docker" ]; then
                pid="docker"
            else
                pid=$(get_pid "${service}")
            fi
        else
            status="${RED}已停止${NC}"
        fi

        printf "║ %-25s │ %b%-2s │ %-8s │ %-6s │ %-6s ║\n" "${service_name}" "${status}" "" "${pid}" "${port}" "${service_type}"
    done

    echo "╠════════════════════════════════════════════════════════════════════════╣"
    echo -e "║ ${CYAN}独立 Rust 服务 (可选)${NC}                                                 ║"

    for service in ${STANDALONE_RUST_SERVICES}; do
        local service_name
        service_name=$(get_service_name "${service}")
        local port
        port=$(get_service_port "${service}")
        local service_type
        service_type=$(get_service_type "${service}")
        local status
        local pid="-"

        if is_running "${service}"; then
            status="${GREEN}运行中${NC}"
            pid=$(get_pid "${service}")
        else
            status="${YELLOW}未启动${NC}"
        fi

        printf "║ %-25s │ %b%-2s │ %-8s │ %-6s │ %-6s ║\n" "${service_name}" "${status}" "" "${pid}" "${port}" "${service_type}"
    done

    echo "╚════════════════════════════════════════════════════════════════════════╝"
    echo ""

    # 显示端口占用情况
    echo "端口占用检查:"
    for service in ${CORE_SERVICES}; do
        local port
        port=$(get_service_port "${service}")
        local service_name
        service_name=$(get_service_name "${service}")
        if check_port "${port}"; then
            echo -e "  ${port} (${service_name}): ${GREEN}已占用${NC}"
        else
            echo -e "  ${port} (${service_name}): ${YELLOW}空闲${NC}"
        fi
    done
    echo ""

    # 显示 Rust 构建状态
    echo "Rust 服务构建状态:"
    if [ -d "${RUST_TARGET_DIR}" ]; then
        for bin in zero-cli zero-gateway zero-channels zero-workflow; do
            if [ -f "${RUST_TARGET_DIR}/${bin}" ]; then
                local size
                size=$(du -h "${RUST_TARGET_DIR}/${bin}" | cut -f1)
                echo -e "  ${bin}: ${GREEN}已构建${NC} (${size})"
            else
                echo -e "  ${bin}: ${YELLOW}未构建${NC}"
            fi
        done
    else
        echo -e "  ${YELLOW}未构建 (运行 ./ops.sh build rust)${NC}"
    fi
    echo ""

    # 显示 Docker 容器信息（如果有）
    if command -v docker &> /dev/null && docker info &> /dev/null; then
        if docker ps --format '{{.Names}}' 2>/dev/null | grep -q "^${WHISPER_CONTAINER}$"; then
            echo "Docker 容器:"
            docker ps --filter "name=${WHISPER_CONTAINER}" --format "  {{.Names}}: {{.Image}} ({{.Status}})"
            echo ""
        fi
    fi
}

# 查看服务日志
show_logs() {
    local service="$1"
    local service_name
    service_name=$(get_service_name "${service}")
    local service_type
    service_type=$(get_service_type "${service}")

    # Docker 使用 Docker 日志
    if [ "${service_type}" = "docker" ]; then
        if ! docker ps -a --format '{{.Names}}' 2>/dev/null | grep -q "^${WHISPER_CONTAINER}$"; then
            log_error "Whisper 容器不存在"
            return 1
        fi
        log_info "显示 ${service_name} 日志 (最后 50 行):"
        echo "----------------------------------------"
        docker logs --tail 50 "${WHISPER_CONTAINER}" 2>&1
        return 0
    fi

    local log_file
    log_file=$(get_log_file "${service}")

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
    local service_name
    service_name=$(get_service_name "${service}")
    local service_type
    service_type=$(get_service_type "${service}")

    # Docker 使用 Docker 日志
    if [ "${service_type}" = "docker" ]; then
        if ! docker ps --format '{{.Names}}' 2>/dev/null | grep -q "^${WHISPER_CONTAINER}$"; then
            log_error "Whisper 容器未运行"
            return 1
        fi
        log_info "实时跟踪 ${service_name} 日志 (Ctrl+C 退出):"
        echo "----------------------------------------"
        docker logs -f "${WHISPER_CONTAINER}" 2>&1
        return 0
    fi

    local log_file
    log_file=$(get_log_file "${service}")

    if [ ! -f "${log_file}" ]; then
        log_error "日志文件不存在: ${log_file}"
        return 1
    fi

    log_info "实时跟踪 ${service_name} 日志 (Ctrl+C 退出):"
    echo "----------------------------------------"
    tail -f "${log_file}"
}

# 服务颜色映射
get_service_color() {
    case "$1" in
        api) echo "\033[0;32m" ;;           # 绿色
        web) echo "\033[0;34m" ;;           # 蓝色
        zero-daemon) echo "\033[0;35m" ;;   # 紫色
        whisper) echo "\033[0;36m" ;;       # 青色
        zero-gateway) echo "\033[0;33m" ;;  # 黄色
        zero-channels) echo "\033[0;31m" ;; # 红色
        zero-workflow) echo "\033[1;34m" ;; # 亮蓝色
        *) echo "\033[0m" ;;                # 默认
    esac
}

# 同时监控所有服务日志
tail_all_logs() {
    local target="${1:-running}"  # running | all | core
    local services_to_tail=""
    local pids=()

    log_info "收集服务日志..."

    # 根据目标确定要监控的服务列表
    local service_list
    case "${target}" in
        core) service_list="${CORE_SERVICES}" ;;
        all) service_list="${ALL_SERVICES}" ;;
        running|*) service_list="${ALL_SERVICES}" ;;
    esac

    # 检查哪些服务有日志可以监控
    for service in ${service_list}; do
        local service_type
        service_type=$(get_service_type "${service}")
        local log_file
        log_file=$(get_log_file "${service}")

        if [ "${target}" = "running" ]; then
            # 只监控运行中的服务
            if ! is_running "${service}"; then
                continue
            fi
        fi

        if [ "${service_type}" = "docker" ]; then
            if docker ps --format '{{.Names}}' 2>/dev/null | grep -q "^${WHISPER_CONTAINER}$"; then
                services_to_tail="${services_to_tail} ${service}"
            fi
        elif [ -f "${log_file}" ]; then
            services_to_tail="${services_to_tail} ${service}"
        fi
    done

    # 检查是否有服务可以监控
    if [ -z "${services_to_tail}" ]; then
        log_warn "没有找到可监控的服务日志"
        if [ "${target}" = "running" ]; then
            echo "  提示: 没有运行中的服务，或服务尚未产生日志"
            echo "  尝试: ./ops.sh tail all  (监控所有已有日志文件)"
        fi
        return 1
    fi

    # 显示将要监控的服务
    echo ""
    echo "╔════════════════════════════════════════════════════════════════════════╗"
    echo "║                      日志聚合监控 (Ctrl+C 退出)                         ║"
    echo "╠════════════════════════════════════════════════════════════════════════╣"
    echo "║ 正在监控以下服务:                                                       ║"
    for service in ${services_to_tail}; do
        local service_name
        service_name=$(get_service_name "${service}")
        local color
        color=$(get_service_color "${service}")
        printf "║   ${color}■${NC} %-20s                                               ║\n" "${service_name}"
    done
    echo "╚════════════════════════════════════════════════════════════════════════╝"
    echo ""

    # 清理函数 - 停止所有后台进程
    cleanup_tail_processes() {
        echo ""
        log_info "停止日志监控..."
        # 杀死整个进程组（包括所有子进程）
        kill -- -$$ 2>/dev/null || true
        for pid in "${pids[@]}"; do
            kill "${pid}" 2>/dev/null || true
        done
        wait 2>/dev/null || true
        log_success "日志监控已停止"
        exit 0
    }

    # 捕获 Ctrl+C 信号
    trap cleanup_tail_processes SIGINT SIGTERM

    # 启动每个服务的日志监控
    for service in ${services_to_tail}; do
        local service_type
        service_type=$(get_service_type "${service}")
        local color
        color=$(get_service_color "${service}")
        local prefix
        # 固定宽度的服务名前缀（15字符）
        prefix=$(printf "%-15s" "[${service}]")

        if [ "${service_type}" = "docker" ]; then
            # Docker 容器日志 - 使用进程替换避免管道信号问题
            while IFS= read -r line; do
                echo -e "${color}${prefix}${NC} ${line}"
            done < <(docker logs -f "${WHISPER_CONTAINER}" 2>&1) &
            pids+=($!)
        else
            # 文件日志 - 使用进程替换避免管道信号问题
            local log_file
            log_file=$(get_log_file "${service}")
            while IFS= read -r line; do
                echo -e "${color}${prefix}${NC} ${line}"
            done < <(tail -f "${log_file}" 2>/dev/null) &
            pids+=($!)
        fi
    done

    # 等待所有后台进程（直到用户按 Ctrl+C）
    log_info "日志监控已启动，按 Ctrl+C 退出"
    echo "════════════════════════════════════════════════════════════════════════════"
    wait
}

# 显示所有服务的最近日志（静态）
show_all_logs() {
    local lines="${1:-20}"

    echo ""
    echo "╔════════════════════════════════════════════════════════════════════════╗"
    echo "║                     所有服务日志 (最后 ${lines} 行)                        ║"
    echo "╚════════════════════════════════════════════════════════════════════════╝"

    for service in ${ALL_SERVICES}; do
        local service_name
        service_name=$(get_service_name "${service}")
        local service_type
        service_type=$(get_service_type "${service}")
        local color
        color=$(get_service_color "${service}")

        echo ""
        echo -e "${color}━━━ ${service_name} ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

        if [ "${service_type}" = "docker" ]; then
            if docker ps -a --format '{{.Names}}' 2>/dev/null | grep -q "^${WHISPER_CONTAINER}$"; then
                docker logs --tail "${lines}" "${WHISPER_CONTAINER}" 2>&1 | head -n "${lines}"
            else
                echo "  (容器不存在)"
            fi
        else
            local log_file
            log_file=$(get_log_file "${service}")
            if [ -f "${log_file}" ]; then
                tail -n "${lines}" "${log_file}"
            else
                echo "  (日志文件不存在)"
            fi
        fi
    done
    echo ""
}

# 构建命令
handle_build() {
    local target="${1:-all}"

    case "${target}" in
        rust)
            build_rust_services
            ;;
        all)
            log_info "构建所有服务..."
            build_rust_services
            ;;
        *)
            log_error "未知构建目标: ${target}"
            echo "  可用目标: rust, all"
            return 1
            ;;
    esac
}

# 健康检查
check_health() {
    local service="$1"
    local service_name
    service_name=$(get_service_name "${service}")
    local port
    port=$(get_service_port "${service}")
    local service_type
    service_type=$(get_service_type "${service}")

    if ! is_running "${service}"; then
        echo -e "  ${service_name}: ${RED}未运行${NC}"
        return 1
    fi

    # HTTP 健康检查
    local url=""
    case "${service}" in
        api) url="http://127.0.0.1:${port}/health" ;;
        web) url="http://127.0.0.1:${port}/" ;;
        zero-daemon|zero-gateway) url="http://127.0.0.1:${port}/health" ;;
        zero-channels|zero-workflow) url="http://127.0.0.1:${port}/health" ;;
        whisper) url="http://127.0.0.1:${port}/health" ;;
    esac

    if [ -n "${url}" ]; then
        local status_code
        status_code=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 2 "${url}" 2>/dev/null || echo "000")
        if [ "${status_code}" = "200" ] || [ "${status_code}" = "204" ]; then
            echo -e "  ${service_name}: ${GREEN}健康${NC} (HTTP ${status_code})"
        elif [ "${status_code}" = "000" ]; then
            echo -e "  ${service_name}: ${YELLOW}运行中但无响应${NC}"
        else
            echo -e "  ${service_name}: ${YELLOW}HTTP ${status_code}${NC}"
        fi
    else
        echo -e "  ${service_name}: ${GREEN}运行中${NC}"
    fi
}

# 健康检查所有服务
health_all() {
    log_info "服务健康检查:"
    echo ""
    for service in ${ALL_SERVICES}; do
        check_health "${service}" || true
    done
    echo ""
}

# 清理 PID 和日志文件
clean_files() {
    local target="${1:-all}"

    case "${target}" in
        pids)
            log_info "清理 PID 文件..."
            rm -rf "${PID_DIR}"/*.pid 2>/dev/null || true
            log_success "PID 文件已清理"
            ;;
        logs)
            log_info "清理日志文件..."
            rm -rf "${LOG_DIR}"/*.log 2>/dev/null || true
            log_success "日志文件已清理"
            ;;
        all)
            log_info "清理所有临时文件..."
            rm -rf "${PID_DIR}"/*.pid 2>/dev/null || true
            rm -rf "${LOG_DIR}"/*.log 2>/dev/null || true
            log_success "所有临时文件已清理"
            ;;
        *)
            log_error "未知清理目标: ${target}"
            echo "  可用目标: pids, logs, all"
            return 1
            ;;
    esac
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
    echo "  health             检查服务健康状态"
    echo "  logs <service>     查看服务日志 (最后 50 行)"
    echo "  logs all [n]       查看所有服务日志 (最后 n 行，默认 20)"
    echo "  tail <service>     实时跟踪服务日志"
    echo "  tail all           实时聚合监控所有服务日志"
    echo "  tail running       实时监控运行中服务日志 (默认)"
    echo "  tail core          实时监控核心服务日志"
    echo "  build [target]     构建服务 (rust|all)"
    echo "  clean [target]     清理临时文件 (pids|logs|all)"
    echo "  help               显示此帮助信息"
    echo ""
    echo "核心服务 (./ops.sh start 默认启动这些):"
    echo "  api                CodeCoder API Server (端口 4400, Bun)"
    echo "  web                Web Frontend (端口 4401, Vite)"
    echo "  zero-daemon        Zero CLI Daemon (端口 4402, Rust)"
    echo "  whisper            Whisper STT Server (端口 4403, Docker)"
    echo ""
    echo "独立 Rust 服务 (可选，用于模块化部署):"
    echo "  zero-gateway       独立网关服务 (端口 4410)"
    echo "  zero-channels      独立频道服务 (端口 4411)"
    echo "  zero-workflow      独立工作流服务 (端口 4412)"
    echo ""
    echo "服务组:"
    echo "  all                所有服务"
    echo "  core               仅核心服务 (默认)"
    echo "  running            仅运行中的服务 (用于 tail 命令)"
    echo ""
    echo "环境变量:"
    echo "  WHISPER_MODEL      Whisper 模型: tiny|base|small|medium|large (默认: base)"
    echo "  WHISPER_IMAGE      Docker 镜像 (默认: fedirz/faster-whisper-server:latest-cpu)"
    echo "  DEBUG=1            显示调试信息"
    echo ""
    echo "示例:"
    echo "  ./ops.sh start                  # 启动核心服务"
    echo "  ./ops.sh start all              # 启动所有服务"
    echo "  ./ops.sh start api              # 只启动 API 服务"
    echo "  ./ops.sh stop web               # 只停止 Web 服务"
    echo "  ./ops.sh restart zero-daemon    # 重启 Zero Daemon"
    echo "  ./ops.sh start whisper          # 启动 Whisper STT (Docker)"
    echo "  ./ops.sh start zero-gateway     # 启动独立网关"
    echo "  ./ops.sh build rust             # 构建 Rust 服务"
    echo "  ./ops.sh status                 # 查看状态"
    echo "  ./ops.sh health                 # 健康检查"
    echo "  ./ops.sh logs zero-daemon       # 查看 Zero Daemon 日志"
    echo "  ./ops.sh logs all               # 查看所有服务日志快照"
    echo "  ./ops.sh logs all 50            # 查看所有服务日志 (最后 50 行)"
    echo "  ./ops.sh tail api               # 实时跟踪 API 日志"
    echo "  ./ops.sh tail all               # 实时聚合监控所有服务"
    echo "  ./ops.sh tail running           # 实时监控运行中的服务"
    echo "  ./ops.sh tail core              # 实时监控核心服务"
    echo "  ./ops.sh clean all              # 清理临时文件"
    echo ""
    echo "架构说明:"
    echo "  zero-daemon 是组合服务，包含 gateway + channels + scheduler"
    echo "  独立 Rust 服务用于需要单独部署各组件的场景"
    echo "  所有服务共享 ~/.codecoder/config.json 配置"
    echo ""
}

# 主函数
main() {
    init_dirs

    local command="${1:-help}"
    local service="${2:-core}"

    case "${command}" in
        start)
            if [ "${service}" = "all" ]; then
                start_all
            elif [ "${service}" = "core" ]; then
                start_core
            elif is_valid_service "${service}"; then
                start_service "${service}"
            else
                log_error "未知服务: ${service}"
                show_help
                exit 1
            fi
            ;;
        stop)
            if [ "${service}" = "all" ] || [ "${service}" = "core" ]; then
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
            elif [ "${service}" = "core" ]; then
                stop_all
                echo ""
                start_core
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
        health)
            health_all
            ;;
        logs)
            if [ "${service}" = "all" ] || [ "${service}" = "core" ]; then
                show_all_logs "${3:-20}"
            elif is_valid_service "${service}"; then
                show_logs "${service}"
            else
                log_error "未知服务: ${service}"
                exit 1
            fi
            ;;
        tail)
            if [ "${service}" = "all" ]; then
                tail_all_logs "all"
            elif [ "${service}" = "core" ]; then
                tail_all_logs "core"
            elif [ "${service}" = "running" ] || [ -z "${2:-}" ]; then
                # 默认监控运行中的服务
                tail_all_logs "running"
            elif is_valid_service "${service}"; then
                tail_logs "${service}"
            else
                log_error "未知服务: ${service}"
                exit 1
            fi
            ;;
        build)
            handle_build "${service}"
            ;;
        clean)
            clean_files "${service}"
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
