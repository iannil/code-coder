#!/usr/bin/env bash
#
# CodeCoder 服务运维脚本
# 用于管理项目所有服务的启动、停止、状态查看
#
# 架构说明:
#   - redis:         Redis Server (Docker) - 可选，IM 渠道持久化 (REDIS_ENABLED=1 启用)
#   - api:           CodeCoder API Server (Bun/TypeScript)
#   - web:           Web Frontend (Vite/React)
#   - zero-daemon:   Zero CLI Daemon (Rust) - 进程编排器 + 服务中枢
#                      内置: gateway, channels, workflow, observer (来自 zero-hub)
#                      管理: zero-trading (4434) 子进程
#   - whisper:       Whisper STT Server (Docker)
#
# Rust Crates 结构 (4 crates):
#   - zero-cli:      主入口 (binary) - daemon + server 功能
#   - zero-trading:  交易服务 (binary) - PO3+SMT 自动化交易
#   - zero-hub:      服务库 (library) - gateway/channels/workflow/observer
#   - zero-core:     工具库 (library) - grep/glob/edit, browser + common (配置、日志、事件总线)
#
# 用法:
#   ./ops.sh start [service]   - 启动服务 (all|redis|api|web|zero-daemon|zero-trading|whisper)
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
CODECODER_DIR="${HOME}/.codecoder"
PID_DIR="${CODECODER_DIR}/.pids"
LOG_DIR="${CODECODER_DIR}/logs"
RUST_SERVICES_DIR="${PROJECT_ROOT}/services"
RUST_TARGET_DIR="${RUST_SERVICES_DIR}/target/release"

# Docker 容器名称
WHISPER_CONTAINER="codecoder-whisper"
WHISPER_IMAGE="${WHISPER_IMAGE:-fedirz/faster-whisper-server:latest-cpu}"
REDIS_CONTAINER="codecoder-redis"
REDIS_IMAGE="${REDIS_IMAGE:-redis:7-alpine}"
REDIS_PORT="${REDIS_PORT:-4410}"

# 服务列表 (按启动顺序)
# 可选基础设施服务 (设置 REDIS_ENABLED=1 启用)
OPTIONAL_SERVICES="redis"
# 核心服务
CORE_SERVICES="api web zero-daemon whisper"
# 默认服务 (不含可选服务)
DEFAULT_SERVICES="${CORE_SERVICES}"
# 所有服务 (可选 + 核心服务)
ALL_SERVICES="${OPTIONAL_SERVICES} ${CORE_SERVICES}"
# Rust 微服务 (由 daemon spawn，日志文件独立)
# 注意: gateway, channels, workflow, observer 已合并到 zero-hub library
# zero-cli daemon 内置这些功能，只需 spawn zero-trading
RUST_MICROSERVICES="zero-trading"

# 噪音过滤模式 (用于 tail 命令)
# 这些模式匹配连接池、HTTP/2 帧等底层库日志，通常不含业务上下文
NOISE_FILTER_PATTERN='hyper_util::client::legacy::pool|pooling idle connection|reuse idle connection|h2::codec|h2::proto|rustls::conn|tokio_util::codec'

# 服务配置函数
get_service_port() {
    case "$1" in
        api) echo "4400" ;;
        web) echo "4401" ;;
        zero-daemon) echo "4402" ;;
        whisper) echo "4403" ;;
        zero-trading) echo "4434" ;;
        redis) echo "${REDIS_PORT}" ;;
        *) echo "" ;;
    esac
}

get_service_name() {
    case "$1" in
        api) echo "CodeCoder API Server" ;;
        web) echo "Web Frontend (Vite)" ;;
        zero-daemon) echo "Zero CLI Daemon" ;;
        whisper) echo "Whisper STT Server" ;;
        redis) echo "Redis Server" ;;
        zero-trading) echo "Zero Trading" ;;
        *) echo "" ;;
    esac
}

get_service_type() {
    case "$1" in
        api|web) echo "node" ;;
        zero-daemon) echo "rust" ;;
        whisper|redis) echo "docker" ;;
        *) echo "" ;;
    esac
}

is_valid_service() {
    case "$1" in
        api|web|zero-daemon|whisper|redis|zero-trading) return 0 ;;
        *) return 1 ;;
    esac
}

is_core_service() {
    case "$1" in
        api|web|zero-daemon|whisper) return 0 ;;
        *) return 1 ;;
    esac
}

is_optional_service() {
    case "$1" in
        redis) return 0 ;;
        *) return 1 ;;
    esac
}

is_redis_enabled() {
    [ "${REDIS_ENABLED:-0}" = "1" ] || [ "${REDIS_ENABLED:-0}" = "true" ]
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

# 打印分割线
print_separator() {
    echo ""
    echo -e "${CYAN}════════════════════════════════════════════════════════════════════════════${NC}"
    echo ""
}

# 往日志文件写入分割线
log_separator() {
    local log_file="$1"
    local action="$2"
    local timestamp
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "" >> "${log_file}"
    echo "════════════════════════════════════════════════════════════════════════════" >> "${log_file}"
    echo "[${timestamp}] ═══ ${action} ═══" >> "${log_file}"
    echo "════════════════════════════════════════════════════════════════════════════" >> "${log_file}"
    echo "" >> "${log_file}"
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
        export RUST_BUILT=true

        # 同步 NAPI 类型定义到 packages/core
        sync_napi_types
    else
        log_error "Cargo 未安装，无法构建 Rust 服务"
        return 1
    fi
}

# 同步 NAPI 类型定义
sync_napi_types() {
    local source="${RUST_SERVICES_DIR}/zero-core/index.d.ts"
    local script="${PROJECT_ROOT}/scripts/sync-napi-types.ts"

    if [ -f "${source}" ]; then
        log_info "同步 NAPI 类型定义..."

        if command -v bun &> /dev/null && [ -f "${script}" ]; then
            # 使用智能合并脚本
            cd "${PROJECT_ROOT}"
            bun "${script}"
        else
            # 回退到简单复制 + 修复保留关键字
            local target="${PROJECT_ROOT}/packages/core/src/binding.d.ts"
            cp "${source}" "${target}"
            sed -i '' \
                -e 's/extends:/extendsFrom:/g' \
                -e 's/extends?:/extendsFrom?:/g' \
                -e 's/interface:/interfaceName:/g' \
                "${target}"
            log_success "NAPI 类型定义已同步 (简单模式)"
        fi
    else
        log_warn "NAPI 类型定义源文件不存在: ${source}"
    fi
}

# 检查服务是否运行
is_running() {
    local service="$1"
    local service_type
    service_type=$(get_service_type "${service}")

    # Docker 容器
    if [ "${service_type}" = "docker" ]; then
        local container_name
        case "${service}" in
            whisper) container_name="${WHISPER_CONTAINER}" ;;
            redis) container_name="${REDIS_CONTAINER}" ;;
            *) return 1 ;;
        esac
        if docker ps --format '{{.Names}}' 2>/dev/null | grep -q "^${container_name}$"; then
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
        local container_name
        case "${service}" in
            whisper) container_name="${WHISPER_CONTAINER}" ;;
            redis) container_name="${REDIS_CONTAINER}" ;;
            *) echo ""; return ;;
        esac
        docker inspect -f '{{.State.Pid}}' "${container_name}" 2>/dev/null || echo ""
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

    # 往日志文件写入启动分割线
    if [ "${service_type}" != "docker" ]; then
        log_separator "${log_file}" "SERVICE START: ${service_name}"
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
            # 编译 Rust 服务（如果尚未编译）
            if [ "${RUST_BUILT:-}" != "true" ]; then
                build_rust_services || return 1
            fi

            cd "${RUST_SERVICES_DIR}"
            # daemon 自动管理 gateway/channels/workflow 子进程
            nohup "${RUST_TARGET_DIR}/zero-cli" daemon --host 127.0.0.1 \
                > "${log_file}" 2>&1 &
            ;;

        redis)
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

            # 检查是否已有同名容器（可能是已停止的）
            if docker ps -a --format '{{.Names}}' 2>/dev/null | grep -q "^${REDIS_CONTAINER}$"; then
                log_info "移除已存在的 Redis 容器..."
                docker rm -f "${REDIS_CONTAINER}" > /dev/null 2>&1 || true
            fi

            local redis_data_dir="${HOME}/.codecoder/redis"
            mkdir -p "${redis_data_dir}"

            log_info "启动 Redis Docker 容器..."
            docker run -d \
                --name "${REDIS_CONTAINER}" \
                -p "${port}:6379" \
                -v "${redis_data_dir}:/data" \
                "${REDIS_IMAGE}" \
                redis-server --appendonly yes \
                > /dev/null 2>&1

            # Docker 容器不使用 PID 文件，直接返回
            sleep 2
            if is_running "redis"; then
                log_success "${service_name} 启动成功 (Container: ${REDIS_CONTAINER}, Port: ${port})"
                echo "  数据目录: ${redis_data_dir}"
                echo "  镜像: ${REDIS_IMAGE}"
            else
                log_error "${service_name} 启动失败"
                echo "  查看日志: docker logs ${REDIS_CONTAINER}"
                return 1
            fi
            return 0
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
    local log_file
    log_file=$(get_log_file "${service}")
    local service_type
    service_type=$(get_service_type "${service}")

    log_info "正在停止 ${service_name}..."

    # Docker 容器
    if [ "${service_type}" = "docker" ]; then
        if ! is_running "${service}"; then
            log_warn "${service_name} 未在运行"
            return 0
        fi
        local container_name
        case "${service}" in
            whisper) container_name="${WHISPER_CONTAINER}" ;;
            redis) container_name="${REDIS_CONTAINER}" ;;
            *) log_error "未知 Docker 服务: ${service}"; return 1 ;;
        esac
        docker stop "${container_name}" > /dev/null 2>&1 || true
        log_success "${service_name} 已停止"
        return 0
    fi

    if ! is_running "${service}"; then
        log_warn "${service_name} 未在运行"
        rm -f "${pid_file}"
        return 0
    fi

    # 往日志文件写入停止分割线
    if [ -f "${log_file}" ]; then
        log_separator "${log_file}" "SERVICE STOP: ${service_name}"
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
    print_separator
    log_info "启动核心服务..."
    echo ""

    # 检查 Redis 是否在运行（IM 渠道可选依赖）
    if ! is_running "redis"; then
        log_warn "Redis 未运行，IM 渠道将使用内存模式 (不持久化)"
        echo "  提示: 设置 REDIS_ENABLED=1 ./ops.sh start 启用 Redis"
        echo ""
    fi

    # 先统一编译 Rust 服务
    local has_rust_service=false
    for service in ${CORE_SERVICES}; do
        if [ "$(get_service_type "${service}")" = "rust" ]; then
            has_rust_service=true
            break
        fi
    done
    if [ "${has_rust_service}" = true ]; then
        build_rust_services || return 1
    fi

    for service in ${CORE_SERVICES}; do
        start_service "${service}"
    done

    print_separator
}

# 启动所有服务
start_all() {
    print_separator
    log_info "启动所有服务..."
    echo ""

    # 先统一编译 Rust 服务
    build_rust_services || return 1

    for service in ${ALL_SERVICES}; do
        start_service "${service}"
    done

    print_separator
}

# 停止所有服务
stop_all() {
    print_separator
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

    print_separator
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

    echo -e "║ ${CYAN}可选服务 (REDIS_ENABLED=1 启用)${NC}                                       ║"

    for service in ${OPTIONAL_SERVICES}; do
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

    echo "╠────────────────────────────────────────────────────────────────────────╣"
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
    echo -e "║ ${CYAN}由 daemon 管理的微服务${NC}                                                ║"
    echo "║   • zero-trading:  端口 4434 (PO3+SMT 自动化交易)                      ║"
    echo "║   (gateway/channels/workflow/observer 已内置于 daemon)               ║"
    echo "╚════════════════════════════════════════════════════════════════════════╝"
    echo ""

    # 显示端口占用情况
    echo "端口占用检查:"
    # 可选服务端口
    for service in ${OPTIONAL_SERVICES}; do
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
    # 核心服务端口
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
    # 检查 daemon 管理的微服务端口
    for port in 4434; do
        if check_port "${port}"; then
            echo -e "  ${port} (daemon 管理): ${GREEN}已占用${NC}"
        else
            echo -e "  ${port} (daemon 管理): ${YELLOW}空闲${NC}"
        fi
    done
    echo ""

    # 显示 Rust 构建状态
    echo "Rust 服务构建状态:"
    if [ -d "${RUST_TARGET_DIR}" ]; then
        for bin in zero-cli zero-trading; do
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
        local container_name
        case "${service}" in
            whisper) container_name="${WHISPER_CONTAINER}" ;;
            redis) container_name="${REDIS_CONTAINER}" ;;
            *) log_error "未知 Docker 服务: ${service}"; return 1 ;;
        esac
        if ! docker ps -a --format '{{.Names}}' 2>/dev/null | grep -q "^${container_name}$"; then
            log_error "${service_name} 容器不存在"
            return 1
        fi
        log_info "显示 ${service_name} 日志 (最后 50 行):"
        echo "----------------------------------------"
        docker logs --tail 50 "${container_name}" 2>&1
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
        local container_name
        case "${service}" in
            whisper) container_name="${WHISPER_CONTAINER}" ;;
            redis) container_name="${REDIS_CONTAINER}" ;;
            *) log_error "未知 Docker 服务: ${service}"; return 1 ;;
        esac
        if ! docker ps --format '{{.Names}}' 2>/dev/null | grep -q "^${container_name}$"; then
            log_error "${service_name} 容器未运行"
            return 1
        fi
        log_info "实时跟踪 ${service_name} 日志 (Ctrl+C 退出):"
        echo "----------------------------------------"
        docker logs -f "${container_name}" 2>&1
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
        redis) echo "\033[0;31m" ;;         # 红色
        zero-trading) echo "\033[0;96m" ;;  # 亮青色
        *) echo "\033[0m" ;;                # 默认
    esac
}

# 清理 ANSI 转义序列
strip_ansi() {
    sed 's/\x1b\[[0-9;]*[mGKHF]//g' | sed 's/\x1b\[?[0-9;]*[0-9;]*[0-9;]*m//g'
}

# 获取日志级别颜色
get_level_color() {
    case "$1" in
        ERROR|error|FATAL|fatal) echo "\033[0;31m" ;;
        WARN|warn|WARNING) echo "\033[0;33m" ;;
        INFO|info) echo "\033[0;32m" ;;
        DEBUG|debug) echo "\033[0;90m" ;;
        TRACE|trace) echo "\033[0;37m" ;;
        *) echo "\033[0;36m" ;;  # 默认青色
    esac
}

# 从日志行提取日志级别
extract_log_level() {
    local line="$1"

    # JSON 格式 - 检查 level 字段
    if echo "$line" | grep -qE '^\{'; then
        # 尝试提取 level 字段
        local level
        level=$(echo "$line" | grep -oE '"level":"?[A-Za-z]+' | head -1 | sed 's/"level"://i' | sed 's/"//g' | tr '[:lower:]' '[:upper:]')
        if [ -n "$level" ]; then
            echo "$level"
            return
        fi
        # 检查 severity 字段 (某些服务可能使用)
        level=$(echo "$line" | grep -oE '"severity":"?[A-Za-z]+' | head -1 | sed 's/"severity"://i' | sed 's/"//g' | tr '[:lower:]' '[:upper:]')
        if [ -n "$level" ]; then
            echo "$level"
            return
        fi
    fi

    # Pretty 格式 - 检测 INFO/WARN/ERROR/DEBUG
    for lvl in ERROR WARN WARNING INFO DEBUG TRACE FATAL; do
        if echo "$line" | grep -qE "\b${lvl}\b"; then
            echo "$lvl"
            return
        fi
    done

    # 默认
    echo "INFO"
}

# 从日志行提取时间戳
extract_timestamp() {
    local line="$1"
    local current_ts
    current_ts=$(date '+%m-%d %H:%M:%S')

    # JSON 格式 - 检查 timestamp 字段
    if echo "$line" | grep -qE '"timestamp'; then
        local ts
        ts=$(echo "$line" | grep -oE '"timestamp":"[^"]+"' | sed 's/"timestamp":"//;s/"//')
        if [ -n "$ts" ]; then
            # 转换 ISO 8601 到 mm-dd HH:MM:SS
            local formatted
            formatted=$(echo "$ts" | sed -E 's/[0-9]{4}-([0-9]{2})-([0-9]{2})T([0-9]{2}):([0-9]{2}):([0-9]{2})\.?[0-9]*Z?/\1-\2 \3:\4:\5/')
            if [ "$formatted" != "$ts" ]; then
                echo "$formatted"
                return
            fi
        fi
    fi

    # Pretty 格式 - 提取时间部分
    if echo "$line" | grep -qE '^[0-9]{4}-[0-9]{2}-[0-9]{2}[T ][0-9]{2}:[0-9]{2}:[0-9]{2}'; then
        local ts_extract
        ts_extract=$(echo "$line" | grep -oE '^[0-9]{4}-[0-9]{2}-[0-9]{2}[T ][0-9]{2}:[0-9]{2}:[0-9]{2}' | \
            sed -E 's/^[0-9]+-([0-9]{2})-([0-9]{2})[T ]([0-9]{2}):([0-9]{2}):([0-9]{2})/\1-\2 \3:\4:\5/')
        if [ -n "$ts_extract" ]; then
            echo "$ts_extract"
            return
        fi
    fi

    # 无时间戳 - 使用当前时间
    echo "$current_ts"
}

# 格式化 JSON 日志为 key=value 格式
format_json_fields() {
    local json="$1"
    local main_message=""
    local use_jq=false

    # 检查 jq 是否可用
    if command -v jq &> /dev/null; then
        use_jq=true
    fi

    # 提取主要消息字段
    if echo "$json" | grep -qE '"message"'; then
        if [ "$use_jq" = true ]; then
            main_message=$(echo "$json" | jq -r '.message // empty' 2>/dev/null)
        else
            main_message=$(echo "$json" | grep -oE '"message":"[^"]*"' | sed 's/"message":"//;s/"$//' | head -1)
        fi
    fi

    # 检查 event 字段 (API 服务使用)
    if echo "$json" | grep -qE '"event"'; then
        local event_msg
        if [ "$use_jq" = true ]; then
            event_msg=$(echo "$json" | jq -r '.event // empty' 2>/dev/null)
        else
            event_msg=$(echo "$json" | grep -oE '"event":"[^"]*"' | sed 's/"event":"//;s/"$//' | head -1)
        fi
        if [ -n "$event_msg" ]; then
            main_message="$event_msg"
        fi
    fi

    # zero-daemon 格式: {"level":"INFO","fields":{"message":"..."},...}
    if echo "$json" | grep -qE '"fields"'; then
        if [ "$use_jq" = true ]; then
            local fields_msg
            fields_msg=$(echo "$json" | jq -r '.fields.message // empty' 2>/dev/null)
            if [ -n "$fields_msg" ]; then
                main_message="$fields_msg"
            fi
        fi
    fi

    # 构建字段字符串
    local fields=""
    local excluded_fields="timestamp level severity target message event fields"

    if [ "$use_jq" = true ]; then
        # 使用 jq 提取所有非排除字段
        fields=$(echo "$json" | jq -r "
            to_entries | .[] |
            select(.key as \$k | \$k != \"timestamp\" and \$k != \"level\" and
                   \$k != \"severity\" and \$k != \"target\" and
                   \$k != \"message\" and \$k != \"event\" and
                   \$k != \"fields\") |
            \"\(.key)=\(.value)\"
        " 2>/dev/null | tr '\n' ' ' | sed 's/ $//')
    else
        # 使用 grep/awk 提取字段 (简化版)
        # 先移除大括号，然后处理每个键值对
        fields=$(echo "$json" | sed -E 's/^\{|\}$//g' | tr ',' '\n' | \
            grep -vE '^"(timestamp|level|severity|target|message|event|fields)"' | \
            while IFS=':' read -r key value; do
                key=$(echo "$key" | sed 's/"//g' | xargs)
                value=$(echo "$value" | sed 's/"//g' | xargs)
                if [ -n "$key" ] && [ -n "$value" ]; then
                    echo "${key}=${value}"
                fi
            done | tr '\n' ' ' | sed 's/ $//')
    fi

    # 组合输出
    if [ -n "$fields" ]; then
        if [ -n "$main_message" ]; then
            echo "${main_message} ${fields}"
        else
            echo "$fields"
        fi
    else
        echo "$main_message"
    fi
}

# 格式化单行日志输出
# 格式: MM-DD HH:MM:SS | service | LEVEL | message
format_log_line() {
    local raw_line="$1"
    local service="$2"
    local service_color="$3"

    # 清理 ANSI
    local clean_line
    clean_line=$(echo "$raw_line" | strip_ansi)

    # 提取信息
    local timestamp
    local level
    local message

    timestamp=$(extract_timestamp "$clean_line")
    level=$(extract_log_level "$clean_line")

    # 检查是否为 JSON
    if echo "$clean_line" | grep -qE '^\{'; then
        message=$(format_json_fields "$clean_line")
        # 如果解析失败，使用原始行
        if [ -z "$message" ]; then
            message="$clean_line"
        fi
    else
        # 移除时间戳前缀 (如果存在)
        # 支持格式: ISO 8601 (2026-02-27T10:30:45.123Z) 和带空格格式
        # 注意: macOS sed 不支持 \s，使用字面空格
        message=$(echo "$clean_line" | sed -E 's/^[0-9]{4}-[0-9]{2}-[0-9]{2}[T ][0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?(Z)? +//')
        # 移除日志级别前缀
        message=$(echo "$message" | sed -E 's/^ *(INFO|WARN|ERROR|DEBUG|TRACE|FATAL|WARNING) *//')
    fi

    # 获取级别颜色
    local level_color
    level_color=$(get_level_color "$level")

    # 格式化输出: 时间戳 | 服务名(右对齐14字符) | 级别 | 消息
    # 服务名对齐: api/web 短名靠右，zero-* 长名自然显示
    printf "%s | ${service_color}%14s${NC} | ${level_color}%-5s${NC} | %s\n" "$timestamp" "$service" "$level" "$message"
}

# 同时监控所有服务日志
# 用法: tail_all_logs <target> <raw>
#   target: running | all | core
#   raw: true 显示全部日志（含噪音），false 过滤噪音
tail_all_logs() {
    local target="${1:-running}"  # running | all | core
    local raw="${2:-false}"       # true | false
    local services_to_tail=""
    local pids=()

    log_info "收集服务日志..."

    # 显示过滤状态
    if [ "${raw}" = "true" ]; then
        log_info "模式: 显示全部日志 (--raw)"
    else
        log_info "模式: 过滤底层库噪音日志 (使用 --raw 显示全部)"
    fi

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

    # 添加 Rust 微服务日志 (由 daemon spawn，日志文件独立)
    for rust_service in ${RUST_MICROSERVICES}; do
        local rust_log="${LOG_DIR}/${rust_service}.log"
        if [ -f "${rust_log}" ]; then
            services_to_tail="${services_to_tail} ${rust_service}"
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
    if [ "${raw}" != "true" ]; then
        echo "╠────────────────────────────────────────────────────────────────────────╣"
        echo "║ 💡 噪音过滤已启用 (hyper/h2/rustls 等底层日志已隐藏)                   ║"
    fi
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

        if [ "${service_type}" = "docker" ]; then
            # Docker 容器日志 - 使用进程替换避免管道信号问题
            if [ "${raw}" = "true" ]; then
                while IFS= read -r line; do
                    format_log_line "$line" "${service}" "${color}"
                done < <(docker logs -f "${WHISPER_CONTAINER}" 2>&1) &
            else
                while IFS= read -r line; do
                    format_log_line "$line" "${service}" "${color}"
                done < <(docker logs -f "${WHISPER_CONTAINER}" 2>&1 | grep -vE "${NOISE_FILTER_PATTERN}") &
            fi
            pids+=($!)
        else
            # 文件日志 - 使用进程替换避免管道信号问题
            local log_file
            log_file=$(get_log_file "${service}")
            if [ "${raw}" = "true" ]; then
                while IFS= read -r line; do
                    format_log_line "$line" "${service}" "${color}"
                done < <(tail -f "${log_file}" 2>/dev/null) &
            else
                while IFS= read -r line; do
                    format_log_line "$line" "${service}" "${color}"
                done < <(tail -f "${log_file}" 2>/dev/null | grep -vE "${NOISE_FILTER_PATTERN}") &
            fi
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

    # 显示 Rust 微服务日志 (由 daemon spawn)
    for rust_service in ${RUST_MICROSERVICES}; do
        local log_file="${LOG_DIR}/${rust_service}.log"
        if [ -f "${log_file}" ]; then
            local service_name
            service_name=$(get_service_name "${rust_service}")
            local color
            color=$(get_service_color "${rust_service}")
            echo ""
            echo -e "${color}━━━ ${service_name} ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
            tail -n "${lines}" "${log_file}"
        fi
    done
    echo ""
}

# 按 trace_id 搜索并聚合所有日志
show_trace_logs() {
    local trace_id="$1"
    if [ -z "${trace_id}" ]; then
        log_error "请提供 trace_id"
        echo "  用法: ./ops.sh logs trace <trace_id>"
        return 1
    fi

    log_info "搜索 trace_id: ${trace_id}"
    echo ""

    local found=false

    # 搜索所有日志文件
    for log_file in "${LOG_DIR}"/*.log; do
        if [ -f "${log_file}" ]; then
            local matches
            matches=$(grep "${trace_id}" "${log_file}" 2>/dev/null || true)
            if [ -n "${matches}" ]; then
                local service_name
                service_name=$(basename "${log_file}" .log)
                local color
                color=$(get_service_color "${service_name}")
                echo -e "${color}━━━ ${service_name} ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
                echo "${matches}"
                echo ""
                found=true
            fi
        fi
    done

    if [ "${found}" = false ]; then
        log_warn "未找到匹配的日志条目"
    fi
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

    # Redis 健康检查 (使用 docker exec)
    if [ "${service}" = "redis" ]; then
        local redis_ping
        redis_ping=$(docker exec "${REDIS_CONTAINER}" redis-cli ping 2>/dev/null || echo "")
        if [ "${redis_ping}" = "PONG" ]; then
            echo -e "  ${service_name}: ${GREEN}健康${NC} (PING PONG)"
        else
            echo -e "  ${service_name}: ${YELLOW}运行中但无响应${NC}"
        fi
        return 0
    fi

    # HTTP 健康检查
    local url=""
    case "${service}" in
        api) url="http://127.0.0.1:${port}/health" ;;
        web) url="http://127.0.0.1:${port}/" ;;
        zero-daemon) url="http://127.0.0.1:${port}/health" ;;
        zero-trading) url="http://127.0.0.1:${port}/health" ;;
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

# ============================================================================
# 可观测性命令 (Observability)
# ============================================================================

# 服务端口映射 (用于 metrics)
METRICS_ENDPOINTS=(
    "ccode-api:4400"
    "zero-daemon:4402"
    "zero-trading:4434"
)

# 获取单个服务的指标
fetch_service_metrics() {
    local service="$1"
    local port="$2"
    local url="http://127.0.0.1:${port}/api/v1/metrics"

    # 尝试获取 JSON 格式指标
    local metrics_json
    metrics_json=$(curl -s --connect-timeout 2 "${url}" 2>/dev/null || echo "")

    if [ -n "${metrics_json}" ] && echo "${metrics_json}" | grep -q '"service"'; then
        echo "${metrics_json}"
    else
        # 返回空 JSON 表示服务不可用
        echo '{"service":"'"${service}"'","total_requests":0,"error_requests":0,"error_rate":0,"p50_ms":0,"p95_ms":0,"p99_ms":0,"active_connections":0,"memory_bytes":0,"uptime_secs":0,"status":"offline"}'
    fi
}

# 格式化内存大小
format_memory() {
    local bytes="$1"
    if [ "${bytes}" -ge 1073741824 ]; then
        echo "$(echo "scale=1; ${bytes}/1073741824" | bc)GB"
    elif [ "${bytes}" -ge 1048576 ]; then
        echo "$(echo "scale=0; ${bytes}/1048576" | bc)MB"
    elif [ "${bytes}" -ge 1024 ]; then
        echo "$(echo "scale=0; ${bytes}/1024" | bc)KB"
    else
        echo "${bytes}B"
    fi
}

# 格式化持续时间
format_duration() {
    local ms="$1"
    if [ "$(echo "${ms} >= 1000" | bc)" -eq 1 ]; then
        echo "$(echo "scale=1; ${ms}/1000" | bc)s"
    else
        echo "${ms}ms"
    fi
}

# 显示服务指标
show_metrics() {
    local target="${1:-all}"
    local watch_mode="${2:-false}"

    # 单次显示或 watch 模式
    show_metrics_once() {
        echo ""
        echo "╔════════════════════════════════════════════════════════════════════════════╗"
        echo "║                           CodeCoder 服务指标                                ║"
        echo "╠════════════════════════════════════════════════════════════════════════════╣"
        printf "║ %-16s │ %8s │ %6s │ %7s │ %7s │ %7s │ %7s ║\n" "服务" "请求数" "错误率" "p50" "p95" "p99" "内存"
        echo "╠════════════════════════════════════════════════════════════════════════════╣"

        local has_metrics=false

        for endpoint in "${METRICS_ENDPOINTS[@]}"; do
            local service="${endpoint%%:*}"
            local port="${endpoint##*:}"

            # 如果指定了单个服务，只显示该服务
            if [ "${target}" != "all" ] && [ "${target}" != "${service}" ]; then
                continue
            fi

            local metrics_json
            metrics_json=$(fetch_service_metrics "${service}" "${port}")

            # 解析 JSON
            local total_requests error_rate p50 p95 p99 memory_bytes status
            if command -v jq &> /dev/null; then
                total_requests=$(echo "${metrics_json}" | jq -r '.total_requests // 0')
                error_rate=$(echo "${metrics_json}" | jq -r '.error_rate // 0')
                p50=$(echo "${metrics_json}" | jq -r '.p50_ms // 0')
                p95=$(echo "${metrics_json}" | jq -r '.p95_ms // 0')
                p99=$(echo "${metrics_json}" | jq -r '.p99_ms // 0')
                memory_bytes=$(echo "${metrics_json}" | jq -r '.memory_bytes // 0')
                status=$(echo "${metrics_json}" | jq -r '.status // "online"')
            else
                # 简单的 grep 解析
                total_requests=$(echo "${metrics_json}" | grep -o '"total_requests":[0-9]*' | grep -o '[0-9]*' || echo "0")
                error_rate=$(echo "${metrics_json}" | grep -o '"error_rate":[0-9.]*' | grep -o '[0-9.]*' || echo "0")
                p50=$(echo "${metrics_json}" | grep -o '"p50_ms":[0-9.]*' | grep -o '[0-9.]*' || echo "0")
                p95=$(echo "${metrics_json}" | grep -o '"p95_ms":[0-9.]*' | grep -o '[0-9.]*' || echo "0")
                p99=$(echo "${metrics_json}" | grep -o '"p99_ms":[0-9.]*' | grep -o '[0-9.]*' || echo "0")
                memory_bytes=$(echo "${metrics_json}" | grep -o '"memory_bytes":[0-9]*' | grep -o '[0-9]*' || echo "0")
                status=$(echo "${metrics_json}" | grep -o '"status":"[^"]*"' | sed 's/"status":"//;s/"//' || echo "online")
            fi

            # 格式化输出
            local memory_str
            memory_str=$(format_memory "${memory_bytes}")
            local error_rate_str
            error_rate_str=$(printf "%.1f%%" "${error_rate}")
            local p50_str p95_str p99_str
            p50_str=$(format_duration "${p50}")
            p95_str=$(format_duration "${p95}")
            p99_str=$(format_duration "${p99}")

            # 状态颜色
            local status_color="${GREEN}"
            if [ "${status}" = "offline" ]; then
                status_color="${RED}"
                error_rate_str="-"
                p50_str="-"
                p95_str="-"
                p99_str="-"
                memory_str="-"
            fi

            printf "║ ${status_color}%-16s${NC} │ %8s │ %6s │ %7s │ %7s │ %7s │ %7s ║\n" \
                "${service}" "${total_requests}" "${error_rate_str}" "${p50_str}" "${p95_str}" "${p99_str}" "${memory_str}"

            has_metrics=true
        done

        echo "╚════════════════════════════════════════════════════════════════════════════╝"
        echo ""

        if [ "${has_metrics}" = false ]; then
            log_warn "未找到任何运行中的服务"
        fi

        # 显示时间戳
        echo -e "${CYAN}最后更新: $(date '+%Y-%m-%d %H:%M:%S')${NC}"
    }

    if [ "${watch_mode}" = "true" ]; then
        # Watch 模式：每 2 秒刷新
        log_info "实时指标监控 (Ctrl+C 退出)"
        while true; do
            clear
            show_metrics_once
            sleep 2
        done
    else
        show_metrics_once
    fi
}

# 可视化显示调用链路
show_trace() {
    local trace_id="$1"

    if [ -z "${trace_id}" ]; then
        log_error "请提供 trace_id"
        echo "  用法: ./ops.sh trace <trace_id>"
        return 1
    fi

    log_info "搜索 trace_id: ${trace_id}"
    echo ""

    # 收集所有匹配的日志条目
    local all_entries=""
    local services_found=""

    for log_file in "${LOG_DIR}"/*.log; do
        if [ -f "${log_file}" ]; then
            local matches
            matches=$(grep "${trace_id}" "${log_file}" 2>/dev/null || true)
            if [ -n "${matches}" ]; then
                local service_name
                service_name=$(basename "${log_file}" .log)
                services_found="${services_found} ${service_name}"
                all_entries="${all_entries}${matches}"$'\n'
            fi
        fi
    done

    if [ -z "${services_found}" ]; then
        log_warn "未找到 trace_id: ${trace_id} 的日志条目"
        return 1
    fi

    # 显示 trace 概览
    echo "╔════════════════════════════════════════════════════════════════════════════╗"
    echo "║                              Trace 详情                                     ║"
    echo "╠════════════════════════════════════════════════════════════════════════════╣"
    echo "║ Trace ID: ${trace_id}"
    echo "║ 涉及服务:${services_found}"
    echo "╠════════════════════════════════════════════════════════════════════════════╣"
    echo "║                              Timeline                                       ║"
    echo "╠════════════════════════════════════════════════════════════════════════════╣"

    # 解析并排序日志条目 (按时间戳)
    # 格式: timestamp | service | event | duration
    echo "${all_entries}" | while IFS= read -r line; do
        if [ -z "${line}" ]; then
            continue
        fi

        # 提取时间戳和服务
        local timestamp
        timestamp=$(extract_timestamp "${line}")
        local service=""
        local event=""
        local duration=""

        # 尝试从日志路径推断服务
        for log_file in "${LOG_DIR}"/*.log; do
            if grep -q "${line}" "${log_file}" 2>/dev/null; then
                service=$(basename "${log_file}" .log)
                break
            fi
        done

        # 尝试从 JSON 提取事件类型
        if echo "${line}" | grep -qE '"event_type"'; then
            if command -v jq &> /dev/null; then
                event=$(echo "${line}" | jq -r '.event_type // .event // "unknown"' 2>/dev/null || echo "unknown")
                duration=$(echo "${line}" | jq -r '.duration_ms // ""' 2>/dev/null || echo "")
            else
                event=$(echo "${line}" | grep -oE '"event_type":"[^"]*"' | sed 's/"event_type":"//;s/"$//' || echo "unknown")
            fi
        else
            # 从消息中提取关键词
            if echo "${line}" | grep -qiE 'request|start'; then
                event="request_start"
            elif echo "${line}" | grep -qiE 'response|end|finish'; then
                event="request_end"
            elif echo "${line}" | grep -qiE 'error|fail'; then
                event="error"
            else
                event="log"
            fi
        fi

        # 获取服务颜色
        local color
        color=$(get_service_color "${service}")

        # 格式化输出
        if [ -n "${duration}" ]; then
            printf "║ %-12s │ ${color}%-14s${NC} │ %-20s │ %8sms ║\n" \
                "${timestamp}" "${service}" "${event}" "${duration}"
        else
            printf "║ %-12s │ ${color}%-14s${NC} │ %-20s │          ║\n" \
                "${timestamp}" "${service}" "${event}"
        fi
    done

    echo "╚════════════════════════════════════════════════════════════════════════════╝"
    echo ""
}

# 显示慢请求
show_slow_requests() {
    local threshold="${1:-1000}"  # 默认 1000ms
    local live_mode="${2:-false}"

    show_slow_once() {
        echo ""
        echo "╔════════════════════════════════════════════════════════════════════════════╗"
        echo "║                      慢请求 (> ${threshold}ms)                                    ║"
        echo "╠════════════════════════════════════════════════════════════════════════════╣"
        printf "║ %-12s │ %-14s │ %-6s │ %-20s │ %8s ║\n" "时间" "服务" "状态" "路径" "延迟"
        echo "╠════════════════════════════════════════════════════════════════════════════╣"

        local found=false

        # 搜索所有日志文件
        for log_file in "${LOG_DIR}"/*.log; do
            if [ ! -f "${log_file}" ]; then
                continue
            fi

            local service_name
            service_name=$(basename "${log_file}" .log)
            local color
            color=$(get_service_color "${service_name}")

            # 搜索包含 duration 字段且超过阈值的日志
            grep -E '"duration_ms":[0-9]+' "${log_file}" 2>/dev/null | while IFS= read -r line; do
                local duration
                if command -v jq &> /dev/null; then
                    duration=$(echo "${line}" | jq -r '.duration_ms // 0' 2>/dev/null || echo "0")
                else
                    duration=$(echo "${line}" | grep -oE '"duration_ms":[0-9]+' | grep -oE '[0-9]+' || echo "0")
                fi

                # 检查是否超过阈值
                if [ "${duration}" -ge "${threshold}" ]; then
                    local timestamp path status
                    timestamp=$(extract_timestamp "${line}")

                    if command -v jq &> /dev/null; then
                        path=$(echo "${line}" | jq -r '.path // .url // "-"' 2>/dev/null | head -c 20)
                        status=$(echo "${line}" | jq -r '.status // .status_code // "-"' 2>/dev/null)
                    else
                        path=$(echo "${line}" | grep -oE '"path":"[^"]*"' | sed 's/"path":"//;s/"$//' | head -c 20 || echo "-")
                        status=$(echo "${line}" | grep -oE '"status":[0-9]+' | grep -oE '[0-9]+' || echo "-")
                    fi

                    local duration_str
                    duration_str=$(format_duration "${duration}")

                    printf "║ %-12s │ ${color}%-14s${NC} │ %-6s │ %-20s │ %8s ║\n" \
                        "${timestamp}" "${service_name}" "${status}" "${path}" "${duration_str}"

                    found=true
                fi
            done
        done

        echo "╚════════════════════════════════════════════════════════════════════════════╝"

        if [ "${found}" = false ]; then
            log_info "没有找到超过 ${threshold}ms 的请求"
        fi

        echo ""
        echo -e "${CYAN}阈值: ${threshold}ms | 最后更新: $(date '+%Y-%m-%d %H:%M:%S')${NC}"
    }

    if [ "${live_mode}" = "true" ]; then
        # 实时模式
        log_info "实时监控慢请求 (Ctrl+C 退出)"
        while true; do
            clear
            show_slow_once
            sleep 5
        done
    else
        show_slow_once
    fi
}

# ============================================================================
# TUI Dashboard
# ============================================================================

# 实时仪表盘
show_dashboard() {
    local refresh_interval="${1:-2}"

    # 清理函数
    cleanup_dashboard() {
        tput cnorm  # 显示光标
        tput sgr0   # 重置颜色
        echo ""
        log_info "仪表盘已关闭"
        exit 0
    }

    trap cleanup_dashboard SIGINT SIGTERM

    # 隐藏光标
    tput civis

    while true; do
        # 清屏并移动光标到顶部
        tput clear
        tput cup 0 0

        local term_width
        term_width=$(tput cols)
        local term_height
        term_height=$(tput lines)

        # 标题
        echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${CYAN}║${NC}                       ${GREEN}CodeCoder 实时仪表盘${NC}                                  ${CYAN}║${NC}"
        echo -e "${CYAN}║${NC}                       $(date '+%Y-%m-%d %H:%M:%S')                                   ${CYAN}║${NC}"
        echo -e "${CYAN}╠══════════════════════════════════════════════════════════════════════════════╣${NC}"

        # 服务状态面板
        echo -e "${CYAN}║${NC} ${YELLOW}服务状态${NC}                                                                      ${CYAN}║${NC}"
        echo -e "${CYAN}╠──────────────────────────────────────────────────────────────────────────────╣${NC}"

        # 检查每个服务
        local services_line=""
        for service in api zero-daemon zero-trading whisper redis; do
            local status_icon
            local port

            case "${service}" in
                api) port=4400 ;;
                zero-daemon) port=4402 ;;
                zero-trading) port=4434 ;;
                whisper) port=4403 ;;
                redis) port="${REDIS_PORT}" ;;
            esac

            if check_port "${port}"; then
                status_icon="${GREEN}●${NC}"
            else
                status_icon="${RED}○${NC}"
            fi

            services_line="${services_line}  ${status_icon} ${service}"
        done
        # Use echo -e to interpret ANSI escape sequences in color codes
        # Fixed padding since ANSI codes affect printf width calculation
        echo -e "${CYAN}║${NC}${services_line}                ${CYAN}║${NC}"

        # 指标面板
        echo -e "${CYAN}╠──────────────────────────────────────────────────────────────────────────────╣${NC}"
        echo -e "${CYAN}║${NC} ${YELLOW}实时指标${NC}                                                                      ${CYAN}║${NC}"
        echo -e "${CYAN}╠──────────────────────────────────────────────────────────────────────────────╣${NC}"

        printf "${CYAN}║${NC} %-14s │ %8s │ %6s │ %7s │ %7s │ %7s │ %7s ${CYAN}║${NC}\n" \
            "服务" "请求数" "错误率" "p50" "p95" "p99" "内存"
        echo -e "${CYAN}║${NC}────────────────┼──────────┼────────┼─────────┼─────────┼─────────┼─────────${CYAN}║${NC}"

        for endpoint in "${METRICS_ENDPOINTS[@]}"; do
            local service="${endpoint%%:*}"
            local port="${endpoint##*:}"
            local metrics_json
            metrics_json=$(fetch_service_metrics "${service}" "${port}")

            if command -v jq &> /dev/null; then
                local total_requests error_rate p50 p95 p99 memory_bytes
                total_requests=$(echo "${metrics_json}" | jq -r '.total_requests // 0')
                error_rate=$(echo "${metrics_json}" | jq -r '.error_rate // 0')
                p50=$(echo "${metrics_json}" | jq -r '.p50_ms // 0')
                p95=$(echo "${metrics_json}" | jq -r '.p95_ms // 0')
                p99=$(echo "${metrics_json}" | jq -r '.p99_ms // 0')
                memory_bytes=$(echo "${metrics_json}" | jq -r '.memory_bytes // 0')

                local memory_str
                memory_str=$(format_memory "${memory_bytes}")
                local error_rate_str
                error_rate_str=$(printf "%.1f%%" "${error_rate}")

                local color="${GREEN}"
                if [ "$(echo "${error_rate} > 5" | bc 2>/dev/null || echo 0)" -eq 1 ]; then
                    color="${RED}"
                elif [ "$(echo "${error_rate} > 1" | bc 2>/dev/null || echo 0)" -eq 1 ]; then
                    color="${YELLOW}"
                fi

                printf "${CYAN}║${NC} ${color}%-14s${NC} │ %8s │ %6s │ %7.0f │ %7.0f │ %7.0f │ %7s ${CYAN}║${NC}\n" \
                    "${service}" "${total_requests}" "${error_rate_str}" "${p50}" "${p95}" "${p99}" "${memory_str}"
            else
                printf "${CYAN}║${NC} %-14s │ %8s │ %6s │ %7s │ %7s │ %7s │ %7s ${CYAN}║${NC}\n" \
                    "${service}" "-" "-" "-" "-" "-" "-"
            fi
        done

        # 最近错误面板
        echo -e "${CYAN}╠──────────────────────────────────────────────────────────────────────────────╣${NC}"
        echo -e "${CYAN}║${NC} ${YELLOW}最近错误 (最后 5 条)${NC}                                                         ${CYAN}║${NC}"
        echo -e "${CYAN}╠──────────────────────────────────────────────────────────────────────────────╣${NC}"

        # 搜索所有日志中的错误
        local error_count=0
        for log_file in "${LOG_DIR}"/*.log; do
            if [ -f "${log_file}" ] && [ "${error_count}" -lt 5 ]; then
                local service_name
                service_name=$(basename "${log_file}" .log)
                local color
                color=$(get_service_color "${service_name}")

                grep -iE '"level":"ERROR"|ERROR|error' "${log_file}" 2>/dev/null | tail -n 3 | while IFS= read -r line; do
                    local timestamp
                    timestamp=$(extract_timestamp "${line}")
                    local message
                    if command -v jq &> /dev/null; then
                        message=$(echo "${line}" | jq -r '.message // .error // "Unknown error"' 2>/dev/null | head -c 50)
                    else
                        message=$(echo "${line}" | grep -oE '"message":"[^"]*"' | sed 's/"message":"//;s/"$//' | head -c 50 || echo "Error")
                    fi

                    printf "${CYAN}║${NC} ${RED}%-8s${NC} [${color}%-12s${NC}] %-45s ${CYAN}║${NC}\n" \
                        "${timestamp}" "${service_name}" "${message}"
                done
                error_count=$((error_count + 1))
            fi
        done

        if [ "${error_count}" -eq 0 ]; then
            printf "${CYAN}║${NC} ${GREEN}%-76s${NC} ${CYAN}║${NC}\n" "No recent errors"
        fi

        # 底部
        echo -e "${CYAN}╠──────────────────────────────────────────────────────────────────────────────╣${NC}"
        echo -e "${CYAN}║${NC} 按 ${YELLOW}Ctrl+C${NC} 退出 | 刷新间隔: ${refresh_interval}s                                          ${CYAN}║${NC}"
        echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════════════════╝${NC}"

        sleep "${refresh_interval}"
    done
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
    echo "  metrics [service]  查看服务指标 (p50/p95/p99 延迟、错误率等)"
    echo "  trace <trace_id>   可视化显示完整调用链路"
    echo "  slow [threshold]   显示慢请求 (默认 > 1000ms)"
    echo "  logs <service>     查看服务日志 (最后 50 行)"
    echo "  logs all [n]       查看所有服务日志 (最后 n 行，默认 20)"
    echo "  logs trace <id>    按 trace_id 搜索并聚合所有服务日志"
    echo "  tail <service>     实时跟踪服务日志"
    echo "  tail all [--raw]   实时聚合监控所有服务日志 (含 Rust 微服务)"
    echo "  tail running       实时监控运行中服务日志 (默认)"
    echo "  tail core          实时监控核心服务日志"
    echo "  build [target]     构建服务 (rust|all)"
    echo "  clean [target]     清理临时文件 (pids|logs|all)"
    echo "  help               显示此帮助信息"
    echo ""
    echo "可观测性命令:"
    echo "  metrics            显示所有服务的实时指标"
    echo "  metrics <service>  显示单个服务的指标"
    echo "  metrics --watch    持续刷新指标 (每 2 秒)"
    echo "  trace <trace_id>   可视化显示完整调用链 (跨服务)"
    echo "  slow               显示慢请求 (默认 > 1000ms)"
    echo "  slow <ms>          显示延迟超过指定毫秒的请求"
    echo "  slow --live        实时监控慢请求"
    echo "  dashboard [secs]   启动实时仪表盘 (默认每 2 秒刷新)"
    echo ""
    echo "tail 命令选项:"
    echo "  --raw              显示全部日志 (不过滤 hyper/h2/rustls 等底层库噪音)"
    echo "                     默认行为: 过滤连接池、HTTP/2 帧等底层日志"
    echo ""
    echo "可选服务 (设置 REDIS_ENABLED=1 启用):"
    echo "  redis              Redis Server (端口 ${REDIS_PORT}, Docker) - IM 渠道持久化"
    echo ""
    echo "核心服务 (./ops.sh start 默认启动这些):"
    echo "  api                CodeCoder API Server (端口 4400, Bun)"
    echo "  web                Web Frontend (端口 4401, Vite)"
    echo "  zero-daemon        Zero CLI Daemon (端口 4402, Rust) - 进程编排器"
    echo "  whisper            Whisper STT Server (端口 4403, Docker)"
    echo ""
    echo "由 daemon 管理的微服务 (自动启动，无需手动管理):"
    echo "  zero-trading       交易服务 (端口 4434) - PO3+SMT 自动化交易"
    echo "  (gateway/channels/workflow/observer 已内置于 daemon)"
    echo ""
    echo "服务组:"
    echo "  all                所有服务 (可选 + 核心服务)"
    echo "  core               仅核心服务 (默认)"
    echo "  running            仅运行中的服务 (用于 tail 命令)"
    echo ""
    echo "环境变量:"
    echo "  REDIS_ENABLED      启用 Redis (设置为 1 或 true 启用, 默认禁用)"
    echo "  REDIS_PORT         Redis 端口 (默认: 4410)"
    echo "  REDIS_IMAGE        Redis Docker 镜像 (默认: redis:7-alpine)"
    echo "  WHISPER_MODEL      Whisper 模型: tiny|base|small|medium|large (默认: base)"
    echo "  WHISPER_IMAGE      Whisper Docker 镜像 (默认: fedirz/faster-whisper-server:latest-cpu)"
    echo "  DEBUG=1            显示调试信息"
    echo ""
    echo "示例:"
    echo "  ./ops.sh start                  # 启动核心服务 (默认不含 Redis)"
    echo "  ./ops.sh start all              # 启动所有服务 (含 Redis)"
    echo "  REDIS_ENABLED=1 ./ops.sh start  # 启动核心服务 + Redis"
    echo "  ./ops.sh start redis            # 只启动 Redis"
    echo "  ./ops.sh start api              # 只启动 API 服务"
    echo "  ./ops.sh stop web               # 只停止 Web 服务"
    echo "  ./ops.sh restart zero-daemon    # 重启 Daemon (会重启所有微服务)"
    echo "  ./ops.sh start whisper          # 启动 Whisper STT (Docker)"
    echo "  ./ops.sh build rust             # 构建 Rust 服务"
    echo "  ./ops.sh status                 # 查看状态"
    echo "  ./ops.sh health                 # 健康检查 (含 Redis PING)"
    echo "  ./ops.sh metrics                # 查看所有服务指标"
    echo "  ./ops.sh metrics --watch        # 实时监控指标"
    echo "  ./ops.sh trace abc123           # 查看 trace_id 为 abc123 的调用链"
    echo "  ./ops.sh slow 500               # 显示 > 500ms 的请求"
    echo "  ./ops.sh logs redis             # 查看 Redis 日志"
    echo "  ./ops.sh logs zero-daemon       # 查看 Daemon 日志"
    echo "  ./ops.sh logs zero-trading      # 查看交易服务日志"
    echo "  ./ops.sh logs all               # 查看所有服务日志快照"
    echo "  ./ops.sh logs trace <trace_id>  # 按 trace_id 搜索日志"
    echo "  ./ops.sh tail api               # 实时跟踪 API 日志"
    echo "  ./ops.sh tail all               # 实时聚合监控所有服务 (已过滤噪音)"
    echo "  ./ops.sh tail all --raw         # 实时监控 (显示全部日志含噪音)"
    echo "  ./ops.sh clean all              # 清理临时文件"
    echo ""
    echo "架构说明:"
    echo "  Redis 用于存储 IM 渠道的会话映射 (conversation_id → session_id)"
    echo "  zero-daemon 是进程编排器 + 服务中枢:"
    echo "    内置: gateway, channels, workflow, observer (来自 zero-hub library)"
    echo "    管理: zero-trading (4434) - PO3+SMT 自动化交易"
    echo "  Management API: http://127.0.0.1:4402 (/health, /status, /restart/:name)"
    echo "  所有服务共享 ~/.codecoder/config.json 配置"
    echo "  所有服务日志和 PID 文件统一存储在: ~/.codecoder/"
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
                print_separator
                start_service "${service}"
                print_separator
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
                print_separator
                stop_service "${service}"
                print_separator
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
                print_separator
                restart_service "${service}"
                print_separator
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
        metrics)
            # 检查参数
            local watch_mode="false"
            local target="all"
            for arg in "${@:2}"; do
                if [ "${arg}" = "--watch" ] || [ "${arg}" = "-w" ]; then
                    watch_mode="true"
                elif [ "${arg}" != "" ]; then
                    target="${arg}"
                fi
            done
            show_metrics "${target}" "${watch_mode}"
            ;;
        trace)
            show_trace "${2:-}"
            ;;
        slow)
            # 检查参数
            local threshold="1000"
            local live_mode="false"
            for arg in "${@:2}"; do
                if [ "${arg}" = "--live" ] || [ "${arg}" = "-l" ]; then
                    live_mode="true"
                elif [[ "${arg}" =~ ^[0-9]+$ ]]; then
                    threshold="${arg}"
                fi
            done
            show_slow_requests "${threshold}" "${live_mode}"
            ;;
        dashboard)
            # 启动实时仪表盘
            local refresh="${2:-2}"
            show_dashboard "${refresh}"
            ;;
        logs)
            if [ "${service}" = "all" ] || [ "${service}" = "core" ]; then
                show_all_logs "${3:-20}"
            elif [ "${service}" = "trace" ]; then
                show_trace_logs "${3:-}"
            elif is_valid_service "${service}"; then
                show_logs "${service}"
            # 支持直接查看 Rust 微服务日志
            elif [ -f "${LOG_DIR}/${service}.log" ]; then
                log_info "显示 ${service} 日志 (最后 50 行):"
                echo "----------------------------------------"
                tail -n 50 "${LOG_DIR}/${service}.log"
            else
                log_error "未知服务: ${service}"
                exit 1
            fi
            ;;
        tail)
            # 检查 --raw 选项
            local raw_mode="false"
            local target="${service}"
            for arg in "${@:2}"; do
                if [ "${arg}" = "--raw" ]; then
                    raw_mode="true"
                elif [ "${arg}" != "${service}" ]; then
                    # 如果不是 service 参数且不是 --raw，可能是 target
                    if [ "${arg}" = "all" ] || [ "${arg}" = "core" ] || [ "${arg}" = "running" ]; then
                        target="${arg}"
                    fi
                fi
            done

            if [ "${target}" = "all" ]; then
                tail_all_logs "all" "${raw_mode}"
            elif [ "${target}" = "core" ]; then
                tail_all_logs "core" "${raw_mode}"
            elif [ "${target}" = "running" ] || [ -z "${2:-}" ]; then
                # 默认监控运行中的服务
                tail_all_logs "running" "${raw_mode}"
            elif is_valid_service "${target}"; then
                tail_logs "${target}"
            else
                log_error "未知服务: ${target}"
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
